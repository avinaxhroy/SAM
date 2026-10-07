//! `today.view`: presentation projection behind the Today screen (UI P2 · U1, UI_PLAN D1).
//!
//! Classifies records into factual categories without arbitrary algorithmic ranking:
//! planned today, overdue reviews, due today, next sequential item, inactive/stale,
//! and upcoming dated items.
//!
//! Invariants:
//! - Intra-group ordering follows plan key ordering.
//! - Across groups, ordering is deterministic and explicit.
//! - Records belong to exactly one group (first matching category).
//! - Generates identical output for UI rendering and `SAM today.view --json` (§4.4).

use serde_json::{Value, json};

use crate::evaluator::{EvalContext, Evaluation, civil_date, parse_date};
use crate::expression::MAX_EVALUATION_STEPS;
use crate::model::Record;
use crate::pipeline;
use crate::resolved_config::ResolvedConfig;
use crate::scheduler::{Review, add_days, date_of, days_between};
use crate::transaction::days_from_civil;
use crate::views::week_count;

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// How wide the day is: `day` is today exactly (plus a three-day look-ahead for
/// dated things); `week` widens the due set to the covering week's end and the
/// look-ahead to seven days. A plan without dated weeks gets today + 7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    Day,
    Week,
}

/// The read's parameters, as the registry declares them.
#[derive(Debug, Clone)]
pub struct TodayRequest {
    /// Compute the day as if this civil date were today (tests, and a caller
    /// asking for another day). `None` = the plan's own clock.
    pub date: Option<String>,
    pub window: Window,
    /// Items per group; every group still reports its full `count`.
    pub limit: usize,
}

impl Default for TodayRequest {
    fn default() -> Self {
        Self {
            date: None,
            window: Window::Day,
            limit: 12,
        }
    }
}

/// One record's facts, gathered once, before any group claims it.
struct Facts<'a> {
    record: &'a Record,
    /// The record's position in the plan's own file order — the second key of
    /// "next in plan order (week.index, then record order)".
    position: usize,
    trackable: bool,
    complete: bool,
    focused: bool,
    review: Option<Review>,
    week_index: Option<i64>,
    /// The record's own date field, when its type declares one (an assessment's
    /// date, a session's date). Never a review date.
    dated: Option<String>,
    /// The record's own `est` duration, in minutes, when its type declares one.
    est: Option<i64>,
    /// The record's own `min` duration, in minutes (a logged session).
    logged: Option<i64>,
    course: Option<Value>,
}

/// One row of the day, already claimed by its group.
struct Row {
    id: String,
    kind: String,
    label: String,
    course: Option<Value>,
    due: Option<String>,
    late_days: i64,
    est: Option<i64>,
    reviewed_days_ago: Option<i64>,
    interval_days: Option<i64>,
    week_index: Option<i64>,
    days_until: Option<i64>,
    reason: String,
}

impl Row {
    fn json(&self) -> Value {
        json!({
            "id": self.id,
            "kind": self.kind,
            "label": self.label,
            "course": self.course,
            "dueDate": self.due,
            "lateDays": self.late_days,
            "est": self.est,
            "reviewedDaysAgo": self.reviewed_days_ago,
            "intervalDays": self.interval_days,
            "weekIndex": self.week_index,
            "daysUntil": self.days_until,
            "reason": self.reason,
        })
    }
}

