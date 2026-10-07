<!-- Today focus widget host (COMPOSER §3.1). -->
<script lang="ts">
  import Variant from '../../../variants/Variant.svelte';
  import type { Coverage, TodayFocusProps, WhyFact } from '../../../variants/today-focus/props';
  import { coverageOf, openRecord, setAside } from '../../../variants/today-focus/host';
  import { app } from '../../../session.svelte';
  import { shared } from '../../shared.svelte';
  import { durationText, todayGroup, type TodayGroupId, type TodayItem } from '../../../types';

  /**
   * The queue's order, and the labels the screen states it with. Committed work
   * comes first because the student put it there; the rest is the engine's
   * declared order (late · due · next · stale).
   */
  const QUEUE: Array<{ id: TodayGroupId; label: string }> = [
    { id: 'committed', label: 'Planned for today' },
    { id: 'late', label: 'Late for review' },
    { id: 'due', label: 'Due today' },
    { id: 'next', label: 'Next in the plan' },
    { id: 'stale', label: 'Not touched in a while' },
  ];

  const rows = $derived(
    QUEUE.flatMap((group) =>
      (todayGroup(app.todayFacts, group.id)?.items ?? []).map((item) => ({
        item,
        group: group.id,
        label: group.label,
      })),
    ),
  );
  const totals = $derived(app.todayFacts?.totals ?? null);

  /** The card's item: the first row the engine ranked. */
  const focus = $derived(rows[0] ?? null);

  /**
   * The write a Start performs: the plan's own time-logging kind. A plan
   * without one says so instead of writing to a guessed field.
   */
  const sessionCommand = $derived(app.sessionKind() ? `${app.sessionKind()}.new` : null);

  function greeting(hour: number): string {
    if (hour < 5) return 'Still up';
    if (hour < 12) return 'Good morning';
    if (hour < 17) return 'Good afternoon';
    return 'Good evening';
  }

  /**
   * A date fact, in the words a student reads. `dueDate` is an ISO string and
   * the type doc is explicit that no screen parses one (`types.ts:98-99`), so it
   * is handed to `Date` once and every word below is taken off the parts.
   * Returns null when the record carries no usable date, and the caller omits
   * the clause — a printed placeholder is a lie about a fact that is not there.
   */
  function dueWords(dueDate: string): string | null {
    const at = new Date(dueDate);
    if (Number.isNaN(at.getTime())) return null;
    const weekday = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'][at.getDay()];
    const clock = `${String(at.getHours()).padStart(2, '0')}:${String(at.getMinutes()).padStart(2, '0')}`;
    return `${weekday} ${clock}`;
  }

  /**
   * The session card's why-line: ONE fact in two voices — a micro-caps key and
   * the value with its own number in ink. Precedence is the order a student
   * would rank them (a crossed deadline, then a deadline, then a lapse in
   * review, then a countdown), and the estimate is the fallback. A fact the
   * record does not carry is omitted rather than printed empty.
   */
  function whyFactOf(item: TodayItem): WhyFact | null {
    if (item.lateDays > 0)
      return { key: 'Late', value: item.lateDays === 1 ? '1 day late' : `${item.lateDays} days late` };
    if (item.dueDate) {
      const words = dueWords(item.dueDate);
      if (words) return { key: 'Due', value: words };
    }
    if (item.reviewedDaysAgo !== null)
      return {
        key: 'Last session',
        value:
          item.reviewedDaysAgo === 0
            ? 'today'
            : `${item.reviewedDaysAgo} ${item.reviewedDaysAgo === 1 ? 'day' : 'days'} ago`,
      };
    if (item.daysUntil !== null)
      return {
        key: 'Next assessment',
        value:
          item.daysUntil === 0
            ? 'today'
            : item.daysUntil === 1
              ? 'tomorrow'
              : `in ${item.daysUntil} days`,
      };
    if (item.est !== null) return { key: 'Estimated', value: `${item.est} min` };
    return null;
  }

  /**
   * The three states the masthead can be in, read from the day rather than
   * chosen: nothing to work on is the all-clear; the oldest thing waiting to
   * come back (the stale group) is the gap; anything else is the decision.
   */
  const focusMode = $derived<'decision' | 'clear' | 'gap'>(
    !focus ? 'clear' : focus.group === 'stale' ? 'gap' : 'decision',
  );

  const dateLine = $derived(
    app.todayFacts ? `${app.todayFacts.weekday} ${app.todayFacts.day} ${app.todayFacts.month}` : '',
  );

  /**
   * The focus card's arc — the course's coverage, read through the same helper
   * the Today screen uses so a composed screen and the designed one print the
   * same number for the same course. The key carries the plan's revision AND
   * the focused course: a publication re-reads, and so does the day ranking the
   * next thing — never a state change that leaves the last course's arc up.
   */
  let coverage = $state<Coverage | null>(null);
  let coverageFor: string | null = null;
  $effect(() => {
    const key = `${app.revision ?? ''}|${focus?.item.course?.id ?? ''}`;
    if (coverageFor === key) return;
    coverageFor = key;
    void coverageOf(focus?.item.course ?? null).then((read) => {
      if (coverageFor === key) coverage = read;
    });
  });

  const focusProps = $derived<TodayFocusProps>({
    greeting: greeting(new Date().getHours()),
    dateLine,
    mode: focusMode,
    title: focus?.item.label ?? null,
    course: focus?.item.course ? { label: focus.item.course.label, wash: focus.item.course.wash } : null,
    tag: focus?.label ?? null,
    why: focus ? whyFactOf(focus.item) : null,
    studiedToday: durationText(totals?.loggedMin ?? 0),
    minutes: shared.targetMin,
    onMinutes: (value) => (shared.targetMin = value),
    sessionCommand,
    onStart: () => {
      if (focus) void app.startSession(focus.item, shared.targetMin);
    },
    onPlan: () => void app.openSheet({ kind: 'today.add' }),
    coverage,
    kind: focus?.item.kind ?? null,
    onOpen: () => {
      if (focus) openRecord(focus.item);
    },
    onDefer: () => {
      if (focus) void setAside(focus.item, app.todayFacts?.date ?? null);
    },
  });
</script>

<Variant surface="today-focus" {...focusProps} />
