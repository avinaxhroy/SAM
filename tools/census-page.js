/* ══════════════════════════════════════════════════════════════════════════
   SAM · THE PAGE-SIDE STRING CENSUS (dev only)

   `UI_PLAN.md` D2's instrument, in the half a machine can decide: every string
   a student can actually read, measured against the vocabulary the interface is
   forbidden to speak — JSON pointers, schema keys, registry ids, absolute
   paths, raw ISO dates, unrounded floats, revision hashes, formula transcripts.

   It measures the **rendered** page, not the source: a key that exists in
   `views.json` and never reaches the screen is not a leak, and a string that is
   built at runtime (`` `+ attempted ${field}` ``) is invisible to a grep. What
   it cannot decide is whether the remaining words are *good* — that is the
   maker's recheck, and no script replaces it.

   Usage (from a driver with a Puppeteer `page`), one report per destination:
     const report = await page.evaluate(CENSUS_PAGE_SCRIPT);
     report.page = 'Today';                       // the driver names the page
     writeFileSync('census-today.json', JSON.stringify(report));
   Then judge the reports with:
     node tools/uicensus.mjs --report census-*.json

   The exemptions are deliberate and each has exactly one intended tenant: **the
   identity pair** (a control's own key, shown beside its friendly label —
   `SAM_PLAN.md` §4.8's teaching device) marks itself `data-identity`, and the
   **four doors** (the source pane, the CLI output, the copy-as-JSON menus) mark
   themselves `data-developer`. What is left over is the leak: the same strings
   used as page prose. No element carries either marker today.
   ══════════════════════════════════════════════════════════════════════════ */

/**
 * The forbidden vocabulary, as regex sources so the page script and the checker
 * read the same list. Each one is a thing `UI_PLAN.md` §6.3 promises will be 0.
 */
export const CENSUS_RULES = [
  {
    id: 'pointer',
    label: 'JSON pointer or document name',
    source: '(?:[\\w.-]+\\.json(?:#/\\S*)?|#/\\S+|views\\.json|rules\\.json|shell\\.json|types\\.json)',
  },
  {
    id: 'key',
    label: 'schema key in camelCase',
    source: '\\b[a-z]+[A-Z][a-zA-Z0-9]*\\b',
  },
  {
    id: 'dotted',
    label: 'dotted registry id or field path',
    // A dotted token that is part of a URL is not a registry id: `https://
    // jeemain.nta.nic.in/document/…` is a link a student may hold, and the
    // first version of this rule counted its host as a leak (found by running
    // the gate over the app's Library screen). The lookarounds keep the match
    // inside a standalone token — never after `/`, `.` or a word character, and
    // never followed by one — so `p.jee.chem.atomic` still counts and a host
    // inside a URL does not.
    source: '(?<![\\w/.])[a-z][a-z0-9]*(?:\\.[a-z][a-z0-9]*)+\\.?(?![\\w/.])',
  },
  {
    id: 'path',
    label: 'absolute filesystem path',
    source: '/(?:Users|home|tmp|var|private)/\\S*|Application Support',
  },
  {
    id: 'rev',
    label: 'revision hash',
    source: '\\brev\\s+[0-9a-f]{6,}\\b',
  },
  {
    id: 'isoDate',
    label: 'raw ISO date',
    source: '\\b\\d{4}-\\d{2}-\\d{2}(?:T\\d{2}:\\d{2})?\\b',
  },
  {
    id: 'rawFloat',
    label: 'unrounded float',
    source: '\\b\\d+\\.\\d{3,}\\b',
  },
  {
    id: 'formula',
    label: 'formula transcript as prose',
    // An aggregate verb is only a leak when it leads a transcript — `count · 8 of
    // library.resources` — never when the word is English. The demo's own copy
    // ("Oldest first", "last seen 9 days ago") is what proved the loose form wrong.
    source: '^(?:count|sum|avg|min|max|ratio|pct|distinct)\\s*[·(]',
  },
];

/** The census, as a string the driver evaluates inside the page. */
export const CENSUS_PAGE_SCRIPT = `(() => {
  const RULES = ${JSON.stringify(CENSUS_RULES)};
  const compiled = RULES.map((rule) => ({ id: rule.id, re: new RegExp(rule.source) }));
  const SKIP = new Set(['SCRIPT', 'STYLE', 'NOSCRIPT', 'TEMPLATE']);
  // Two exemptions, one tenant each (§4.8): the identity pair beside a control,
  // and the four doors themselves. Everything else in this vocabulary is a leak.
  const EXEMPT = '[data-identity], [data-developer]';

  const counts = {};
  const samples = {};
  let total = 0;

  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  let node;
  while ((node = walker.nextNode())) {
    const parent = node.parentElement;
    if (!parent || SKIP.has(parent.tagName)) continue;
    // The identity pair and the doors are allowed to speak this vocabulary.
    if (typeof parent.closest === 'function' && parent.closest(EXEMPT)) continue;
    const style = getComputedStyle(parent);
    if (style.display === 'none' || style.visibility === 'hidden') continue;
    // Present in the DOM is not the same as rendered: a collapsed pane holds text.
    if (parent.getClientRects().length === 0) continue;

    const text = (node.textContent || '').replace(/\\s+/g, ' ').trim();
    if (!text) continue;
    total += 1;

    for (const rule of compiled) {
      if (!rule.re.test(text)) continue;
      counts[rule.id] = (counts[rule.id] || 0) + 1;
      const list = samples[rule.id] || (samples[rule.id] = []);
      if (list.length < 10 && list.indexOf(text) === -1) list.push(text.slice(0, 100));
    }
  }

  return {
    page: null, // the driver names it
    theme: document.documentElement.getAttribute('data-theme-id') || document.documentElement.getAttribute('data-theme') || null,
    total,
    counts,
    samples,
  };
})()`;