/// The whole day, as facts.
pub fn today_view_json(
    config: &ResolvedConfig,
    context: &EvalContext,
    request: &TodayRequest,
) -> Value {
    let today = request
        .date
        .clone()
        .unwrap_or_else(|| civil_date(context.now_unix, context.tz_offset_minutes));
    let (year, month, day) = parse_date(&today).unwrap_or((1970, 1, 1));
    let today_days = days_from_civil(year, month, day);

    // The plan's own calendar: which week covers the day, and how many weeks the
    // term declares. `views.today_json` reads the same two facts, so the frame
    // and the screen cannot disagree about the week number.
    let evaluation = Evaluation::new(*context, MAX_EVALUATION_STEPS);
    let covering = evaluation.covering_week(today_days);
    let week = json!({
        "index": covering.as_ref().map(|(_, index)| *index),
        "of": week_count(config, &evaluation),
        "id": covering.as_ref().map(|(id, _)| id.clone()),
    });

    // The horizon for dated things. `week` reaches the covering week's own end
    // (the plan's calendar, not a guess); `day` looks three days ahead.
    let week_end = covering
        .as_ref()
        .and_then(|(id, _)| config.record(id))
        .and_then(|week| week.fields.get("end"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let horizon = match request.window {
        Window::Week => {
            week_end.unwrap_or_else(|| add_days(&today, 7).unwrap_or_else(|| today.clone()))
        }
        Window::Day => add_days(&today, 3).unwrap_or_else(|| today.clone()),
    };

    // ── gather ─────────────────────────────────────────────────────────────
    let washes = identity_washes(config);
    let mut facts: Vec<Facts> = Vec::with_capacity(config.record_count());
    for (position, record) in config.records_iter().enumerate() {
        let Some(def) = config.types.get(&record.type_) else {
            continue;
        };
        let trackable = def.trackable == Some(true);
        let focused = def.field("focus").is_some()
            && record.fields.get("focus").and_then(Value::as_str) == Some(today.as_str());
        let (complete, review) = if trackable {
            match pipeline::pipeline_for(config, &record.type_) {
                Ok((_, pipeline_def)) => {
                    let entry = context
                        .state
                        .and_then(|state| state.progress.get(&record.id));
                    (
                        pipeline::complete(pipeline_def, entry),
                        entry
                            .and_then(|entry| entry.review.as_ref())
                            .and_then(Review::from_json),
                    )
                }
                Err(_) => (false, None),
            }
        } else {
            (false, None)
        };
        let week_index = record
            .links
            .get("week")
            .and_then(|ids| ids.first())
            .and_then(|id| config.record(id))
            .and_then(|week| week.fields.get("index"))
            .and_then(Value::as_i64);
        let dated = def
            .field("date")
            .and_then(|_| record.fields.get("date"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let est = def
            .field("est")
            .and_then(|_| record.fields.get("est"))
            .and_then(Value::as_i64);
        let logged = def
            .field("min")
            .and_then(|_| record.fields.get("min"))
            .and_then(Value::as_i64);
        facts.push(Facts {
            record,
            position,
            trackable,
            complete,
            focused,
            review,
            week_index,
            dated,
            est,
            logged,
            course: course_of(config, record, &washes),
        });
    }

    // ── classify ───────────────────────────────────────────────────────────
    // Each item is claimed by the first group whose fact it carries; the order
    // below IS the cross-group order the screen states.
    let mut claimed: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    let reviewed_ago = |fact: &Facts| -> Option<i64> {
        fact.review
            .as_ref()
            .and_then(|review| review.last.as_deref())
            .and_then(|last| date_of(last, context.tz_offset_minutes))
            .and_then(|date| days_between(&date, &today))
    };

    let mut committed: Vec<Row> = Vec::new();
    for fact in &facts {
        if !fact.focused {
            continue;
        }
        claimed.insert(fact.record.id.clone());
        let late_days = fact
            .review
            .as_ref()
            .map(|review| review.overdue_days(&today))
            .unwrap_or(0);
        let mut reason = "planned for today".to_string();
        if late_days > 0 {
            reason.push_str(&format!(" · was due {} ago", days_phrase(late_days)));
        }
        committed.push(row(fact, late_days, None, reason, reviewed_ago(fact)));
    }
    committed.sort_by(|a, b| b.late_days.cmp(&a.late_days).then_with(|| a.id.cmp(&b.id)));

    let mut late: Vec<Row> = Vec::new();
    let mut due: Vec<Row> = Vec::new();
    for fact in &facts {
        if !fact.trackable || claimed.contains(&fact.record.id) {
            continue;
        }
        let Some(review) = &fact.review else { continue };
        let Some(due_date) = review.due.as_deref() else {
            continue;
        };
        let in_window = due_date <= today.as_str()
            || (request.window == Window::Week && due_date <= horizon.as_str());
        if !in_window {
            continue;
        }
        claimed.insert(fact.record.id.clone());
        let late_days = review.overdue_days(&today);
        let ago = reviewed_ago(fact);
        let mut reason = match (late_days, due_date) {
            (0, _) => "due today".to_string(),
            (n, _) => format!("{} late", days_phrase(n)),
        };
        if let Some(ago) = ago {
            reason.push_str(&format!(" · {}", reviewed_phrase(ago)));
        }
        let row = row(fact, late_days, Some(due_date), reason, ago);
        if late_days > 0 {
            late.push(row);
        } else {
            due.push(row);
        }
    }
    late.sort_by(|a, b| b.late_days.cmp(&a.late_days).then_with(|| a.id.cmp(&b.id)));
    due.sort_by(|a, b| a.due.cmp(&b.due).then_with(|| a.id.cmp(&b.id)));

    // "The plan says comes next": unstarted or unscheduled work, in the plan's
    // own order — week index first, then the order the records sit in the file.
    let mut next_order: Vec<&Facts> = facts
        .iter()
        .filter(|fact| {
            fact.trackable
                && !fact.complete
                && !claimed.contains(&fact.record.id)
                && fact
                    .review
                    .as_ref()
                    .is_none_or(|review| review.due.is_none())
        })
        .collect();
    next_order.sort_by(|a, b| {
        a.week_index
            .unwrap_or(i64::MAX)
            .cmp(&b.week_index.unwrap_or(i64::MAX))
            .then_with(|| a.position.cmp(&b.position))
    });
    let next_count = next_order.len();
    let mut next: Vec<Row> = Vec::new();
    for fact in next_order {
        claimed.insert(fact.record.id.clone());
        let reason = match fact.week_index {
            Some(index) => format!("next in the plan · week {index}"),
            None => "next in the plan".to_string(),
        };
        next.push(row(fact, 0, None, reason, reviewed_ago(fact)));
    }

    // Not touched in a while: older than twice the interval the item itself was
    // last put on. A missing interval is a missing fact, never a guess.
    let mut stale: Vec<Row> = Vec::new();
    for fact in &facts {
        if !fact.trackable || claimed.contains(&fact.record.id) {
            continue;
        }
        let Some(review) = &fact.review else { continue };
        let (Some(interval), Some(ago)) = (review.interval_days, reviewed_ago(fact)) else {
            continue;
        };
        if interval > 0 && ago > interval * 2 {
            claimed.insert(fact.record.id.clone());
            stale.push(row(
                fact,
                0,
                review.due.as_deref(),
                format!(
                    "{} · usually every {} day{}",
                    reviewed_phrase(ago),
                    interval,
                    plural(interval)
                ),
                Some(ago),
            ));
        }
    }
    stale.sort_by(|a, b| {
        b.reviewed_days_ago
            .cmp(&a.reviewed_days_ago)
            .then_with(|| a.id.cmp(&b.id))
    });

    // Dated things coming up: anything whose own kind declares a date and whose
    // date falls inside the horizon. Untracked by the scheduler — an obligation,
    // not a review.
    let mut upcoming: Vec<Row> = Vec::new();
    for fact in &facts {
        if fact.trackable {
            continue;
        }
        let Some(date) = fact.dated.as_deref() else {
            continue;
        };
        if date < today.as_str() || date > horizon.as_str() {
            continue;
        }
        let days_until = days_between(&today, date).unwrap_or(0);
        upcoming.push(Row {
            id: fact.record.id.clone(),
            kind: fact.record.type_.clone(),
            label: crate::scheduler::record_label(fact.record),
            course: fact.course.clone(),
            due: Some(date.to_string()),
            late_days: 0,
            est: fact.est,
            reviewed_days_ago: None,
            interval_days: None,
            week_index: fact.week_index,
            days_until: Some(days_until),
            reason: due_phrase(&today, date),
        });
    }
    upcoming.sort_by(|a, b| a.due.cmp(&b.due).then_with(|| a.id.cmp(&b.id)));

    // ── totals ─────────────────────────────────────────────────────────────
    // Logged: every record whose own kind carries both a date and a duration.
    // The last seven days are reported individually — a day strip and a stem
    // chart read the same facts the total is summed from.
    let mut logged_by_date: std::collections::BTreeMap<String, i64> =
        std::collections::BTreeMap::new();
    for fact in &facts {
        if let (Some(date), Some(min)) = (fact.dated.as_deref(), fact.logged) {
            *logged_by_date.entry(date.to_string()).or_insert(0) += min;
        }
    }
    let days: Vec<Value> = (0..7)
        .rev()
        .map(|back| {
            let date = add_days(&today, -back).unwrap_or_else(|| today.clone());
            let (year, month, day) = parse_date(&date).unwrap_or((1970, 1, 1));
            let weekday = weekday_of(days_from_civil(year, month, day));
            json!({
                "date": date,
                "weekday": weekday,
                "day": day,
                "loggedMin": logged_by_date.get(&date).copied().unwrap_or(0),
                "isToday": date == today,
            })
        })
        .collect();
    let logged_min = logged_by_date.get(&today).copied().unwrap_or(0);
    // The day's own estimate: summed over the whole queue, not the slice a
    // screen draws, so a card that says "about 9 h of work" means the queue and
    // not the twelve rows it happened to show.
    let planned_min: i64 = committed
        .iter()
        .chain(late.iter())
        .chain(due.iter())
        .chain(next.iter())
        .filter_map(|row| row.est)
        .sum();
    let target_min = config
        .rules
        .study
        .as_ref()
        .and_then(|study| study.get("dailyTargetMin"))
        .and_then(Value::as_i64);

    // ── the gap (F13) ──────────────────────────────────────────────────────
    // A fact about the plan, not a judgement: the last day the plan saw any
    // work, and how much of it is now waiting. Offered only when it is old
    // enough to matter AND something is actually waiting.
    let last_touch = facts
        .iter()
        .filter_map(|fact| fact.dated.clone())
        .chain(facts.iter().filter_map(|fact| {
            fact.review
                .as_ref()
                .and_then(|review| review.last.as_deref())
                .and_then(|last| date_of(last, context.tz_offset_minutes))
        }))
        .filter(|date| date.as_str() <= today.as_str())
        .max();
    let waiting: Vec<&Facts> = facts
        .iter()
        .filter(|fact| {
            fact.trackable
                && !fact.complete
                && fact.review.as_ref().is_none_or(|review| {
                    review
                        .due
                        .as_deref()
                        .is_none_or(|due| due <= today.as_str())
                })
        })
        .collect();
    let gap = match (&last_touch, waiting.len()) {
        (Some(since), waiting_count) if waiting_count > 0 => {
            let days_since = days_between(since, &today).unwrap_or(0);
            if days_since >= 3 {
                Some(json!({
                    "daysSince": days_since,
                    "since": since,
                    "waiting": waiting_count,
                    "estMin": waiting.iter().filter_map(|fact| fact.est).sum::<i64>(),
                }))
            } else {
                None
            }
        }
        _ => None,
    };

    json!({
        "date": today,
        "weekday": weekday_of(today_days),
        "day": day,
        "month": MONTHS[month.clamp(1, 12) as usize - 1],
        "week": week,
        "groups": [
            { "id": "committed", "count": committed.len(), "items": take(&committed, request.limit) },
            { "id": "late", "count": late.len(), "items": take(&late, request.limit) },
            { "id": "due", "count": due.len(), "items": take(&due, request.limit) },
            { "id": "next", "count": next_count, "items": take(&next, request.limit) },
            { "id": "stale", "count": stale.len(), "items": take(&stale, request.limit) },
            { "id": "upcoming", "count": upcoming.len(), "items": take(&upcoming, request.limit) },
        ],
        "totals": {
            "plannedMin": planned_min,
            "loggedMin": logged_min,
            "targetMin": target_min,
        },
        // How much plan there is at all. Without it, "nothing needs you" and
        // "there is nothing here" are the same payload, and the only screen
        // that must tell them apart — the first run, which arrived with a plan
        // it had just created empty — would have to guess (§4.4: the engine
        // states, the screen reads).
        "records": config.record_count(),
        // The last seven days, oldest first, ending on `date` — what the day
        // strip and the stem chart draw.
        "days": days,
        // "Nothing needs you" is a fact about committed, late and due only:
        // `next` and `stale` are always populated on a working plan, and a
        // screen that called that "clear" would be lying.
        "allClear": committed.is_empty() && late.is_empty() && due.is_empty(),
        "gap": gap,
    })
}

fn take(rows: &[Row], limit: usize) -> Vec<Value> {
    rows.iter().take(limit).map(Row::json).collect()
}

fn row(
    fact: &Facts,
    late_days: i64,
    due: Option<&str>,
    reason: String,
    reviewed_days_ago: Option<i64>,
) -> Row {
    Row {
        id: fact.record.id.clone(),
        kind: fact.record.type_.clone(),
        label: crate::scheduler::record_label(fact.record),
        course: fact.course.clone(),
        due: due.map(str::to_string),
        late_days,
        est: fact.est,
        reviewed_days_ago,
        interval_days: fact.review.as_ref().and_then(|review| review.interval_days),
        week_index: fact.week_index,
        days_until: None,
        reason,
    }
}

/// The record's course, by the rule the review queue already uses (a kind's
/// parent edge first, then `course`) — one rule, one implementation. The wash
/// comes from the identity map, so the same course is the same room everywhere.
fn course_of(
    config: &ResolvedConfig,
    record: &Record,
    washes: &std::collections::BTreeMap<String, String>,
) -> Option<Value> {
    let context = crate::scheduler::context_of(config, record)?;
    let id = context.get("id")?.as_str()?.to_string();
    Some(json!({
        "id": id,
        "label": context.get("label")?,
        "wash": washes.get(&id),
    }))
}

/// The identity washes: the design system's four rooms, assigned over the type
/// a plan marks `colorRole: "identity"`, in the plan's own order (the `order`
/// field when the kind declares one, else the file order). One assignment, so
/// every screen paints one course the same way.
pub(crate) fn identity_washes(
    config: &ResolvedConfig,
) -> std::collections::BTreeMap<String, String> {
    const WASHES: [&str; 4] = ["mint", "lilac", "butter", "sky"];
    let mut out = std::collections::BTreeMap::new();
    for (name, def) in &config.types {
        if def.color_role.as_deref() != Some("identity") {
            continue;
        }
        let mut records: Vec<&Record> = config
            .records_iter()
            .filter(|record| &record.type_ == name)
            .collect();
        records.sort_by_key(|record| {
            record
                .fields
                .get("order")
                .and_then(Value::as_i64)
                .unwrap_or(i64::MAX)
        });
        for (index, record) in records.iter().enumerate() {
            out.insert(record.id.clone(), WASHES[index % WASHES.len()].to_string());
        }
    }
    out
}

fn weekday_of(days: i64) -> &'static str {
    // 1970-01-01 was a Thursday, so day zero sits at index 4 of a Sunday-first
    // week — the same constant `views.today_json` uses.
    const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    WEEKDAYS[(days + 4).rem_euclid(7) as usize]
}

fn plural(n: i64) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// `3 days late`, `1 day late` — never `3 day late`.
fn days_phrase(n: i64) -> String {
    format!("{n} day{}", plural(n))
}

/// `last reviewed 9 days ago`, `reviewed yesterday`, `reviewed today`.
fn reviewed_phrase(days_ago: i64) -> String {
    match days_ago {
        0 => "reviewed today".to_string(),
        1 => "reviewed yesterday".to_string(),
        n => format!("last reviewed {n} days ago"),
    }
}

/// `today`/`tomorrow`/`in 3 days` for a date at or after `today`.
fn due_phrase(today: &str, date: &str) -> String {
    match days_between(today, date) {
        Some(0) => "today".to_string(),
        Some(1) => "tomorrow".to_string(),
        Some(n) => format!("in {n} days"),
        None => date.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn resources() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Sources/SAMCore/Resources")
    }

    fn load(fixture: &str) -> ResolvedConfig {
        crate::config_store::load_with(&resources().join(fixture), "explicit", &resources())
            .expect("the fixture loads")
    }

    /// 2026-09-21 in the plan's own timezone (Asia/Kolkata) — the day the
    /// fixture's reviews are read against. Frozen, so `lateDays` is arithmetic
    /// rather than a race with the clock.
    fn frozen(config: &ResolvedConfig) -> EvalContext<'_> {
        EvalContext::frozen(config, None, 1_790_000_000)
    }

    fn with_state<'a>(
        config: &'a ResolvedConfig,
        state: &'a crate::views::PlanState,
    ) -> EvalContext<'a> {
        EvalContext::frozen(config, Some(state), 1_790_000_000)
    }

    fn group<'a>(value: &'a Value, id: &str) -> &'a Value {
        value["groups"]
            .as_array()
            .expect("groups")
            .iter()
            .find(|group| group["id"] == id)
            .expect("a declared group")
    }

    fn ids(value: &Value, id: &str) -> Vec<String> {
        group(value, id)["items"]
            .as_array()
            .expect("items")
            .iter()
            .map(|item| item["id"].as_str().unwrap_or("?").to_string())
            .collect()
    }

    #[test]
    fn the_read_states_the_six_facts_in_a_fixed_order() {
        let config = load("presets/jee");
        let context = frozen(&config);
        let value = today_view_json(&config, &context, &TodayRequest::default());
        let order: Vec<String> = value["groups"]
            .as_array()
            .unwrap()
            .iter()
            .map(|group| group["id"].as_str().unwrap_or("?").to_string())
            .collect();
        assert_eq!(
            order,
            ["committed", "late", "due", "next", "stale", "upcoming"]
        );
        // Every row carries the numbers its reason was composed from, and a
        // label a student would say out loud — never the raw id.
        for group in value["groups"].as_array().unwrap() {
            for item in group["items"].as_array().unwrap() {
                assert!(
                    item["label"]
                        .as_str()
                        .is_some_and(|label| !label.is_empty())
                );
                assert!(
                    item["label"].as_str() != item["id"].as_str(),
                    "a row is titled by a name, not by its id"
                );
                assert!(item["reason"].as_str().is_some_and(|r| !r.is_empty()));
                assert!(!item["reason"].as_str().unwrap().contains('{'));
            }
        }
        assert!(value["date"].is_string());
        assert!(value["totals"]["loggedMin"].is_number());
    }

    #[test]
    fn plan_order_is_week_index_then_record_order() {
        let config = load("presets/jee");
        let context = frozen(&config);
        let value = today_view_json(&config, &context, &TodayRequest::default());
        let next = ids(&value, "next");
        assert!(
            !next.is_empty(),
            "a preset with unstarted work has a `next`"
        );
        assert_eq!(
            next.first().map(String::as_str),
            Some("t.jee.phy.kinematics.01"),
            "the plan's first week comes first"
        );
        let weeks: Vec<i64> = group(&value, "next")["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["weekIndex"].as_i64().unwrap_or(i64::MAX))
            .collect();
        assert!(weeks.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn an_item_is_claimed_by_exactly_one_group() {
        let config = load("fixtures/phase6");
        let state = crate::views::PlanState::load(&resources().join("fixtures/phase6"));
        let context = with_state(&config, &state);
        let value = today_view_json(&config, &context, &TodayRequest::default());
        let mut seen: Vec<String> = Vec::new();
        for group in value["groups"].as_array().unwrap() {
            for item in group["items"].as_array().unwrap() {
                let id = item["id"].as_str().unwrap().to_string();
                assert!(!seen.contains(&id), "{id} appears in two groups");
                seen.push(id);
            }
        }
        // `t.bayes` was due in January and is read against September: it is the
        // late group's member, with its lateness stated as a number.
        assert!(ids(&value, "late").contains(&"t.bayes".to_string()));
        let bayes = group(&value, "late")["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "t.bayes")
            .unwrap();
        assert!(bayes["lateDays"].as_i64().unwrap() > 200);
        assert!(
            bayes["reason"]
                .as_str()
                .unwrap()
                .starts_with("253 days late"),
            "the why-line leads with the lateness: {}",
            bayes["reason"]
        );
    }

    #[test]
    fn work_planned_for_today_is_committed_before_its_due_date() {
        let mut config = load("fixtures/phase6");
        let state = crate::views::PlanState::load(&resources().join("fixtures/phase6"));
        // The thin intention: one date field on the record, declared by its kind
        // (a preset field — `focus`) and set for today. Set in memory — the read
        // is what is under test, not the write path that sets it.
        let topic = config
            .types
            .get_mut("topic")
            .expect("the fixture declares topic");
        topic.fields.push(crate::model::FieldDef {
            key: "focus".into(),
            type_: "date".into(),
            label: Some("Planned for".into()),
            to: None,
            cardinality: None,
            required: None,
            options: None,
            expr: None,
        });
        for record in config
            .positioned_records
            .iter_mut()
            .map(|positioned| &mut positioned.record)
        {
            if record.id == "t.bayes" {
                record
                    .fields
                    .insert("focus".into(), serde_json::json!("2026-09-21"));
            }
        }
        let context = with_state(&config, &state);
        let value = today_view_json(&config, &context, &TodayRequest::default());
        assert_eq!(ids(&value, "committed"), vec!["t.bayes"]);
        assert!(!ids(&value, "late").contains(&"t.bayes".to_string()));
        let item = &group(&value, "committed")["items"][0];
        assert_eq!(item["reason"], "planned for today · was due 253 days ago");
        assert_eq!(item["lateDays"], 253);
    }

    #[test]
    fn a_gap_is_offered_only_when_something_is_waiting() {
        let config = load("fixtures/phase6");
        let state = crate::views::PlanState::load(&resources().join("fixtures/phase6"));
        let context = with_state(&config, &state);
        // The fixture's last activity is January and the read is September: the
        // fact is stated, with the days and a minute estimate behind it.
        let value = today_view_json(&config, &context, &TodayRequest::default());
        let gap = &value["gap"];
        assert!(gap["daysSince"].as_i64().unwrap() > 200, "{gap}");
        assert!(gap["waiting"].as_u64().unwrap() > 0);
        assert!(gap["since"].as_str().unwrap().starts_with("2026-01"));
        // A plan read on the day of its own last review has no gap to report.
        let same_day = today_view_json(
            &config,
            &context,
            &TodayRequest {
                date: Some("2026-01-10".into()),
                ..TodayRequest::default()
            },
        );
        assert!(same_day["gap"].is_null());
    }

    #[test]
    fn all_clear_ignores_the_backlog_that_is_always_there() {
        let config = load("presets/jee");
        let context = frozen(&config);
        let value = today_view_json(&config, &context, &TodayRequest::default());
        // The preset has unstarted work (a populated `next`), but nothing is
        // due, late or committed — which is what "nothing needs you" means.
        assert!(value["allClear"].as_bool().unwrap());
        assert!(group(&value, "next")["count"].as_u64().unwrap() > 0);
    }

    #[test]
    fn the_dated_things_come_from_their_own_kinds_date_field() {
        let config = load("presets/jee");
        let context = frozen(&config);
        // The preset's assessments are dated; read with a window wide enough to
        // reach them, they appear with their distance in words.
        let value = today_view_json(
            &config,
            &context,
            &TodayRequest {
                window: Window::Week,
                ..TodayRequest::default()
            },
        );
        for item in group(&value, "upcoming")["items"].as_array().unwrap() {
            assert!(item["daysUntil"].as_i64().unwrap() >= 0);
            assert!(item["dueDate"].as_str().is_some());
            assert!(
                !item["reason"].as_str().unwrap().contains("2026-"),
                "a distance in words, never an ISO date"
            );
        }
    }

    #[test]
    fn the_strip_is_seven_days_ending_today() {
        let config = load("presets/jee");
        let context = frozen(&config);
        let value = today_view_json(&config, &context, &TodayRequest::default());
        let days = value["days"].as_array().expect("a strip");
        assert_eq!(days.len(), 7);
        assert_eq!(days[6]["isToday"], true);
        assert_eq!(days[6]["date"], value["date"]);
        assert_eq!(days[6]["loggedMin"], value["totals"]["loggedMin"]);
        assert!(
            days.windows(2)
                .all(|pair| pair[0]["date"].as_str() < pair[1]["date"].as_str()),
            "oldest first"
        );
    }

    #[test]
    fn a_course_carries_its_wash_from_the_plans_own_order() {
        let config = load("presets/jee");
        let context = frozen(&config);
        let value = today_view_json(&config, &context, &TodayRequest::default());
        let washes = identity_washes(&config);
        assert_eq!(washes.len(), 3, "the three courses");
        assert_eq!(
            washes.get("c.jee.physics").map(String::as_str),
            Some("mint")
        );
        assert_eq!(
            washes.get("c.jee.maths").map(String::as_str),
            Some("butter")
        );
        let first = &group(&value, "next")["items"][0];
        assert_eq!(first["course"]["wash"], "mint");
    }

    #[test]
    fn totals_state_what_is_planned_logged_and_targeted() {
        let config = load("presets/jee");
        let context = frozen(&config);
        let value = today_view_json(&config, &context, &TodayRequest::default());
        assert_eq!(value["totals"]["targetMin"], 240);
        // A day with nothing logged states 0, not null — a zero is a fact.
        assert_eq!(value["totals"]["loggedMin"], 0);
        assert!(value["totals"]["plannedMin"].as_i64().unwrap() > 0);
    }
}
