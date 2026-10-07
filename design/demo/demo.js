/* ══════════════════════════════════════════════════════════════════════════
   CADENCE demo — demo.js
   No build step, no dependencies, works from file://. State lives in memory:
   reload resets the seed data, which is the point — it is a design demo.

   Reads as: 1 DATA · 2 STATE · 3 UTIL · 4 ENGINE · 5 VIEWS · 6 SHEETS/SEARCH
             · 7 EVENTS · 8 ROUTER · 9 THEME · 10 INIT
   ═════════════════════════════════════════════════════════════════════════ */
(function () {
  'use strict';

  /* ─────────────────────────────────────────────────────────── 1 · DATA ── */
  var DAY = 86400000;
  var WD = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
  var MO = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

  function at(off, h, m) {
    var d = new Date();
    d.setDate(d.getDate() + off);
    d.setHours(h, m || 0, 0, 0);
    return d;
  }
  function startOf(d) { var x = new Date(d); x.setHours(0, 0, 0, 0); return x; }
  function daysTo(d) { return Math.round((startOf(d) - startOf(new Date())) / DAY); }
  function hm(d) { return String(d.getHours()).padStart(2, '0') + ':' + String(d.getMinutes()).padStart(2, '0'); }
  function fmtdue(d) {
    var n = daysTo(d);
    if (n === 0) return 'today ' + hm(d);
    if (n === 1) return 'tomorrow ' + hm(d);
    if (n === -1) return 'yesterday ' + hm(d);
    if (n < -1) return Math.abs(n) + ' days ago';
    return WD[d.getDay()] + ' ' + hm(d);
  }
  var COURSES = [
    {
      id: 'cs201', code: 'CS201', title: 'Data Structures & Algorithms',
      teacher: 'Dr. R. Menon', when: 'Mon · Wed · 10:00', wash: 'mint',
      exam: { name: 'Mid-semester', on: at(12, 9, 0) },
      topics: [
        ['Arrays & amortised analysis', 'solid', 12], ['Linked lists', 'solid', 19],
        ['Stacks & queues', 'solid', 15], ['Hash tables: collisions', 'learning', 9],
        ['Hash tables: resizing', 'new', null], ['Binary search trees', 'proof', 6],
        ['Heaps & priority queues', 'learning', 4], ['Graphs: BFS & DFS', 'new', null],
        ['Shortest paths', 'new', null], ['Sorting: comparison bounds', 'solid', 22],
        ['Union–find', 'new', null], ['Balanced trees: rotations', 'new', null]
      ]
    },
    {
      id: 'ma210', code: 'MA210', title: 'Linear Algebra',
      teacher: 'Prof. A. Iyer', when: 'Tue · Thu · 14:00', wash: 'lilac',
      exam: { name: 'Mid-semester', on: at(12, 9, 0) },
      topics: [
        ['Vector spaces', 'solid', 20], ['Span & independence', 'solid', 18],
        ['Basis & dimension', 'solid', 14], ['Linear maps', 'proof', 8],
        ['Matrix representation', 'learning', 8], ['Rank–nullity theorem', 'proof', 8],
        ['Determinants', 'learning', 5], ['Eigenvalues', 'learning', 4],
        ['Eigenvectors', 'new', null], ['Diagonalisation', 'new', null],
        ['Inner products', 'new', null], ['Orthogonal projection', 'new', null]
      ]
    },
    {
      id: 'cs120', code: 'CS120', title: 'Discrete Mathematics',
      teacher: 'Dr. S. Bose', when: 'Mon · Fri · 09:00', wash: 'butter',
      exam: { name: 'Quiz 4 (in class)', on: at(19, 9, 0) },
      topics: [
        ['Propositional logic', 'solid', 24], ['Proof techniques', 'solid', 21],
        ['Sets & relations', 'solid', 17], ['Functions & cardinality', 'proof', 13],
        ['Induction', 'learning', 4], ['Strong induction', 'proof', 8],
        ['Counting', 'learning', 6], ['Pigeonhole principle', 'learning', 5],
        ['Recurrence relations', 'learning', 3], ['Graphs & trees', 'new', null],
        ['Modular arithmetic', 'new', null], ['Generating functions', 'new', null]
      ]
    },
    {
      id: 'cs310', code: 'CS310', title: 'Operating Systems',
      teacher: 'Prof. K. Rao', when: 'Wed · Fri · 15:00', wash: 'sky',
      exam: { name: 'Mid-semester', on: at(11, 9, 0) },
      topics: [
        ['Processes & threads', 'solid', 16], ['Scheduling', 'learning', 7],
        ['Synchronisation', 'learning', 6], ['Deadlock', 'learning', 7],
        ['Memory: paging', 'proof', 9], ['Virtual memory', 'learning', 5],
        ['File systems', 'new', null], ['I/O & disks', 'new', null],
        ['Concurrency bugs', 'new', null], ['Protection & isolation', 'new', null],
        ['Virtual machines', 'new', null], ['Case study: Linux CFS', 'new', null]
      ]
    }
  ];

  var DEADLINES = [
    { id: 'd3', title: 'Quiz 3 — induction', course: 'cs120', due: at(-1, 17, 0), weight: '10%', status: 'overdue' },
    { id: 'd1', title: 'Problem set 7 — change of basis', course: 'ma210', due: at(1, 23, 59), weight: '5%', status: 'risk', why: '2 topics uncovered' },
    { id: 'd2', title: 'Assignment 4 — hash tables', course: 'cs201', due: at(2, 17, 0), weight: '15%', status: 'ok' },
    { id: 'd6', title: 'Reading response — concurrency', course: 'cs310', due: at(3, 9, 0), weight: '5%', status: 'ok' },
    { id: 'd4', title: 'Lab report — CPU scheduling', course: 'cs310', due: at(5, 17, 0), weight: '10%', status: 'risk', why: 'sits in the mid-sem week' },
    { id: 'd5', title: 'Assignment 5 — graph traversal', course: 'cs201', due: at(9, 17, 0), weight: '15%', status: 'ok' },
    { id: 'd7', title: 'Mid-semester exam', course: 'cs310', due: at(11, 9, 0), weight: '30%', status: 'risk', why: '3 topics uncovered' },
    { id: 'd8', title: 'Problem set 8 — eigenvalues', course: 'ma210', due: at(13, 23, 59), weight: '5%', status: 'ok' }
  ];

  var QUEUE = [
    { id: 'q1', title: 'Hash tables: collisions — 2 practice problems', course: 'cs201', due: at(2, 17, 0) },
    { id: 'q2', title: 'Problem set 7 — change of basis (Q3–Q5)', course: 'ma210', due: at(1, 23, 59) },
    { id: 'q3', title: 'Review the 12 due recalls', course: null, due: null, review: true },
    { id: 'q4', title: 'Scheduling — read §4.2, note 3 questions', course: 'cs310', due: at(3, 9, 0) },
    { id: 'q5', title: 'Induction quiz — redo the 2 wrong answers', course: 'cs120', due: at(-1, 17, 0) }
  ];

  var REVIEWS = [
    { id: 'r1', course: 'cs201', topic: 'Hash tables: collisions', lastSeen: 9, prompt: 'Why does separate chaining degrade to O(n), and what keeps lookups at O(1) expected?' },
    { id: 'r2', course: 'ma210', topic: 'Rank–nullity theorem', lastSeen: 8, prompt: 'State rank–nullity. What does it tell you about the solution space of Ax = b?' },
    { id: 'r3', course: 'cs120', topic: 'Strong induction', lastSeen: 8, prompt: 'State the strong induction hypothesis and one problem where it beats weak induction.' },
    { id: 'r4', course: 'cs310', topic: 'Deadlock: 4 Coffman conditions', lastSeen: 7, prompt: 'Name the four conditions. Give one way to break each.' },
    { id: 'r5', course: 'cs201', topic: 'Binary search trees', lastSeen: 6, prompt: 'Worst-case height of a BST built from sorted input — and the fix.' },
    { id: 'r6', course: 'ma210', topic: 'Diagonalisation', lastSeen: 6, prompt: 'When is a matrix diagonalisable? Give a 2×2 counterexample.' },
    { id: 'r7', course: 'cs120', topic: 'Pigeonhole principle', lastSeen: 5, prompt: 'State it, then give one application that is not obvious.' },
    { id: 'r8', course: 'cs310', topic: 'Paging vs segmentation', lastSeen: 5, prompt: 'One sentence each: what problem does each solve?' },
    { id: 'r9', course: 'cs201', topic: 'Amortised analysis', lastSeen: 4, prompt: 'Why is appending to a dynamic array amortised O(1)?' },
    { id: 'r10', course: 'ma210', topic: 'Eigenvalues', lastSeen: 4, prompt: 'How do you find eigenvalues without computing a determinant by hand?' },
    { id: 'r11', course: 'cs120', topic: 'Recurrence relations', lastSeen: 3, prompt: 'Solve T(n) = 2T(n/2) + n. Name the method you used.' },
    { id: 'r12', course: 'cs310', topic: 'Virtual memory: TLB', lastSeen: 3, prompt: 'What does a TLB miss cost, and why does locality matter?' }
  ];

  var SESSIONS = [
    { day: -6, course: 'cs201', minutes: 45, topic: 'Linked lists' },
    { day: -6, course: 'ma210', minutes: 30, topic: 'Basis & dimension' },
    { day: -5, course: 'cs310', minutes: 25, topic: 'Scheduling' },
    { day: -4, course: 'cs120', minutes: 55, topic: 'Counting' },
    { day: -4, course: 'cs201', minutes: 25, topic: 'Heaps & priority queues' },
    { day: -3, course: 'ma210', minutes: 40, topic: 'Rank–nullity theorem' },
    { day: -1, course: 'cs310', minutes: 50, topic: 'Memory: paging' },
    { day: -1, course: 'cs120', minutes: 30, topic: 'Recurrence relations' }
  ];

  /* Days −7 to −13: older log, kept as bare totals so the 30-day view has a
     history without inventing per-topic detail for it. */
  var PAST = [40, 0, 75, 55, 30, 0, 25];
  var PAST_C = {
    cs201: [0, 0, 35, 0, 30, 0, 0],
    ma210: [40, 0, 40, 0, 0, 0, 25],
    cs310: [0, 0, 0, 30, 0, 0, 0],
    cs120: [0, 0, 0, 25, 0, 0, 0]
  };

  /* ────────────────────────────────────────────────────────── 2 · STATE ── */
  var topics = [];
  COURSES.forEach(function (c) {
    c.topics.forEach(function (t, i) {
      topics.push({
        id: c.id + '-t' + (i + 1), course: c.id, name: t[0],
        status: t[1], lastSeen: t[2], seq: i + 1, est: 25
      });
    });
  });

  /* The demo runs inside week 6 of a 16-week term. Every "of term" figure
     in the UI is computed from this, not written into the copy. */
  var TERM = { week: 6, weeks: 16 };

  var state = {
    route: { name: 'today', param: null },
    theme: 'light',
    deferred: [],              // topic ids set aside today
    later: [],                 // queue ids set aside today
    queue: {},                 // queue id → 'done'
    reviews: REVIEWS.slice(),
    ratings: {},               // review id → 1..4
    redo: [],                  // review ids graded "forgot"
    revealed: false,           // current recall answer shown
    reviewsStarted: false,
    planRange: '14',           // days of horizon on the Plan view
    progressRange: '14',       // days of history on the Progress view
    target: 25,                // minutes the student wants a session to run
    durEdit: false,            // the session-length capsule is open
    focus: null,               // { label, course, topicId, seconds, target, handle }
    extra: [],                 // tasks the student captured
    open: {},                  // topic id → disclosure open
    q: '',                     // palette query
    layer: null,               // 'find' | 'add' | null
    sel: 0,                    // palette selection index
    pendingTopic: null         // topic to open after a cross-view jump
  };

  /* ─────────────────────────────────────────────────────────── 3 · UTIL ── */
  /* ── Which OS is this? ──
     One attribute decides, and everything derives from it: the titlebar inset
     and the shortcut labels. A keys hint that says ⌘K on Windows is not a
     style choice, it is a wrong instruction. The attribute can still be set
     by hand to preview the other platform. */
  function detectOs() {
    var ua = navigator.userAgentData && navigator.userAgentData.platform;
    var p = ua || navigator.platform || '';
    return /mac|iphone|ipad/i.test(p) ? 'mac' : 'win';
  }
  function osIsMac() { return $('.cd-titlebar').getAttribute('data-os') !== 'win'; }
  function modKey() { return osIsMac() ? '\u2318K' : 'Ctrl K'; }
  function applyOs() {
    var chip = $('.cd-kbd');
    if (chip) chip.textContent = modKey();
  }
  function initOs() {
    $('.cd-titlebar').setAttribute('data-os', detectOs());
    applyOs();
  }

  function $(s, r) { return (r || document).querySelector(s); }
  function esc(s) {
    return String(s).replace(/[&<>"']/g, function (c) {
      return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c];
    });
  }
  function course(id) { for (var i = 0; i < COURSES.length; i++) if (COURSES[i].id === id) return COURSES[i]; return null; }
  function topicsOf(id) { return topics.filter(function (t) { return t.course === id; }); }
  function topic(id) { for (var i = 0; i < topics.length; i++) if (topics[i].id === id) return topics[i]; return null; }
  function covered(id) { return topicsOf(id).filter(function (t) { return t.status !== 'new'; }).length; }
  function totalTopics() { return topics.length; }
  function coveredAll() { return topics.filter(function (t) { return t.status !== 'new'; }).length; }
  function solidAll() { return topics.filter(function (t) { return t.status === 'solid'; }).length; }
  /* ───────────────────────────────────────────────────────── 4 · ENGINE ── */
  /* Explainable by construction: same inputs, same answer, and the why-line
     names the signal that won. */
  function nearestDeadline(courseId) {
    var best = null;
    DEADLINES.forEach(function (d) {
      if (d.course !== courseId || daysTo(d.due) < 0) return;
      if (!best || d.due < best.due) best = d;
    });
    return best;
  }
  function pickNext() {
    var best = null;
    topics.forEach(function (t) {
      if (t.status === 'solid' || state.deferred.indexOf(t.id) !== -1) return;
      var ex = daysTo(course(t.course).exam.on);
      var dl = nearestDeadline(t.course);
      var dlDays = dl ? daysTo(dl.due) : 99;
      var score = (t.status === 'proof' ? 3.2 : t.status === 'learning' ? 2.4 : 1.4)
        + Math.max(0, 16 - ex) * 1.6
        + (t.lastSeen == null ? 1.2 : Math.min(t.lastSeen, 21) * 0.55)
        + (dlDays <= 3 ? 4 : dlDays <= 7 ? 2 : 0)
        + (t.lastSeen != null && t.lastSeen >= 8 ? 2.4 : 0);
      if (!best || score > best.score) best = { topic: t, score: score, ex: ex, dl: dl, dlDays: dlDays };
    });
    if (!best) return null;
    var t = best.topic, why;
    if (t.lastSeen != null && t.lastSeen >= 7) {
      why = (best.dl && best.dlDays <= 3 ? 'due ' + fmtdue(best.dl.due) + ' · ' : '') + 'last reviewed ' + t.lastSeen + ' days ago';
    } else if (best.ex <= 16) {
      why = 'exam in ' + best.ex + ' days · ' + (t.lastSeen == null ? 'never self-tested' : 'reviewed ' + t.lastSeen + ' days ago');
    } else if (best.dl) {
      why = 'due ' + fmtdue(best.dl.due) + ' · ' + course(t.course).code + ' sequence';
    } else {
      why = 'next in the course sequence';
    }
    return { title: t.name, course: t.course, why: why, minutes: t.est, topicId: t.id, status: t.status };
  }

  /* ──────────────────────────────────────────────────── 3b · INSTRUMENTS ──
     Markup emitters. One instrument per job: the chip says which course, the
     meter says how much of the term is covered, the gauge says the same thing
     once per course at display size, the stems say when the work happened.
     No emoji (they cannot inherit ink and render differently per OS), and no
     colour spent on anything that is not one of the three colour jobs. */
  var LADDER = { 'new': 0, 'learning': 1, 'proof': 2, 'solid': 3 };
  var STATELBL = { 'new': 'Not started', 'learning': 'Learning', 'proof': 'Proof', 'solid': 'Solid' };
  var STATUS_CHIP = { 'new': 'info', 'learning': 'risk', 'proof': 'ok', 'solid': 'ok' };

  function icon(name, size) {
    return '<svg width="' + (size || 14) + '" height="' + (size || 14) + '" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><use href="#i-' + name + '"/></svg>';
  }
  function glyphTile(name, size) {
    return '<span class="cd-ictile">' + icon(name, size || 17) + '</span>';
  }
  /* Identity: the wash is the mark. A course never gets a 3px side rule. */
  function idChip(id) {
    var c = course(id);
    if (!c) return '<span class="cd-chip">Personal</span>';
    return '<span class="cd-chip cd-chip--code" data-w="' + c.wash + '">' + c.code + '</span>';
  }
  function meter(a, b, opt) {
    opt = opt || {};
    var p = b ? Math.round(a / b * 100) : 0;
    var cls = 'cd-meter' + (opt.lg ? ' cd-meter--lg' : '')
      + (opt.head ? ' cd-meter--head' : '') + (opt.wash ? ' cd-meter--wash' : '');
    return '<span class="' + cls + (opt.wrap ? ' ' + opt.wrap : '') + '">' +
      '<span class="cd-meter__track"><span class="cd-meter__fill" style="--v:' + p + '%"></span></span>' +
      (opt.label === false ? '' : '<span class="cd-meter__frac">' + (opt.label || p + '%') + '</span>') +
      '</span>';
  }
  function covOf(id) { return Math.round(covered(id) / topicsOf(id).length * 100); }
  /* The gauge: one arc, 180°, rounded caps, deep course ink on a light track.
     Not a donut — a donut is a dashboard ornament; a half-arc reads as a dial
     on an instrument, which is what this actually is. */
  function gauge(pct, cap) {
    var arc = Math.max(0, Math.min(100, pct));
    return '<div class="cd-gauge">' +
      '<svg viewBox="0 0 132 74" aria-hidden="true">' +
        '<path class="cd-gauge__track" d="M8 66 A 58 58 0 0 1 124 66" />' +
        '<path class="cd-gauge__arc" pathLength="100" stroke-dasharray="' + arc + ' 100" d="M8 66 A 58 58 0 0 1 124 66" />' +
      '</svg>' +
      '<div class="cd-gauge__val">' + pct + '%</div>' +
      '<div class="cd-gauge__cap">' + esc(cap) + '</div>' +
    '</div>';
  }
  function statusChip(st) {
    var cls = STATUS_CHIP[st] || 'info';
    return '<span class="cd-chip cd-chip--' + cls + '">' + esc(STATELBL[st] || st) + '</span>';
  }
  function countdown(n) {
    if (n < 0) return Math.abs(n) + 'd late';
    if (n === 0) return 'today';
    if (n === 1) return 'tomorrow';
    return 'in ' + n + ' days';
  }
  function ago(n) {
    if (n == null) return 'never self-tested';
    if (n === 0) return 'today';
    if (n === 1) return 'yesterday';
    return n + ' days ago';
  }

  /* ── Session arithmetic — the only "statistics" in the product, and every
        number below is a sum of the log, never an estimate. ─────────────── */
  function minutesOn(dayOff) {
    if (dayOff <= -7) return PAST[-dayOff - 7] || 0;   // the seven days before the log
    return SESSIONS.filter(function (s) { return s.day === dayOff; })
      .reduce(function (a, s) { return a + s.minutes; }, 0);
  }
  function minutesFor(courseId, from, to) {
    if (from <= -7) return (PAST_C[courseId] || []).filter(function (_, i) { return -i - 7 >= from && -i - 7 <= to; })
      .reduce(function (a, m) { return a + m; }, 0) +
      SESSIONS.filter(function (s) { return s.course === courseId && s.day >= from && s.day <= to; })
        .reduce(function (a, s) { return a + s.minutes; }, 0);
    return SESSIONS.filter(function (s) { return s.course === courseId && s.day >= from && s.day <= to; })
      .reduce(function (a, s) { return a + s.minutes; }, 0);
  }
  function totalMinutes(from, to) {
    var t = 0;
    for (var d = from; d <= to; d++) t += minutesOn(d);
    return t;
  }
  function hmOf(mins) {
    var h = Math.floor(mins / 60), m = mins % 60;
    return h ? h + 'h' + (m ? ' ' + m + 'm' : '') : m + 'm';
  }
  /* The zero-row: a day with nothing logged is reported as a fact with no
     adjective attached. No broken-streak guilt, no red. */
  function zeroDays(from, to) {
    var n = 0;
    for (var d = from; d <= to; d++) if (minutesOn(d) === 0) n++;
    return n;
  }
  function emptyWeekNote() {
    var n = zeroDays(-6, 0);
    if (n === 0) return 'Bars are logged minutes only. All seven days have work on them.';
    return 'Bars are logged minutes only. ' + n + (n === 1 ? ' day has' : ' days have') +
      ' nothing logged — a fact, not a warning.';
  }
  /* QUEUE is the seeded term; state.extra is whatever the student added. */
  function allTasks() { return QUEUE.concat(state.extra); }
  function queueItems() {
    return allTasks().filter(function (q) {
      return !state.queue[q.id] && state.later.indexOf(q.id) === -1;
    });
  }
  function dueRecalls() {
    return state.reviews.filter(function (r) { return !state.ratings[r.id]; });
  }
  function lvl(st) { return LADDER[st]; }

  /* ───────────────────────────────────────────────── 3c · BLOCK BUILDERS ──
     Four shapes carry every screen: the page head, the card, the row and the
     tile. A view is an arrangement of these, never a new widget. */
  var USER = 'Avinash';   // the demo's signed-in student
  var EST = { q1: 25, q2: 40, q3: 15, q4: 30, q5: 20 };

  function pageHead(title, sub, aside) {
    return '<header class="cd-pagehead">' +
      '<div><h1 class="cd-pagehead__title">' + title + '</h1>' +
      (sub ? '<p class="cd-pagehead__sub">' + sub + '</p>' : '') + '</div>' +
      (aside ? '<div class="cd-pagehead__aside">' + aside + '</div>' : '') +
    '</header>';
  }
  function cardHead(iconName, title, sub, wash) {
    return '<div class="cd-card__head"' + (wash ? ' data-w="' + wash + '"' : '') + '>' +
      glyphTile(iconName) +
      '<div><div class="cd-card__title">' + title + '</div>' +
      (sub ? '<div class="cd-card__sub">' + sub + '</div>' : '') + '</div>' +
      '<span class="cd-card__spacer"></span>';
  }
  function card(iconName, title, sub, right, body, wash) {
    return '<section class="cd-card"' + (wash ? ' data-w="' + wash + '"' : '') + '>' +
      (title ? cardHead(iconName, title, sub, wash) + (right || '') + '</div>' : '') +
      body +
    '</section>';
  }
  /* The primary way this app gets something done: pick one thing, start the
     clock, log the minutes. It is offered on every row, not only the top. */
  /* Every row can be started in place. The one row the app would give you
     next is the dark pill; the rest are quiet, so a list never becomes a
     column of black buttons. */
  function playBtn(label, id, kind, quiet) {
    return '<button type="button" class="cd-pill cd-pill--sm' + (quiet ? ' cd-pill--quiet' : '') + '" data-act="start" data-kind="' + kind + '" data-id="' + id + '">' +
      icon('clock', 13) + '<span>' + esc(label) + '</span></button>';
  }
  /* The session length, edited where it is read. Fused it is one capsule
     that says "25 min" and carries a pencil; opened it splits into a field
     and a check. It is the same input in both states, so the number never
     jumps when the editor opens. */
  function durCapsule() {
    var e = state.durEdit;
    return '<div class="cd-dur" data-edit="' + (e ? 1 : 0) + '">' +
      '<span class="cd-dur__part cd-dur__part--v">' +
        '<input id="durV" type="text" inputmode="numeric" maxlength="3" value="' + state.target + '"' +
          ' tabindex="' + (e ? 0 : -1) + '" aria-label="Session length in minutes" />' +
        '<span class="cd-dur__unit">min</span>' +
      '</span>' +
      '<button type="button" class="cd-dur__part cd-dur__part--act" data-act="' + (e ? 'dur-save' : 'dur-open') + '"' +
        ' aria-label="' + (e ? 'Save session length' : 'Change session length') + '">' +
        '<span class="cd-dur__ico cd-dur__ico--pencil">' + icon('pencil', 15) + '</span>' +
        '<span class="cd-dur__ico cd-dur__ico--check">' + icon('check', 15) + '</span>' +
      '</button>' +
    '</div>';
  }
  /* The number is the student's; the estimate in the tag above it is the
     app's. A length is clamped to something a session can actually be, and a
     field left empty keeps what was there rather than setting zero. */
  function commitDur() {
    var el = $('#durV');
    var n = el ? parseInt(String(el.value).replace(/[^0-9]/g, ''), 10) : NaN;
    if (n) state.target = Math.min(Math.max(n, 5), 180);
    state.durEdit = false;
    render();
    /* Both callers land here — the check and the Enter key — so the focus
       restore lives here too, or the keyboard path drops the caret on body. */
    setTimeout(function () { var b = $('.cd-dur__part--act'); if (b) b.focus(); }, 30);
  }
  function quietBtn(label, act, id, iconName) {
    return '<button type="button" class="cd-iconbtn" data-act="' + act + '" data-id="' + id + '" aria-label="' + esc(label) + '" title="' + esc(label) + '">' +
      icon(iconName, 15) + '</button>';
  }
  function taskRow(q, isNext) {
    var done = state.queue[q.id] === 'done';
    var late = !done && q.due && daysTo(q.due) < 0;
    var c = q.course ? course(q.course) : null;
    var meta = [];
    if (c) meta.push(idChip(c.id));
    else meta.push('<span class="cd-chip cd-chip--info">Recall</span>');
    if (q.review) meta.push('<span>' + dueRecalls().length + ' questions</span>');
    else if (q.due) meta.push('<span>' + (late ? Math.abs(daysTo(q.due)) + ' day' + (Math.abs(daysTo(q.due)) === 1 ? '' : 's') + ' late' : dueWhen(q.due)) + '</span>');
    meta.push('<span>' + (EST[q.id] || 20) + ' min</span>');
    return '<div class="cd-task' + (late ? ' cd-task--late' : '') + '" data-done="' + (done ? 1 : 0) + '"' +
        (c ? ' data-w="' + c.wash + '"' : '') + '>' +
      '<button type="button" class="cd-check" data-act="' + (done ? 'undo-done' : 'done') + '" data-id="' + q.id + '"' +
        ' aria-pressed="' + (done ? 'true' : 'false') + '" aria-label="' + (done ? 'Reopen' : 'Mark done') + ': ' + esc(q.title) + '">' +
        icon('check', 13) + '</button>' +
      '<div>' +
        '<div class="cd-task__title">' + esc(q.title) + '</div>' +
        '<div class="cd-task__meta">' + meta.join('') + '</div>' +
      '</div>' +
      '<div class="cd-task__right">' +
        (done ? '<span class="cd-task__mins">done</span>' :
          '<div class="cd-task__acts">' + quietBtn('Set aside for today', 'defer-task', q.id, 'clock') + '</div>' +
          playBtn(q.review ? 'Review' : 'Start', q.id, q.review ? 'review' : 'task', !isNext)) +
      '</div>' +
    '</div>';
  }
  /* One deadline, one shape — used by the Plan ledger and by Today's Coming
     up column. Three deadlines that are the same kind of thing read as rows
     of one card; three slabs would be three competing products. */
  function deadlineRow(d, link) {
    var c = course(d.course);
    var n = daysTo(d.due);
    var late = n < 0;
    var body =
      '<span class="cd-datetile">' + d.due.getDate() + '</span>' +
      '<div>' +
        '<div class="cd-task__title">' + esc(d.title) + '</div>' +
        '<div class="cd-task__meta">' + idChip(c.id) + '<span>' + WD[d.due.getDay()] + ' ' + hm(d.due) + '</span>' +
          '<span>' + esc(d.weight) + ' of the grade</span>' +
          (d.why && !late ? '<span class="cd-chip cd-chip--risk">' + esc(d.why) + '</span>' : '') + '</div>' +
      '</div>' +
      '<div class="cd-task__right">' +
        '<span class="cd-chip cd-chip--' + (late ? 'overdue' : (n <= 1 || d.status === 'risk') ? 'risk' : 'ok') + '">' +
          countdown(n) + '</span>' +
      '</div>';
    return link
      ? '<a class="cd-task" data-w="' + c.wash + '" href="#/plan">' + body + '</a>'
      : '<div class="cd-task" data-w="' + c.wash + '">' + body + '</div>';
  }
  /* A course as an object: wash, code, coverage, the next thing due. */
  function courseTile(c) {
    var dl = nearestDeadline(c.id);
    var pct = covOf(c.id);
    return '<button type="button" class="cd-tile d-course" data-w="' + c.wash + '" data-act="open-course" data-id="' + c.id + '">' +
      '<div class="cd-tile__top">' +
        '<span class="cd-chip cd-chip--onwash">' + c.code + '</span>' +
        '<span class="cd-chip cd-chip--onwash">' + covered(c.id) + '/' + topicsOf(c.id).length + ' topics</span>' +
      '</div>' +
      '<div class="cd-tile__title">' + esc(c.title) + '</div>' +
      '<div class="cd-tile__sub">' + esc(c.teacher) + ' · ' + esc(c.when) + '</div>' +
      '<div class="cd-tile__foot">' +
        '<span class="cd-tile__num">' + pct + '%</span>' +
        '<span class="cd-tile__cap">' + (dl ? 'next: ' + esc(dl.title) : 'nothing due') + '</span>' +
        '<span class="cd-tile__cta"><span class="cd-round">' + icon('arrow', 14) + '</span></span>' +
      '</div>' +
    '</button>';
  }
  function deadlineTile(d) {
    return deadlineRow(d, true);
  }
  function statTile(v, l, wash) {
    return '<div class="cd-stat' + (wash ? ' cd-stat--wash' : '') + '"' + (wash ? ' data-w="' + wash + '"' : '') + '>' +
      '<div class="cd-stat__v">' + v + '</div><div class="cd-stat__l">' + l + '</div></div>';
  }
  /* Stems, not an area chart: a bar per day is read as a fact, an area is
     read as a trend, and this app only ever knows facts. */
  function stemChart(days) {
    var cap = 90;                       // a full day for this student
    var max = Math.max(cap, Math.max.apply(null, days.map(function (d) { return d.minutes; })));
    var bars = days.map(function (d) {
      var pct = Math.round(d.minutes / max * 100);
      /* No data-w here on purpose: a day is not a course. A stem's colour is
         its value, and its value has exactly two states — work happened or it
         did not — so the ink is the fact and the empty track is the absence. */
      return '<i class="cd-stem" data-empty="' + (d.minutes ? 0 : 1) + '"' +
        (d.today ? ' data-today="1"' : '') +
        ' style="--v:' + Math.max(4, pct) + '%" title="' + esc(d.label + ' — ' + (d.minutes ? hmOf(d.minutes) : 'nothing logged')) + '"></i>';
    }).join('');
    var axis = days.map(function (d) { return '<span>' + d.axis + '</span>'; }).join('');
    return '<div class="cd-chart">' +
      '<div class="cd-chart__plot">' + bars +
        '<span class="cd-chart__cap" data-label="full day · 90m"></span>' +
      '</div>' +
      '<div class="cd-chart__axis">' + axis + '</div>' +
    '</div>';
  }
  /* Last-7 companion for the Today rail: one line, no axis, read at a glance. */
  /* One builder for every stem chart. Plan, Progress and Today's week all
     count days the same way, so the range, the labels and the today flag
     cannot drift apart between screens. */
  function chartDays(range, axis) {
    var out = [];
    for (var i = range - 1; i >= 0; i--) {
      var d = new Date(); d.setDate(d.getDate() - i);
      out.push({
        minutes: minutesOn(-i),
        today: i === 0,
        label: fullDate(d),
        axis: axis === 'weekday'
          ? WD[d.getDay()].slice(0, 1)
          : (i % 3 === 0 || i === 0 ? String(d.getDate()) : '·')
      });
    }
    return out;
  }

  /* ── Dates, said the way a person says them ──────────────────────────── */
  function fullDate(d) { return WD[d.getDay()] + ' ' + d.getDate() + ' ' + MO[d.getMonth()]; }
  function dueWhen(d) { return WD[d.getDay()] + ' ' + d.getDate() + ' ' + MO[d.getMonth()] + ' · ' + hm(d); }
  function longDate(d) { return WD[d.getDay()] + ' ' + d.getDate() + ' ' + MO[d.getMonth()] + ' ' + d.getFullYear(); }
  function dayPart() {
    var h = new Date().getHours();
    return h < 12 ? 'morning' : h < 17 ? 'afternoon' : 'evening';
  }

  /* ─────────────────────────────────────────────────────────── 5 · TODAY ──
     The whole screen is one answer and its ledger, in that order. */
  function focusCard() {
    var nxt = pickNext();
    if (!nxt) {
      return '<section class="cd-focus" data-w="mint">' +
        '<div><div class="cd-focus__eyebrow"><span class="cd-sparkle">' + icon('sparkle', 16) + '</span>' +
        '<span>Nothing urgent is left</span></div>' +
        '<h2 class="cd-focus__text">Today is yours</h2>' +
        '<p class="cd-focus__why">Every uncovered topic has room before its exam. If you want to work anyway, pick anything from the console.</p>' +
        '<div class="cd-focus__acts"><button type="button" class="cd-pill cd-pill--lg" data-act="pal">' + icon('search', 14) + 'Choose a topic</button></div></div>' +
      '</section>';
    }
    var c = course(nxt.course);
    var t = topic(nxt.topicId);
    var whyBits = nxt.why.split(' · ');
    var first = '<b>' + esc(whyBits[0]) + '</b>';
    var rest = whyBits.slice(1).map(esc);
    /* The ladder is only climbed by a self-test, and this line says which
       rung the topic is on and what a session would do to it. */
    var rung = t.status === 'new' ? 'Not started yet — a first pass puts it on the board.'
      : t.status === 'learning' ? 'Learning — a session moves it toward proof.'
      : t.status === 'proof' ? 'Proof — one clean recall makes it solid.'
      : 'Solid.';
    return '<section class="cd-focus" data-w="' + c.wash + '">' +
      '<div>' +
        '<div class="cd-focus__eyebrow">' +
          idChip(c.id) +
          '<span class="cd-focus__tag">Recommended next</span>' +
          /* "Not this" sits with the claim, not with the actions: it is a
             verdict on the recommendation, not a way to run the session.
             The length lives on the capsule beside Start and nowhere else. */
          '<span class="cd-focus__spacer"></span>' +
          '<button type="button" class="cd-pill cd-pill--sm cd-pill--quiet" data-act="defer-topic" data-id="' + nxt.topicId + '">Not this</button>' +
        '</div>' +
        '<h2 class="cd-focus__text">' + esc(nxt.title) + '</h2>' +
        '<p class="cd-focus__why">' + [first].concat(rest).join(' · ') + ' · ' + esc(rung) + '</p>' +
        '<div class="cd-focus__acts">' +
          playBtn('Start', nxt.topicId, 'topic').replace('cd-pill--sm', 'cd-pill--lg') +
          durCapsule() +
          '<button type="button" class="cd-pill cd-pill--ghost cd-pill--lg" data-act="topic" data-id="' + nxt.topicId + '">Open topic</button>' +
        '</div>' +
      '</div>' +
      '<div data-w="' + c.wash + '">' + gauge(covOf(c.id), c.code + ' covered') + '</div>' +
    '</section>';
  }

  /* Late first, then whatever is due soonest, then the undated rest. The
     queue has to read in the order it should be worked, or the top row
     means nothing. */
  function byUrgency(a, b) {
    return (a.due ? daysTo(a.due) : 99) - (b.due ? daysTo(b.due) : 99);
  }

  function viewToday() {
    var q = queueItems().sort(byUrgency);
    var doneList = allTasks().filter(function (x) { return state.queue[x.id] === 'done'; });
    var remaining = q.filter(function (x) { return !x.review; })
      .reduce(function (a, x) { return a + (EST[x.id] || 20); }, 0) + (q.some(function (x) { return x.review; }) ? 15 : 0);
    var sub = fullDate(new Date()) + ' · week ' + TERM.week + ' of ' + TERM.weeks +
      ' · ' + hmOf(totalMinutes(-6, 0)) + ' studied this week';

    var rows = q.map(function (x, i) { return taskRow(x, i === 0); })
      .concat(doneList.map(function (x) { return taskRow(x, false); })).join('');
    var laterLine = state.later.length
      ? '<div class="cd-dashed"><b>' + state.later.length + ' set aside for today</b>' +
        '<span>Still there. Nothing is deleted by hiding it.</span>' +
        '<button type="button" class="cd-pill cd-pill--ghost cd-pill--sm" data-act="reset-later">Bring them back</button></div>'
      : '';
    var queueBody = q.length || doneList.length
      ? '<div class="cd-tasks">' + rows + '</div>'
      : '<div class="cd-empty">' + '<span class="cd-empty__art">' + icon('check', 34) + '</span>' +
        '<div class="cd-empty__t">The queue is empty</div>' +
        '<div class="cd-empty__s">Nothing is scheduled for today. Recalls and deadlines come back on their own.</div>' +
        '<button type="button" class="cd-pill cd-pill--ghost cd-pill--sm" data-act="quick-add">Add something anyway</button></div>';

    var comingUp = DEADLINES
      .filter(function (d) { return daysTo(d.due) >= -3; })
      .sort(function (a, b) { return a.due - b.due; })
      .slice(0, 3)
      .map(deadlineTile).join('');

    return pageHead(
      'Good ' + dayPart() + ', ' + USER,
      sub
    ) +
    '<div class="cd-grid-2">' +
      '<div class="cd-stack">' +
        focusCard() +
        card('calendar-day', "Today's queue",
          q.length + ' left · about ' + hmOf(remaining) + ' of work' + (doneList.length ? ' · ' + doneList.length + ' done' : ''),
          '<button type="button" class="cd-pill cd-pill--quiet cd-pill--sm" data-act="quick-add">' + icon('plus', 13) + 'Add</button>',
          queueBody + laterLine) +
      '</div>' +
      '<div class="cd-rail">' +
        card('flag', 'Coming up', 'Next in the calendar', '', '<div class="cd-tasks">' + comingUp + '</div>') +
        card('progress', 'This week', hmOf(totalMinutes(-6, 0)) + ' across 4 courses', '',
          '<div class="d-chart-sm">' + stemChart(chartDays(7, 'weekday')) + '</div>' +
          '<div class="d-note">' + icon('info', 14) + '<span>' + emptyWeekNote() + '</span></div>') +
      '</div>' +
    '</div>';
  }

  /* ──────────────────────────────────────────────────────────── 6 · PLAN ──
     Where the term is going: the week ahead, the exams, the deadlines, the
     load. Same four blocks as Today, arranged around a calendar instead of
     a recommendation. */
  function dayStrip() {
    var days = '';
    for (var i = 0; i < 7; i++) {
      var d = new Date(); d.setDate(d.getDate() + i);
      var due = DEADLINES.filter(function (x) { return daysTo(x.due) === i; }).length;
      var mins = i === 0 ? minutesOn(0) : minutesOn(i);
      var v = due ? due + ' due' : (mins ? hmOf(mins) : '—');
      days += '<div class="cd-day"' + (i === 0 ? ' aria-current="date"' : '') +
        ' data-nothing="' + (due || mins ? 0 : 1) + '">' +
        '<span class="cd-day__lbl">' + WD[d.getDay()] + '</span>' +
        '<span class="cd-day__n">' + d.getDate() + '</span>' +
        '<span class="cd-day__v">' + v + '</span>' +
      '</div>';
    }
    return '<div class="cd-days">' + days + '</div>';
  }
  function viewPlan() {
    var range = parseInt(state.planRange, 10);
    var inRange = DEADLINES.filter(function (d) { var n = daysTo(d.due); return n >= -7 && n <= range; })
      .sort(function (a, b) { return a.due - b.due; });
    var exams = COURSES.map(function (c) { return { c: c, n: daysTo(c.exam.on) }; })
      .sort(function (a, b) { return a.n - b.n; });

    var days = chartDays(range, 'date');
    var studyDays = days.filter(function (x) { return x.minutes > 0; }).length;

    var seg = '<div class="cd-seg" role="group" aria-label="Plan horizon">' +
      [['7', '1 week'], ['14', '2 weeks'], ['30', 'A month']].map(function (o) {
        return '<button type="button" class="cd-seg__pill" data-act="plan-range" data-v="' + o[0] + '"' +
          ' aria-pressed="' + (state.planRange === o[0]) + '">' + o[1] + '</button>';
      }).join('') + '</div>';

    var examsHtml = exams.map(function (e) {
      return '<div class="d-exam" data-w="' + e.c.wash + '">' +
        '<div class="d-exam__n num">' + e.n + '<i>' + (e.n === 1 ? 'day' : 'days') + '</i></div>' +
        '<div class="d-exam__t">' + e.c.code + ' · ' + esc(e.c.exam.name) + '</div>' +
        '<div class="d-exam__s">' + covOf(e.c.id) + '% covered · ' + longDate(e.c.exam.on) + '</div>' +
      '</div>';
    }).join('');

    var load = COURSES.map(function (c) {
      var mins = minutesFor(c.id, -(range - 1), 0);
      var total = Math.max(1, COURSES.reduce(function (a, x) { return a + minutesFor(x.id, -(range - 1), 0); }, 0));
      return '<div class="d-loadrow" data-w="' + c.wash + '">' +
        '<div><div class="cd-rowflex"><span class="d-covrow__name">' + c.code + '</span>' +
        '<span class="cd-hint">' + esc(c.title) + '</span></div>' +
        meter(mins, total, { head: true, label: hmOf(mins), wash: true }) + '</div>' +
        '<div class="cd-hint">' + covered(c.id) + ' of ' + topicsOf(c.id).length + ' topics covered</div>' +
      '</div>';
    }).join('');

    return pageHead('Plan', 'Week ' + TERM.week + ' of ' + TERM.weeks + ' · ' + inRange.length +
      ' deadlines in the next ' + range + ' days · ' + exams.length + ' exams left in the term', seg) +
    '<div class="cd-stack">' +
      '<div class="cd-grid-2">' +
        card('calendar-day', 'The week ahead', 'Today plus six days', '',
          dayStrip() +
          '<div class="d-note">' + icon('info', 14) + '<span>Each circle is a day you can actually use. A dash means nothing is logged and nothing is due — that is room, not failure.</span></div>') +
        card('flag', 'Exams', 'Every one is on the calendar', '', '<div class="d-exams">' + examsHtml + '</div>') +
      '</div>' +
      card('clock', 'Deadlines in view', inRange.length + ' items · overdue first', '',
        '<div class="cd-tasks">' + inRange.map(deadlineRow).join('') + '</div>') +
      card('progress', 'Load', studyDays + '/' + range + ' days have logged work', '',
        stemChart(days) +
        '<div class="d-note">' + icon('info', 14) + '<span>' + hmOf(totalMinutes(-(range - 1), 0)) +
        ' logged in ' + range + ' days. The dashed ceiling is a full 90-minute day — reaching it is a choice, not a target.</span></div>' +
        '<div class="d-note">' + icon('info', 14) + '<span>The bars share the selected window with the courses below. Switch the horizon to change both.</span></div>') +
      card('courses', 'Where the work has gone', 'Logged minutes, by course', '', load) +
    '</div>';
  }

  /* ───────────────────────────────────────────────────────── 7 · COURSES ──
     A course is a room with a colour; this screen is the list of rooms. */
  function viewCourses() {
    var all = COURSES.map(courseTile).join('');
    var rows = COURSES.map(function (c) {
      var ts = topicsOf(c.id);
      var by = { solid: 0, proof: 0, learning: 0, new: 0 };
      ts.forEach(function (t) { by[t.status]++; });
      return '<div class="d-covrow" data-w="' + c.wash + '">' +
        '<span class="cd-chip cd-chip--code">' + c.code + '</span>' +
        '<div><div class="d-covrow__name">' + esc(c.title) + '</div>' +
          meter(covered(c.id), ts.length, { head: true, label: covOf(c.id) + '%' }) + '</div>' +
        '<div class="cd-hint">' + by.solid + ' solid · ' + by.proof + ' proof · ' +
          by.learning + ' learning · <span class="cd-code">' + by['new'] + ' untouched</span></div>' +
      '</div>';
    }).join('');
    return pageHead('Courses', COURSES.length + ' courses · ' +
      totalTopics() + ' topics · ' + coveredAll() + ' covered (' + Math.round(coveredAll() / totalTopics() * 100) + '%)',
      '<button type="button" class="cd-pill cd-pill--ghost" data-act="pal">' + icon('search', 14) + 'Find a topic</button>') +
    '<div class="cd-stack">' +
      '<div class="cd-tiles">' + all + '</div>' +
      card('target', 'Where each course stands', 'Coverage counts a topic once you have opened it — mastery is a separate ladder', '', rows) +
    '</div>';
  }

  /* A topic row is a disclosure, not a table cell. The three-bar ladder on
     the left is the mastery state; the chip on the right is the same fact in
     words, because a mark alone should never be the only carrier. */
  function topicRow(t) {
    var c = course(t.course);
    var open = !!state.open[t.id];
    var reviewsOn = state.reviews.filter(function (r) { return r.topic === t.name; }).length;
    return '<div class="d-topic">' +
      '<button type="button" class="cd-task cd-task--btn" data-act="topic" data-id="' + t.id + '" data-w="' + c.wash + '"' +
        ' aria-expanded="' + (open ? 'true' : 'false') + '">' +
        '<span class="cd-lvl" data-v="' + lvl(t.status) + '" aria-hidden="true"><i></i><i></i><i></i></span>' +
        '<span>' +
          '<span class="cd-task__title">' + esc(t.name) + '</span>' +
          '<span class="cd-task__meta">' + idChip(c.id) +
            '<span>' + (t.lastSeen == null ? 'never self-tested' : 'last reviewed ' + ago(t.lastSeen)) + '</span>' +
            '<span>' + (reviewsOn ? reviewsOn + ' recall in the queue' : 'no recall queued') + '</span>' +
          '</span>' +
        '</span>' +
        '<span class="d-topic__end">' + statusChip(t.status) +
          '<span class="d-topic__chev">' + icon('chevron', 14) + '</span></span>' +
      '</button>' +
      (open ? topicDetail(t) : '') +
    '</div>';
  }
  function topicDetail(t) {
    var c = course(t.course);
    var dls = DEADLINES.filter(function (d) { return d.course === c.id && daysTo(d.due) >= 0; })
      .sort(function (a, b) { return a.due - b.due; });
    var deferred = state.deferred.indexOf(t.id) !== -1;
    var solid = topicsOf(c.id).filter(function (x) { return x.status === 'solid'; }).length;
    return '<div class="cd-detail" data-w="' + c.wash + '">' +
      '<div class="cd-detail__grid">' +
        '<div><div class="cd-detail__k">Mastery</div><div class="cd-detail__v">' + STATELBL[t.status] + '</div></div>' +
        '<div><div class="cd-detail__k">Last self-test</div><div class="cd-detail__v">' + (t.lastSeen == null ? '—' : ago(t.lastSeen)) + '</div></div>' +
        '<div><div class="cd-detail__k">Position</div><div class="cd-detail__v">Topic ' + t.seq + ' of ' + topicsOf(c.id).length + ' in ' + c.code + '</div></div>' +
        '<div><div class="cd-detail__k">Next deadline</div><div class="cd-detail__v">' + (dls[0] ? esc(dls[0].title) + ' · ' + countdown(daysTo(dls[0].due)) : 'none scheduled') + '</div></div>' +
        '<div><div class="cd-detail__k">Course so far</div><div class="cd-detail__v">' + solid + ' solid of ' + topicsOf(c.id).length + '</div></div>' +
      '</div>' +
      '<p class="cd-detail__note">The ladder only moves on evidence: a session logs time, a self-test moves the topic. ' +
        'This one sits at <b>' + STATELBL[t.status] + '</b>, which means ' +
        (t.status === 'new' ? 'it has never been tested — the first session puts it on the board.'
          : t.status === 'learning' ? 'you have met it once and it has not been recalled clean.'
          : t.status === 'proof' ? 'you can use it in a problem, but not yet from memory.'
          : 'you have recalled it clean from memory.') + '</p>' +
      '<div class="cd-detail__acts">' +
        playBtn('Start ' + state.target + ' min', t.id, 'topic') +
        (state.reviews.some(function (r) { return r.topic === t.name && !state.ratings[r.id]; })
          ? '<button type="button" class="cd-pill cd-pill--ghost cd-pill--sm" data-act="review-jump" data-id="' + t.id + '">Test it now</button>'
          : '') +
        '<button type="button" class="cd-pill cd-pill--quiet cd-pill--sm" data-act="' + (deferred ? 'reset-defer' : 'defer-topic') + '" data-id="' + t.id + '">' +
          (deferred ? 'Put it back in the running' : 'Set aside today') + '</button>' +
      '</div>' +
    '</div>';
  }
  function viewCourse(id) {
    var c = course(id);
    if (!c) return viewCourses();
    var ts = topicsOf(c.id);
    var by = { solid: 0, proof: 0, learning: 0, new: 0 };
    ts.forEach(function (t) { by[t.status]++; });
    var ex = daysTo(c.exam.on);
    var next = ts.filter(function (t) { return t.status !== 'solid'; })
      .sort(function (a, b) { return lvl(a.status) - lvl(b.status) || (b.lastSeen || 0) - (a.lastSeen || 0); })[0];
    return '<button type="button" class="d-back" data-act="go" data-href="#/courses">' + icon('chevron', 13) + 'All courses</button>' +
      pageHead(esc(c.title) + ' <span class="cd-hint">' + c.code + '</span>',
        esc(c.teacher) + ' · ' + esc(c.when) + ' · ' + ts.length + ' topics · ' + covOf(c.id) + '% covered',
        '<div class="d-examcount">' +
          '<div><div class="cd-hint">' + esc(c.exam.name) + '</div>' +
          '<div class="d-examcount__n">' + ex + '</div>' +
          '<div class="cd-hint">' + (ex === 0 ? 'today' : ex === 1 ? 'day' : 'days') + ' · ' + longDate(c.exam.on) + '</div></div>' +
        '</div>') +
    '<div class="cd-grid-2">' +
      '<div class="cd-stack">' +
        card('book', 'Topics', ts.length + ' in sequence · open one to see where it stands', '',
          '<div class="cd-tasks">' + ts.map(topicRow).join('') + '</div>') +
      '</div>' +
      '<div class="cd-rail">' +
        card('target', 'Coverage', 'A topic counts once opened', '', gauge(covOf(c.id), 'of ' + c.code) +
          '<div class="cd-detail__grid d-mt-lg">' +
            '<div><div class="cd-detail__k">Solid</div><div class="cd-detail__v">' + by.solid + '</div></div>' +
            '<div><div class="cd-detail__k">Proof</div><div class="cd-detail__v">' + by.proof + '</div></div>' +
            '<div><div class="cd-detail__k">Learning</div><div class="cd-detail__v">' + by.learning + '</div></div>' +
            '<div><div class="cd-detail__k">Untouched</div><div class="cd-detail__v">' + by['new'] + '</div></div>' +
          '</div>', c.wash) +
        (next ? card('arrow', 'Next in line', 'What the engine would pick here', '',
          '<div class="cd-task" data-w="' + c.wash + '">' +
            '<span class="cd-lvl" data-v="' + lvl(next.status) + '" aria-hidden="true"><i></i><i></i><i></i></span>' +
            '<span><span class="cd-task__title">' + esc(next.name) + '</span>' +
            '<span class="cd-task__meta">' + statusChip(next.status) + '<span>' + ago(next.lastSeen) + '</span></span></span>' +
            '<span class="cd-task__right">' + playBtn('Start', next.id, 'topic') + '</span>' +
          '</div>') : '') +
        card('clock', 'Logged here', c.code + ' in the last 14 days', '',
          statTile(hmOf(minutesFor(c.id, -13, 0)), 'on this course') , c.wash) +
      '</div>' +
    '</div>';
  }

  /* ─────────────────────────────────────────────────────────── 8 · REVIEW ──
     One question at a time, at the size of a question. Everything else on
     this screen is a count. */
  var ANSWERS = {
    r1: 'Chaining degrades to O(n) when every key lands in one bucket — that is Θ(n) per lookup. What keeps it O(1) expected is the load factor: resize (or rehash) when n/m passes a threshold, so the expected chain length stays a constant.',
    r2: 'rank(A) + nullity(A) = number of columns. Nullity is the dimension of the solution space of Ax = 0, so rank tells you how many of those columns are independent and nullity tells you how much freedom the solutions have. Ax = b is consistent exactly when rank(A) = rank([A|b]).',
    r3: 'Strong induction: assume P(j) for all j with j₀ ≤ j < k, then prove P(k) from that. It beats weak induction on anything that splits — e.g. every integer n > 1 is a product of primes, where n = ab needs P(a) and P(b), not just P(n − 1).',
    r4: 'Mutual exclusion, hold-and-wait, no preemption, circular wait. Break them by: making resources shareable or serialising access; requesting all resources up front; allowing the OS to take a resource back; imposing a global ordering on resource requests.',
    r5: 'A BST built from sorted input degenerates to a chain: height Θ(n), searches Θ(n). The fix is to rebalance as you insert — an AVL or red-black tree keeps height Θ(log n) with rotations.',
    r6: 'A is diagonalisable when it has n linearly independent eigenvectors — equivalently when each eigenvalue\'s geometric multiplicity equals its algebraic multiplicity. Counterexample: [[1,1],[0,1]] has the repeated eigenvalue 1 but only one independent eigenvector.',
    r7: 'If n items go into m containers and n > m, some container holds at least two. Non-obvious version: among any 5 points inside a 2×2 square, two are within √2 — four 1×1 quadrants, five points, pigeonhole.',
    r8: 'Paging splits memory into fixed-size frames, so it removes external fragmentation and the need to find a contiguous run. Segmentation splits it into variable-length logical units, so it matches how the program is written (code, stack, heap) at the cost of fitting those pieces somewhere.',
    r9: 'Doubling makes the expensive copies rare. Appending n elements costs 1 + 2 + 4 + … ≤ 2n total work, so the cost per append is constant even though a single append occasionally costs Θ(n).',
    r10: 'Diagonalisation is the cheap route: eigendecomposition or the characteristic polynomial split into factors. Hand computation is a last resort — for a 3×3, expand along a row with zeros.',
    r11: 'T(n) = 2T(n/2) + n = Θ(n log n) by the master theorem, case 2, since n^(log₂2) = n matches the work term. The recursion tree shows it too: log n levels, each costing n.',
    r12: 'A TLB miss costs a page-table walk — up to 4 memory reads on a 4-level table — before the access even begins. Locality matters because a TLB is tiny: touching the same pages repeatedly keeps the translations hot.'
  };

  function currentReview() {
    var left = dueRecalls();
    return left.length ? left[0] : null;
  }
  function reviewQueue() {
    return state.reviews.slice().sort(function (a, b) {
      return (state.ratings[a.id] ? 1 : 0) - (state.ratings[b.id] ? 1 : 0) || b.lastSeen - a.lastSeen;
    });
  }
  function viewReview() {
    var left = dueRecalls();
    var done = state.reviews.length - left.length;
    var r = currentReview();
    var never = state.reviews.filter(function (x) { return x.lastSeen >= 8; }).length;
    var mins = Math.max(1, Math.round(left.length * 1.5));

    if (!state.reviewsStarted && left.length) {
      return pageHead('Review', left.length + ' recalls waiting · about ' + mins + ' minutes · ' +
        never + ' are over a week old',
        '<button type="button" class="cd-pill" data-act="start-review">' + icon('review', 14) + 'Start reviewing</button>') +
      card('review', 'The queue', 'Oldest first — that is the only ordering rule', '',
        '<div class="cd-tasks">' + reviewQueue().map(function (x) {
          var c = course(x.course);
          return '<div class="cd-task" data-w="' + c.wash + '">' +
            '<span class="cd-chip cd-chip--code">' + c.code + '</span>' +
            '<div class="cd-task__title">' + esc(x.topic) + '</div>' +
            '<div class="cd-task__right"><span class="cd-hint">' + ago(x.lastSeen) + '</span></div>' +
          '</div>';
        }).join('') + '</div>') +
      '<div class="d-note">' + icon('info', 14) + '<span>Answer out loud before you reveal. A recall you get wrong ' +
      'comes back tomorrow; one you get right comes back later. Nothing is scored, and nothing is ever marked wrong permanently.</span></div>';
    }

    if (!r) {
      var graded = state.reviews.length;
      var solid = state.ratings ? Object.keys(state.ratings).filter(function (k) { return state.ratings[k] === 4; }).length : 0;
      return '<div class="d-recall-wrap">' +
        '<div class="cd-complete">' +
          '<div class="cd-complete__spark">' +
            '<span class="cd-sparkle">' + icon('sparkle', 24) + '</span>' +
            '<span class="cd-sparkle">' + icon('sparkle', 30) + '</span>' +
            '<span class="cd-sparkle">' + icon('sparkle', 24) + '</span>' +
          '</div>' +
          '<div class="cd-complete__t">That is the queue clear.</div>' +
          '<div class="cd-complete__s">Here is exactly what just happened, counted.</div>' +
          '<div class="cd-complete__ledger">' +
            '<div class="cd-complete__cell"><div class="cd-complete__v">' + graded + '</div><div class="cd-complete__k">recalls answered</div></div>' +
            '<div class="cd-complete__cell"><div class="cd-complete__v">' + solid + '</div><div class="cd-complete__k">recalled clean</div></div>' +
            '<div class="cd-complete__cell"><div class="cd-complete__v">' + state.redo.length + '</div><div class="cd-complete__k">back tomorrow</div></div>' +
            '<div class="cd-complete__cell"><div class="cd-complete__v">' + state.deferred.length + '</div><div class="cd-complete__k">set aside today</div></div>' +
          '</div>' +
          '<button type="button" class="cd-pill cd-pill--lg" data-act="go" data-href="#/today">Back to today</button>' +
        '</div>' +
      '</div>';
    }

    var c = course(r.course);
    var idx = done + 1;
    return '<div class="d-recall-wrap">' +
      '<div class="d-spread d-mb-md">' +
        '<span class="cd-hint">Recall ' + idx + ' of ' + state.reviews.length + '</span>' +
        '<span class="cd-hint">' + left.length + ' left · about ' + mins + ' min</span>' +
      '</div>' +
      meter(done, state.reviews.length, { label: done + '/' + state.reviews.length }) +
      '<section class="cd-recall d-mt-md" data-w="' + c.wash + '">' +
        '<div class="cd-focus__eyebrow">' + idChip(c.id) +
          '<span class="cd-chip cd-chip--onwash">' + esc(r.topic) + '</span>' +
          '<span class="cd-chip">last seen ' + ago(r.lastSeen) + '</span>' +
        '</div>' +
        '<h2 class="cd-recall__prompt">' + esc(r.prompt) + '</h2>' +
        (state.revealed
          ? '<div class="cd-recall__answer">' + esc(ANSWERS[r.id] || '') + '</div>' +
            '<div class="cd-rate">' +
              rateBtn(1, 'Forgot', 'back tomorrow') +
              rateBtn(2, 'Hard', 'stays learning') +
              rateBtn(3, 'Good', 'moves to proof') +
              rateBtn(4, 'Solid', 'no recall for 2 weeks') +
            '</div>'
          : '<div class="d-reveal">' +
              '<span class="cd-hint">Say it out loud, or write it down, before you look.</span>' +
              '<button type="button" class="cd-pill cd-pill--lg" data-act="reveal">' + icon('check', 14) + 'Reveal the answer</button>' +
              '<span class="cd-hint">Space</span>' +
            '</div>') +
      '</section>' +
      '<div class="d-note">' + icon('info', 14) + '<span>Grades are a judgment about memory, not about you — and only a grade moves a topic up the ladder. Keys 1–4 grade; Space reveals.</span></div>' +
      '</div>';
  }
  function rateBtn(n, label, sub) {
    return '<button type="button" class="cd-rate__btn" data-act="rate" data-v="' + n + '">' +
      '<b>' + n + ' · ' + label + '</b><span>' + sub + '</span></button>';
  }

  /* ───────────────────────────────────────────────────────── 9 · PROGRESS ──
     Only arithmetic the app can defend: sums of the session log and counts
     of the queue. No scores, no grades, no projections. */
  function sessionCount(from, to) {
    return SESSIONS.filter(function (s) { return s.day >= from && s.day <= to; }).length;
  }
  function viewProgress() {
    var range = parseInt(state.progressRange, 10);
    var days = chartDays(range, 'date');
    var mins = totalMinutes(-(range - 1), 0);
    var worked = days.filter(function (x) { return x.minutes > 0; }).length;
    var answered = Object.keys(state.ratings).length;
    var graded = state.reviews.length;
    var solid = state.ratings ? Object.keys(state.ratings).filter(function (k) { return state.ratings[k] === 4; }).length : 0;

    var seg = '<div class="cd-seg" role="group" aria-label="History window">' +
      [['7', '7 days'], ['14', '14 days'], ['30', '30 days']].map(function (o) {
        return '<button type="button" class="cd-seg__pill" data-act="prog-range" data-v="' + o[0] + '"' +
          ' aria-pressed="' + (state.progressRange === o[0]) + '">' + o[1] + '</button>';
      }).join('') + '</div>';

    var byCourse = COURSES.map(function (c) {
      var m = minutesFor(c.id, -(range - 1), 0);
      var total = Math.max(1, COURSES.reduce(function (a, x) { return a + minutesFor(x.id, -(range - 1), 0); }, 0));
      return '<div class="d-progrow" data-w="' + c.wash + '">' +
        '<span class="d-progrow__l">' + c.code + '</span>' +
        meter(m, total, { head: true, label: false, lg: true, wash: true }) +
        '<span class="cd-hint num" style="text-align:right">' + hmOf(m) + '</span>' +
      '</div>';
    }).join('');

    var health = [['Untouched', totalTopics() - coveredAll(), 'info'],
      ['Learning', topics.filter(function (t) { return t.status === 'learning'; }).length, 'risk'],
      ['Proof', topics.filter(function (t) { return t.status === 'proof'; }).length, 'ok'],
      ['Solid', solidAll(), 'ok']];

    return pageHead('Progress', 'Last ' + range + ' days · ' + hmOf(mins) + ' logged · ' +
      worked + '/' + range + ' days have work on them', seg) +
    '<div class="cd-stack">' +
      '<div class="cd-stats">' +
        statTile(hmOf(mins), 'logged in ' + range + ' days') +
        statTile(sessionCount(-(range - 1), 0), 'sessions finished') +
        statTile(worked + '/' + range, 'days with work') +
        statTile(answered + '/' + graded, 'recalls answered') +
      '</div>' +
      card('progress', 'Work per day', 'Each bar is one day of logged minutes', '',
        stemChart(days) +
        '<div class="d-note">' + icon('info', 14) + '<span>The dashed ceiling is a full 90-minute day. ' +
          zeroDays(-(range - 1), 0) + ' of these ' + range + ' days have nothing logged — the app reports that as a count, never as a broken streak.</span></div>') +
      '<div class="cd-grid-2">' +
        card('courses', 'Where the time went', 'Share of logged minutes', '', byCourse) +
        card('review', 'Recall health', solid + ' of ' + totalTopics() + ' topics are solid', '',
          '<div class="d-health">' + health.map(function (h) {
            return '<div><div class="cd-stat__v">' + h[1] + '</div><div class="cd-stat__l">' + h[0] + '</div>' +
              '<span class="cd-chip d-mt-xs cd-chip--' + h[2] + '">' + Math.round(h[1] / totalTopics() * 100) + '% of the course load</span></div>';
          }).join('') + '</div>' +
          '<div class="d-note">' + icon('info', 14) + '<span>A topic only reaches solid after a clean recall. Nothing rises because time passed or because a session ran long.</span></div>') +
      '</div>' +
    '</div>';
  }

  /* ─────────────────────────────────────────────────────────── 10 · SYSTEM ──
     The specification page renders the real components. If this page drifts
     from the app, the app is what is wrong. */
  function swatch(name, token, wash) {
    return '<div class="d-swatch">' +
      '<div class="d-swatch__chip"' + (wash ? ' data-w="' + wash + '"' : '') + ' style="background:' + token + '"></div>' +
      '<div class="d-swatch__meta"><div class="d-swatch__n">' + name + '</div><div class="d-swatch__v">' + token + '</div></div>' +
    '</div>';
  }
  function viewSystem() {
    var typeScale = [
      ['52px display', 'var(--text-display)', 'Page hero, once per screen'],
      ['32px', 'var(--text-2xl)', 'The decision statement'],
      ['26px', 'var(--text-xl)', 'Recall prompt, gauge value'],
      ['20px', 'var(--text-lg)', 'Card titles in tiles'],
      ['17px', 'var(--text-md)', 'Card titles'],
      ['15px', 'var(--text-base)', 'Body and row titles'],
      ['13px', 'var(--text-xs) / var(--text-sm)', 'Meta, chips, sub-lines'],
      ['11px', 'var(--text-2xs)', 'Only when the row is dense']
    ].map(function (t) {
      return '<div><small>' + t[0] + '</small><span style="font-size:' + t[1] + ';font-weight:var(--weight-display);letter-spacing:var(--track-title)">Cadence</span>' +
        '<span class="cd-hint">' + t[2] + '</span></div>';
    }).join('');

    return pageHead('Design system', 'Warm Workbench · every colour, radius and duration the app is allowed to use',
      '<button type="button" class="cd-pill cd-pill--ghost" data-act="toast-demo">' + icon('info', 14) + 'Test a toast</button>') +
    '<div class="cd-stack">' +
      card('layers', 'Surfaces', 'A quiet room, a cool sheet, white objects on it', '',
        '<div class="d-swatches">' +
          swatch('Backdrop', 'var(--backdrop)') +
          swatch('Sheet', 'var(--sheet)') +
          swatch('Well', 'var(--well)') +
          swatch('Card', 'var(--card)') +
          swatch('Ink', 'var(--ink)') +
          swatch('Ink 2', 'var(--ink-2)') +
          swatch('Ink 3 · text floor', 'var(--ink-3)') +
          swatch('Ink 4 · non-text', 'var(--ink-4)') +
        '</div>') +
      card('target', 'Identity', 'Four washes, each with its own deep ink. A course owns one and fills with it', '',
        '<div class="d-swatches">' +
          ['mint', 'lilac', 'butter', 'sky'].map(function (w) {
            return '<div class="d-swatch" data-w="' + w + '">' +
              '<div class="d-swatch__chip" style="background:var(--wash)"></div>' +
              '<div class="d-swatch__meta"><div class="d-swatch__n">' + w + '</div>' +
              '<div class="d-swatch__v" style="color:var(--onwash);font-weight:var(--weight-display)">deep ink on wash</div></div></div>';
          }).join('') +
        '</div>' +
        '<div class="cd-cluster d-mt-lg">' +
          /* The deadline ladder, in words: past due, inside 48 hours, and the
             rest. It is the one place a due date is spoken, so it is the one
             place the mapping is visible. */
          [['overdue', '2 days late'], ['risk', 'today'], ['risk', 'tomorrow'], ['ok', 'in 6 days'], ['info', 'recall']].map(function (s) {
            return '<span class="cd-chip cd-chip--' + s[0] + '">' + s[1] + '</span>';
          }).join('') +
          '<span class="cd-chip cd-chip--wash" data-w="butter">wash chip</span>' +
          '<span class="cd-chip cd-chip--outline">outline is for quiet metadata only</span>' +
        '</div>') +
      card('info', 'Type', 'Plus Jakarta Sans variable, 200–800. Embedded, not linked', '',
        '<div class="d-typespec">' + typeScale + '</div>' +
        '<div class="d-note">' + icon('info', 14) + '<span>Numbers are tabular everywhere they line up in a column. Display sizes take negative tracking; body text does not.</span></div>') +
      card('layers', 'Geometry and depth', 'One radius family, five shadows', '',
        '<div class="d-spec">' +
          ['card and window 24 · tile 18 · disc 16 · item 14 · mini 12 · mark 9 · key 6 · pill 999',
           'a chip is a pill, not a box — and the field is its contents plus the padding around them',
           'shadow 1 (a row or tile on a card) · shadow 2 (a card on the sheet) · shadow 3 (the window) · pop (an overlay) · ink (a pressed dark pill)']
            .map(function (t, i) {
              return '<div class="d-spec__row"><span class="d-spec__k">' + (i === 0 ? 'corners' : i === 1 ? 'containers' : 'depth') + '</span>' +
                '<span class="cd-hint">' + t + '</span></div>';
            }).join('') +
        '</div>' +
        '<div class="cd-cluster d-mt-lg">' +
          '<span style="width:64px;height:42px;border-radius:var(--r-card);background:var(--well);box-shadow:var(--sh-2);display:inline-block"></span>' +
          '<span style="width:64px;height:42px;border-radius:var(--r-tile);background:var(--well);box-shadow:var(--sh-1);display:inline-block"></span>' +
          '<span style="width:64px;height:42px;border-radius:var(--r-disc);background:var(--well);display:inline-block"></span>' +
          '<span style="width:64px;height:42px;border-radius:var(--r-item);background:var(--w-butter);display:inline-block"></span>' +
          '<span style="width:64px;height:24px;border-radius:var(--r-pill);background:var(--ink);display:inline-block"></span>' +
        '</div>') +
      card('check', 'Controls', 'One dark pill per screen: that is the whole emphasis budget', '',
        '<div class="cd-cluster">' +
          '<button type="button" class="cd-pill">Primary</button>' +
          '<button type="button" class="cd-pill cd-pill--ghost">Secondary</button>' +
          '<button type="button" class="cd-pill cd-pill--quiet">Quiet</button>' +
          '<button type="button" class="cd-pill cd-pill--sm">Small</button>' +
          durCapsule() +            /* fused; open it and it splits into its own parts */
          '<button type="button" class="cd-iconbtn is-active">' + icon('today', 16) + '</button>' +
          '<button type="button" class="cd-iconbtn">' + icon('moon', 16) + '</button>' +
          '<span class="cd-round">' + icon('arrow', 14) + '</span>' +
          '<span class="cd-kbd">K</span>' +
        '</div>' +
        '<div class="cd-cluster d-mt-lg">' +
          '<span class="cd-seg"><button type="button" class="cd-seg__pill" aria-pressed="true">Pressed</button>' +
          '<button type="button" class="cd-seg__pill" aria-pressed="false">Resting</button></span>' +
          '<span class="cd-chip cd-chip--code" data-w="mint">CS201</span>' +
          '<span class="cd-check" aria-pressed="false">' + icon('check', 13) + '</span>' +
          '<span class="cd-check" aria-pressed="true" style="background:var(--fg-mint);color:var(--card)">' + icon('check', 13) + '</span>' +
          '<span class="cd-datetile">24</span>' +
        '</div>') +
      card('target', 'Instruments', 'One mark per job: identity, amount, mastery, time', '',
        '<div class="cd-stack cd-stack--tight">' +
          '<div class="d-spec__row"><span class="d-spec__k">meter, hatched headroom</span><div data-w="mint">' + meter(7, 12, { head: true, label: '7/12' }) + '</div></div>' +
          '<div class="d-spec__row"><span class="d-spec__k">meter, share</span><div data-w="sky">' + meter(42, 100, { head: true, label: '42%', wash: true }) + '</div></div>' +
          '<div class="d-spec__row"><span class="d-spec__k">mastery ladder</span><div class="cd-cluster">' +
            '<span class="cd-lvl" data-v="0"><i></i><i></i><i></i></span>' +
            '<span class="cd-lvl" data-v="1"><i></i><i></i><i></i></span>' +
            '<span class="cd-lvl" data-v="2"><i></i><i></i><i></i></span>' +
            '<span class="cd-lvl" data-v="3"><i></i><i></i><i></i></span>' +
            '<span class="cd-hint">not started · learning · proof · solid</span></div></div>' +
          '<div class="d-spec__row"><span class="d-spec__k">coverage gauge</span><div data-w="lilac">' + gauge(62, 'of MA210') + '</div></div>' +
          '<div class="d-spec__row"><span class="d-spec__k">day strip</span>' + dayStrip() + '</div>' +
        '</div>') +
      card('info', 'States', 'Empty, loading and set-aside are all screens with words. The skeleton is the one grey thing allowed, and it is the shape of what is coming', '',
        '<div class="cd-duo">' +
          '<div class="cd-empty">' + '<span class="cd-empty__art">' + icon('check', 34) + '</span>' +
            '<div class="cd-empty__t">Queue empty</div>' +
            '<div class="cd-empty__s">Nothing is scheduled. Recalls and deadlines come back on their own.</div>' +
            '<button type="button" class="cd-pill cd-pill--ghost cd-pill--sm">Add something anyway</button></div>' +
          '<div class="cd-dashed"><b>3 set aside for today</b><span>Still there. Nothing is deleted by hiding it.</span>' +
            '<button type="button" class="cd-pill cd-pill--ghost cd-pill--sm">Bring them back</button></div>' +
        '</div>' +
        /* The loading state, in all three shapes the contract names: rows at
           the 76px grid of `.cd-task`, a card at a course tile's 193px, and
           the 132px plot. Each borrows the arriving height, so nothing moves
           vertically when the content lands. */
        '<div class="d-mt-lg" aria-busy="true">' +
          '<span class="cd-sr">Loading the queue, the course deck and the chart</span>' +
          '<div class="cd-skel__rows" aria-hidden="true">' +
            [58, 44, 51].map(function (w) {
              return '<div class="cd-skel__row"><i class="cd-skel"></i>' +
                '<i class="cd-skel" style="--w:' + w + '%"></i>' +
                '<i class="cd-skel"></i></div>';
            }).join('') +
          '</div>' +
          '<div class="cd-duo d-mt-lg" aria-hidden="true">' +
            '<div class="cd-skel cd-skel--card">' +
              '<i class="cd-skel" style="--w:38%"></i>' +
              '<i class="cd-skel" style="--w:78%"></i>' +
              '<i class="cd-skel" style="--w:64%"></i>' +
            '</div>' +
            '<div class="cd-skel__plot">' +
              [52, 78, 34, 61, 88, 45, 70].map(function (v) {
                return '<i class="cd-skel" style="--v:' + v + '%"></i>';
              }).join('') +
            '</div>' +
          '</div>' +
        '</div>') +
      card('review', 'Keyboard', 'Every flow finishes without a pointer', '',
        '<div class="d-spec">' +
          [[modKey() + ' or /', 'Open the console — go anywhere, start anything, add a task'],
           ['1 – 5', 'Jump between Today, Plan, Courses, Review, Progress'],
           ['a', 'New task'],
           ['Space', 'Reveal the answer in a recall'],
           ['1 – 4', 'Grade the recall you just revealed'],
           ['Esc', 'Close the console, or the open disclosure']]
            .map(function (k) {
              return '<div class="d-spec__row"><span class="d-spec__k"><span class="cd-kbd">' + k[0] + '</span></span>' +
                '<span class="cd-hint">' + k[1] + '</span></div>';
            }).join('') +
        '</div>') +
    '</div>';
  }

  /* ─────────────────────────────────────────────── 11 · SHELL & CONSOLE ──
     Chrome behaviour: the toast, the timer, the rail, the keyboard console and the
     one floating layer. Still no framework, still no dependencies. */
  var TOAST_MAX = 3;   // more than three at once is a wall, not a notice
  function toast(msg, ic) {
    var host = $('#toasts');
    while (host.children.length >= TOAST_MAX) host.removeChild(host.firstChild);
    var t = document.createElement('div');
    t.className = 'cd-toast';
    t.innerHTML = icon(ic || 'check', 14) + '<span>' + esc(msg) + '</span>';
    host.appendChild(t);
    setTimeout(function () {
      t.style.transition = 'opacity var(--dur-2) var(--ease), transform var(--dur-2) var(--ease)';
      t.style.opacity = '0';
      t.style.transform = 'translateY(6px)';
      setTimeout(function () { t.remove(); }, 240);
    }, 2600);
  }
  function navigate(href) {
    if (location.hash === href) { closeLayer(); render(); }
    else location.hash = href;
  }
  function labelFor(kind, id) {
    if (kind === 'topic') { var t = topic(id); return t ? t.name : 'Session'; }
    if (kind === 'task') { var q = allTasks().filter(function (x) { return x.id === id; })[0]; return q ? q.title : 'Task'; }
    return 'Reviewing recalls';
  }
  function startFocus(kind, id) {
    if (state.focus) stopFocus(false);
    state.focus = {
      /* The length is a snapshot: a running session keeps the agreement it
         started with, so editing the capsule changes the next one. */
      kind: kind, id: id, seconds: 0, target: state.target,
      label: labelFor(kind, id),
      course: kind === 'topic' ? (topic(id) || {}).course : kind === 'task' ? (allTasks().filter(function (x) { return x.id === id; })[0] || {}).course : null,
      handle: null
    };
    state.focus.handle = setInterval(function () {
      if (!state.focus) return;
      state.focus.seconds++;
      var el = $('.cd-timer__time');
      if (el) el.textContent = mmss(state.focus.seconds);
      if (state.focus.seconds === state.focus.target * 60) toast(state.focus.target + ' minutes on ' + state.focus.label + ' — keep going or stop', 'clock');
    }, 1000);
    renderTimer();
    toast('Session started · ' + state.focus.label, 'clock');
  }
  function mmss(s) {
    var m = Math.floor(s / 60), r = s % 60;
    return m + ':' + String(r).padStart(2, '0');
  }
  /* Only real time is logged, and only above a minute. A session that lasted
     forty seconds is not knowledge and is not counted as any. */
  function stopFocus(silent) {
    if (!state.focus) return;
    var f = state.focus;
    clearInterval(f.handle);
    state.focus = null;
    var mins = Math.round(f.seconds / 60);
    if (mins >= 1) {
      SESSIONS.push({ day: 0, course: f.course, minutes: mins, topic: f.label });
      if (!silent) toast('Logged ' + mins + ' min on ' + f.label, 'check');
    } else if (!silent) {
      toast('Under a minute — nothing logged', 'info');
    }
    renderTimer();
    if (state.route.name === 'progress' || state.route.name === 'today') render();
  }
  function renderTimer() {
    var host = $('#timerHost');
    if (!state.focus) { host.innerHTML = ''; return; }
    host.innerHTML = '<span class="cd-timer">' +
      '<span class="cd-timer__label">' + esc(state.focus.label) + '</span>' +
      '<span class="cd-timer__time num">' + mmss(state.focus.seconds) + '</span>' +
      '<span class="cd-hint">of ' + mmss(state.focus.target * 60) + '</span>' +
      '<button type="button" class="cd-pill cd-pill--sm" data-act="stop-focus">' + icon('stop', 12) + 'Stop</button>' +
    '</span>';
  }

  var NAV = [
    { name: 'today', href: '#/today', icon: 'today', label: 'Today' },
    { name: 'plan', href: '#/plan', icon: 'plan', label: 'Plan' },
    { name: 'courses', href: '#/courses', icon: 'courses', label: 'Courses' },
    { name: 'review', href: '#/review', icon: 'review', label: 'Review' },
    { name: 'progress', href: '#/progress', icon: 'progress', label: 'Progress' }
  ];
  function navCount(name) {
    /* Only Review carries a number. A count on every destination is a
       dashboard shouting in the chrome; the one number worth interrupting
       for is the one that says memory is expiring. Everything else the
       page says for itself the moment you look at it. */
    return name === 'review' ? dueRecalls().length : 0;
  }
  /* The rail: one glass pill nesting a column of discs, one solid ink disc
     for where you are, the date at the top and the two chrome acts pinned to
     the bottom. Each disc nests its glyph in a fixed box and its name in a
     label that is only as wide as --rail-label while the pointer is on it. */
  function renderNav() {
    var here = state.route.name === 'course' ? 'courses' : state.route.name;
    var now = new Date();
    var discs = NAV.map(function (n) {
      var c = navCount(n.name);
      var dated = n.name === 'today';
      var says = n.label + (c ? ' · ' + c + ' waiting' : '');
      return '<a class="cd-disc' + (dated ? ' cd-disc--date' : '') + '" href="' + n.href + '"' +
        ' data-act="go" data-href="' + n.href + '"' +
        ' title="' + esc(says) + '" aria-label="' + esc(says) + '"' +
        (here === n.name ? ' aria-current="page"' : '') + '>' +
        '<span class="cd-disc__ico">' +
        (dated
          ? '<b class="num">' + now.getDate() + '</b><i>' + WD[now.getDay()] + '</i>'
          : icon(n.icon, 20)) +
        (c ? '<span class="cd-disc__n">' + c + '</span>' : '') +
        '</span>' +
        '<span class="cd-disc__label">' + esc(n.label) + '</span>' +
        '</a>';
    }).join('');
    /* The two chrome acts never carry the screen's one dark pill — that
       belongs to the work, not to the furniture. */
    $('#navRail').innerHTML = discs +
      '<span class="cd-nav__spacer"></span>' +
      '<button type="button" class="cd-disc" data-act="quick-add"' +
        ' title="Add to today · A" aria-label="Add to today">' +
        '<span class="cd-disc__ico">' + icon('plus', 18) + '</span>' +
        '<span class="cd-disc__label">Add</span></button>' +
      '<span class="cd-nav__sep" aria-hidden="true"></span>' +
      '<button type="button" class="cd-disc" data-act="theme"' +
        ' title="Switch appearance" aria-label="Switch appearance">' +
        '<span class="cd-disc__ico">' + icon(state.theme === 'dark' ? 'sun' : 'moon', 18) + '</span>' +
        '<span class="cd-disc__label">Theme</span></button>';
  }
  function syncChrome() {
    $('#termMeta').textContent = 'Week ' + TERM.week + ' of ' + TERM.weeks;
  }

  /* ── The console: one layer, two jobs (find, capture). ────────────────── */
  function paletteItems(q) {
    var query = q.trim().toLowerCase();
    var out = [];
    if (state.layer === 'add') {
      var text = q.trim();
      return [{ ic: 'plus', label: text ? 'Add "' + text + '" to today' : 'Type it, then press Enter',
        sub: text ? 'Goes to the top of the queue · Enter' : 'Anything counts — a chapter, a problem set, a fear',
        run: function () { addTask(text); } }];
    }
    if (!query) {
      out.push({ group: 'Go to' });
      NAV.forEach(function (n, i) {
        out.push({ ic: n.icon, label: n.label, sub: navCount(n.name) ? navCount(n.name) + ' waiting' : 'nothing waiting',
          kbd: String(i + 1), run: function () { navigate(n.href); } });
      });
      out.push({ ic: 'layers', label: 'Design system', sub: 'Every token and component', run: function () { navigate('#/system'); } });
      out.push({ group: 'Do' });
      var nxt = pickNext();
      if (nxt) out.push({ ic: 'clock', label: 'Start a session on ' + nxt.title, sub: course(nxt.course).code + ' · ' + nxt.minutes + ' min',
        wash: course(nxt.course).wash, run: function () { navigate('#/today'); startFocus('topic', nxt.topicId); } });
      if (dueRecalls().length) out.push({ ic: 'review', label: 'Review ' + dueRecalls().length + ' recalls', sub: 'Oldest first',
        run: function () { state.reviewsStarted = true; navigate('#/review'); } });
      out.push({ ic: 'plus', label: 'Add a task', sub: 'Type it, then Enter', run: function () { openLayer('add'); } });
      out.push({ ic: state.theme === 'dark' ? 'sun' : 'moon', label: 'Switch appearance', sub: 'Now: ' + state.theme,
        run: function () { toggleTheme(); } });
      return out;
    }
    out.push({ group: 'Looks like' });
    out.push({ ic: 'plus', label: 'Add "' + q.trim() + '" as a task', sub: 'Goes to the top of today', run: function () { addTask(q.trim()); } });
    var courses = COURSES.filter(function (c) {
      return (c.code + ' ' + c.title).toLowerCase().indexOf(query) !== -1;
    });
    if (courses.length) {
      out.push({ group: 'Courses' });
      courses.forEach(function (c) {
        out.push({ ic: 'courses', label: c.code + ' · ' + c.title, sub: covered(c.id) + ' of ' + topicsOf(c.id).length + ' topics covered',
          wash: c.wash, run: function () { navigate('#/course/' + c.id); } });
      });
    }
    var tops = topics.filter(function (t) { return t.name.toLowerCase().indexOf(query) !== -1; }).slice(0, 6);
    if (tops.length) {
      out.push({ group: 'Topics' });
      tops.forEach(function (t) {
        out.push({ ic: 'book', label: t.name, sub: course(t.course).code + ' · ' + STATELBL[t.status],
          wash: course(t.course).wash, run: function () { navigate('#/course/' + t.course); state.pendingTopic = t.id; } });
      });
    }
    var tasks = allTasks().filter(function (x) { return x.title.toLowerCase().indexOf(query) !== -1; }).slice(0, 5);
    if (tasks.length) {
      out.push({ group: 'Tasks' });
      tasks.forEach(function (x) {
        out.push({ ic: state.queue[x.id] === 'done' ? 'check' : 'calendar-day', label: x.title, sub: state.queue[x.id] === 'done' ? 'done' : 'open',
          run: function () { navigate('#/today'); } });
      });
    }
    return out;
  }
  function openLayer(kind) {
    state.layer = kind;
    state.q = '';
    state.sel = 0;
    renderLayer();
  }
  function closeLayer() {
    state.layer = null;
    state.q = '';
    $('#layer').innerHTML = '';
  }
  /* The shell is built once per open and never again: re-writing the input on
     every keystroke would take the caret with it, and a search field that
     drops characters is a search field nobody types in. Only the list re-renders. */
  function layerShell() {
    return '<div class="cd-layer" data-act="layer-close">' +
      '<div class="cd-palette" role="dialog" aria-modal="true" aria-label="' + (state.layer === 'add' ? 'Add a task' : 'Console') + '">' +
        '<div class="cd-palette__field">' + icon(state.layer === 'add' ? 'plus' : 'search', 17) +
          '<input id="palInput" type="text" placeholder="' + (state.layer === 'add' ? 'What needs doing?' : 'Search or jump to') +
            '" autocomplete="off" spellcheck="false" aria-label="' + (state.layer === 'add' ? 'Task title' : 'Search') + '" />' +
          '<span class="cd-kbd">Esc</span>' +
        '</div>' +
        '<div class="cd-palette__list cd-scroll" id="palList"></div>' +
      '</div>' +
    '</div>';
  }
  function renderList() {
    var list = $('#palList');
    if (!list) return;
    var items = paletteItems(state.q);
    var body = '';
    var sel = 0;
    items.forEach(function (it) {
      if (it.group) { body += '<div class="cd-palette__group">' + esc(it.group) + '</div>'; return; }
      var idx = sel++;
      body += '<button type="button" class="cd-palette__item"' + (idx === state.sel ? ' data-sel="1"' : '') + ' data-pal="' + idx + '">' +
        '<span class="cd-ictile"' + (it.wash ? ' data-w="' + it.wash + '"' : '') + '>' + icon(it.ic, 15) + '</span>' +
        '<span class="cd-palette__label">' + esc(it.label) + '</span>' +
        (it.kbd ? '<span class="cd-kbd">' + it.kbd + '</span>' : '') +
        (it.sub ? '<span class="cd-hint">' + esc(it.sub) + '</span>' : '') +
      '</button>';
    });
    if (!body) body = '<div class="cd-palette__empty">Nothing matches "' + esc(state.q) + '". Press Enter to add it as a task.</div>';
    list.innerHTML = body;
    var on = list.querySelector('[data-sel]');
    if (on && on.scrollIntoView) on.scrollIntoView({ block: 'nearest' });
  }
  function renderLayer() {
    if (!state.layer) return;
    if ($('#palList')) { renderList(); return; }
    $('#layer').innerHTML = layerShell();
    var inp = $('#palInput');
    if (inp) { inp.value = state.q; inp.focus(); inp.select(); }
    renderList();
  }

  function palItemAt(i) { return paletteItems(state.q).filter(function (x) { return !x.group; })[i]; }
  function moveSel(d) {
    var n = paletteItems(state.q).filter(function (x) { return !x.group; }).length;
    if (!n) return;
    state.sel = (state.sel + d + n) % n;
    renderLayer();
  }
  function runSel() {
    if (state.layer === 'add') { addTask(state.q || $('#palInput').value); return; }
    var it = palItemAt(state.sel);
    if (it) { closeLayer(); it.run(); }
  }
  function addTask(text) {
    text = (text || '').trim();
    if (!text) { closeLayer(); return; }
    state.extra.push({ id: 'x' + state.extra.length + 1, title: text, course: null, due: null });
    closeLayer();
    toast('Added to today', 'plus');
    if (state.route.name === 'today') render(); else navigate('#/today');
  }
  function toggleTheme() {
    state.theme = state.theme === 'dark' ? 'light' : 'dark';
    applyTheme();
    toast(state.theme === 'dark' ? 'Dark room' : 'Daylight', state.theme === 'dark' ? 'moon' : 'sun');
    render();
  }
  /* The appearance is one attribute on <html>; the rail re-renders the glyph
     from it. Nothing else touches the theme, so the two can never disagree.
     This is also the only writer of the choice: `index.html` reads the same key
     in <head> before the first paint, so a preference outlives the window
     instead of resetting to the OS on every launch. The write is best-effort —
     a shell with storage blocked still switches, it just does not remember. */
  function applyTheme() {
    document.documentElement.setAttribute('data-theme', state.theme);
    try { localStorage.setItem('cadence.theme', state.theme); } catch (e) { /* storage blocked */ }
  }
  /* Grading is a judgment about memory, and only a grade moves the ladder. */
  function grade(value) {
    var r = currentReview();
    if (!r) return;
    var t = topics.filter(function (x) { return x.name === r.topic; })[0];
    state.ratings[r.id] = value;
    if (value === 1) {
      if (state.redo.indexOf(r.id) === -1) state.redo.push(r.id);
      if (t) { t.status = 'learning'; t.lastSeen = 0; }
      toast('Back tomorrow — that one is worth another look', 'review');
    } else {
      if (t) {
        t.status = value === 2 ? 'learning' : value === 3 ? 'proof' : 'solid';
        t.lastSeen = 0;
      }
      toast(value === 4 ? 'Solid. No recall for two weeks.' : value === 3 ? 'Proof — one more clean recall makes it solid.' : 'Still learning — it comes back soon.', 'check');
    }
    state.revealed = false;
    render();
  }

  /* ───────────────────────────────────────────────────── 12 · BEHAVIOUR ──
     One delegated click handler and one delegated key handler. New markup is
     wired by declaring data-act, never by attaching listeners at render time. */
  function onClick(e) {
    var pal = e.target.closest ? e.target.closest('[data-pal]') : null;
    if (pal) { state.sel = parseInt(pal.getAttribute('data-pal'), 10); runSel(); return; }
    var hit = e.target.closest ? e.target.closest('[data-act]') : null;
    if (!hit) return;
    var act = hit.getAttribute('data-act');
    var id = hit.getAttribute('data-id');

    switch (act) {
      case 'go':
        e.preventDefault();
        closeLayer();
        navigate(hit.getAttribute('data-href'));
        return;
      case 'pal': openLayer('find'); return;
      case 'quick-add': openLayer('add'); return;
      case 'layer-close':
        if (e.target === hit) closeLayer();
        return;
      case 'theme': toggleTheme(); return;
      case 'open-course': navigate('#/course/' + id); return;
      case 'topic': {
        state.open[id] = !state.open[id];
        var keep = window.scrollY;
        render();
        var node = $('[data-act="topic"][data-id="' + id + '"]');
        if (node && state.open[id]) node.focus({ preventScroll: true });
        window.scrollTo(0, keep);
        return;
      }
      case 'defer-topic':
        if (state.deferred.indexOf(id) === -1) state.deferred.push(id);
        toast('Set aside for today — still on the board', 'clock');
        render();
        return;
      case 'reset-defer':
        state.deferred = state.deferred.filter(function (x) { return x !== id; });
        toast('Back in the running', 'check');
        render();
        return;
      case 'defer-task':
        state.later.push(id);
        toast('Set aside — one tap brings it back', 'clock');
        render();
        return;
      case 'reset-later':
        state.later = [];
        toast('All set-aside tasks are back', 'check');
        render();
        return;
      case 'done':
        state.queue[id] = 'done';
        toast('Done. That is logged as work, not as knowledge.', 'check');
        render();
        return;
      case 'undo-done':
        state.queue[id] = 'open';
        render();
        return;
      case 'start': startFocus(hit.getAttribute('data-kind'), id); return;
      case 'dur-open':
        state.durEdit = true;
        render();
        var dv = $('#durV');
        if (dv) { dv.focus(); dv.select(); }
        return;
      case 'dur-save': commitDur(); return;
      case 'stop-focus': stopFocus(false); return;
      case 'plan-range': state.planRange = hit.getAttribute('data-v'); render(); return;
      case 'prog-range': state.progressRange = hit.getAttribute('data-v'); render(); return;
      case 'reveal': state.revealed = true; render(); setTimeout(function () { var b = $('.cd-rate__btn'); if (b) b.focus(); }, 30); return;
      case 'rate': grade(parseInt(hit.getAttribute('data-v'), 10)); return;
      case 'start-review': state.reviewsStarted = true; render(); return;
      case 'review-jump': {
        var m = state.reviews.filter(function (r) { return r.topic === (topic(id) || {}).name; })[0];
        state.reviews = m ? [m].concat(state.reviews.filter(function (r) { return r !== m; })) : state.reviews;
        state.reviewsStarted = true;
        navigate('#/review');
        return;
      }
      case 'toast-demo': toast('Toasts never carry an action you cannot repeat', 'info'); return;
    }
  }

  function typing(t) {
    return t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable);
  }
  function onInput(e) {
    if (e.target.id !== 'palInput') return;
    state.q = e.target.value;
    state.sel = 0;
    renderLayer();
  }
  function onKey(e) {
    var mod = e.metaKey || e.ctrlKey;
    if (mod && e.key.toLowerCase() === 'k') { e.preventDefault(); if (state.layer) { closeLayer(); } else { openLayer('find'); } return; }

    if (state.layer) {
      if (e.key === 'Escape') { e.preventDefault(); closeLayer(); return; }
      if (e.key === 'ArrowDown') { e.preventDefault(); moveSel(1); return; }
      if (e.key === 'ArrowUp') { e.preventDefault(); moveSel(-1); return; }
      if (e.key === 'Enter') { e.preventDefault(); runSel(); return; }
      if (e.key === 'Tab') { e.preventDefault(); return; }
      return;
    }
    /* While the length is open it owns Enter and Escape, so a session can be
       typed without the app navigating out from under the caret. */
    if (state.durEdit) {
      if (e.key === 'Enter') { e.preventDefault(); commitDur(); return; }
      if (e.key === 'Escape') { e.preventDefault(); state.durEdit = false; render(); return; }
    }
    if (typing(e.target)) return;

    if (e.key === '/') { e.preventDefault(); openLayer('find'); return; }
    if (e.key.toLowerCase() === 'a' && !mod) { e.preventDefault(); openLayer('add'); return; }

    var r = currentReview();
    if (state.route.name === 'review' && r) {
      if (e.key === ' ' && !state.revealed) { e.preventDefault(); state.revealed = true; render(); return; }
      if (state.revealed && e.key >= '1' && e.key <= '4') { e.preventDefault(); grade(parseInt(e.key, 10)); return; }
    }
    if (e.key >= '1' && e.key <= '5') {
      e.preventDefault();
      navigate(NAV[parseInt(e.key, 10) - 1].href);
      return;
    }
    if (e.key === 'Escape') {
      var openIds = Object.keys(state.open).filter(function (k) { return state.open[k]; });
      if (openIds.length) { state.open = {}; render(); }
      else if (document.activeElement) document.activeElement.blur();
      return;
    }
  }

  /* ───────────────────────────────────────────────────────── 13 · ROUTER ── */
  function viewFor(route) {
    switch (route.name) {
      case 'today': return viewToday();
      case 'plan': return viewPlan();
      case 'courses': return viewCourses();
      case 'course': return viewCourse(route.id);
      case 'review': return viewReview();
      case 'progress': return viewProgress();
      case 'system': return viewSystem();
      default: return viewToday();
    }
  }
  function parseHash() {
    var h = location.hash.replace(/^#\/?/, '') || 'today';
    var parts = h.split('/');
    if (parts[0] === 'course') return { name: 'course', id: parts[1] || COURSES[0].id };
    var known = NAV.map(function (n) { return n.name; }).concat(['system']);
    return { name: known.indexOf(parts[0]) === -1 ? 'today' : parts[0], id: null };
  }
  function render() {
    state.route = parseHash();
    var v = $('#view');
    v.innerHTML = viewFor(state.route);
    renderNav();
    renderTimer();
    syncChrome();
    if (state.pendingTopic) {
      var t = state.pendingTopic;
      state.pendingTopic = null;
      state.open[t] = true;
      render();
      var node = $('[data-act="topic"][data-id="' + t + '"]');
      if (node) node.scrollIntoView({ block: 'center', behavior: 'smooth' });
    }
  }
  function route() {
    state.durEdit = false;
    applyOs();
    render();
    $('#scroller').scrollTop = 0;
    window.scrollTo(0, 0);
    if (booted) $('#view').focus({ preventScroll: true });
  }

  /* ─────────────────────────────────────────────────────────── 14 · BOOT ── */
  var booted = false;
  document.addEventListener('click', onClick);
  document.addEventListener('input', onInput);
  document.addEventListener('keydown', onKey);
  window.addEventListener('hashchange', route);
  /* Read back what <head> already decided rather than asking the OS a second
     time — the attribute is the single source of truth for the first theme, so
     the room and the glyph on the rail can never be one launch apart. Asking
     the OS again here would also let a dark desktop overwrite the choice a
     student made, since applyTheme() writes what it is given. */
  state.theme = document.documentElement.getAttribute('data-theme') === 'dark' ? 'dark' : 'light';
  initOs();
  applyTheme();
  var search = $('#q');
  if (search) {
    search.addEventListener('focus', function () { search.blur(); openLayer('find'); });
    search.addEventListener('click', function () { openLayer('find'); });
  }
  route();
  booted = true;
  console.log('[cadence] warm workbench · system page at #/system');
})();
