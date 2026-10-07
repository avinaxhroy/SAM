/* ══════════════════════════════════════════════════════════════════════════
   SAM · THE PAGE-SIDE ACCESSIBILITY REPORT (dev only)

   Phase 7's interaction half, in the half a machine can decide: a function that
   runs **inside the page** and reports what is actually rendered — accessible
   names, focusability, dialog semantics, reduced-motion durations, text-scale
   overflow, colour-only cues and computed contrast.

   What it cannot decide is left to the platform checklist (VoiceOver/Narrator,
   IME, substitutions, a real ⌘Z in a field): those are recorded per platform in
   `BUILDLOG.md`, never assumed here.

   Usage (from a driver with a Puppeteer `page`):
     const report = await page.evaluate(A11Y_PAGE_SCRIPT);
   Then validate it with `node tools/a11ycheck.mjs --report <file.json>`.
   ══════════════════════════════════════════════════════════════════════════ */

export const A11Y_PAGE_SCRIPT = `(() => {
  const name = (el) => {
    const aria = el.getAttribute('aria-label');
    if (aria && aria.trim()) return aria.trim();
    const labelled = el.getAttribute('aria-labelledby');
    if (labelled) {
      const text = labelled.split(/\\s+/)
        .map((id) => document.getElementById(id)?.textContent ?? '')
        .join(' ').trim();
      if (text) return text;
    }
    const text = (el.textContent ?? '').trim();
    if (text) return text;
    const title = el.getAttribute('title');
    if (title && title.trim()) return title.trim();
    const placeholder = el.getAttribute('placeholder');
    if (placeholder && placeholder.trim()) return placeholder.trim();
    if (el.id) {
      const label = document.querySelector('label[for="' + CSS.escape(el.id) + '"]');
      if (label?.textContent?.trim()) return label.textContent.trim();
    }
    const wrapping = el.closest('label');
    if (wrapping?.textContent?.trim()) return wrapping.textContent.trim();
    return '';
  };

  const describe = (el) => {
    const id = el.id ? '#' + el.id : '';
    const command = el.getAttribute('data-command');
    const cls = (el.className || '').toString().split(/\\s+/).filter(Boolean).slice(0, 2).join('.');
    return el.tagName.toLowerCase() + id + (cls ? '.' + cls : '') + (command ? '[' + command + ']' : '');
  };

  const focusable = (el) =>
    !el.hasAttribute('disabled') &&
    (['BUTTON', 'INPUT', 'SELECT', 'TEXTAREA'].includes(el.tagName) ||
      (el.tagName === 'A' && el.hasAttribute('href')) ||
      Number(el.getAttribute('tabindex') ?? '-1') >= 0);

  // What counts as a control. A negative tabindex is the *container* pattern — a
  // dialog focused on open, a scroll region like CodeMirror's — so it is only a
  // finding when the element also carries a role a user would take for a control
  // (a fake button is still a fake button). Before this rule an open dialog, or
  // an open source pane, failed the gate for being what it is.
  const INTERACTIVE_ROLES = new Set([
    'button', 'link', 'menuitem', 'menuitemcheckbox', 'menuitemradio', 'option',
    'tab', 'checkbox', 'radio', 'switch', 'combobox', 'textbox', 'searchbox',
  ]);
  const interactive = [
    ...document.querySelectorAll(
      'button, a[href], input:not([type="hidden"]), select, textarea, [tabindex], [data-command]'
    ),
  ].filter((el) => {
    const tabindex = el.getAttribute('tabindex');
    if (tabindex !== null && Number(tabindex) < 0) {
      const role = (el.getAttribute('role') ?? '').toLowerCase();
      if (!INTERACTIVE_ROLES.has(role)) return false;
    }
    return el.offsetParent !== null || el === document.activeElement;
  });

  const withoutName = [];
  const weakName = [];
  const unfocusable = [];
  for (const el of interactive) {
    const label = name(el);
    if (!label) withoutName.push(describe(el));
    // A one-character name is a name that says nothing ("T" for Today): the
    // control must carry a real label, which a single letter is not.
    else if (label.trim().length < 2) weakName.push(describe(el) + ' → "' + label + '"');
    // A disabled control is *meant* to be unreachable; it still had to pass the
    // name check above. Only an enabled control that cannot be reached by Tab
    // is a finding.
    if (!el.hasAttribute('disabled') && !focusable(el)) unfocusable.push(describe(el));
  }

  const dialogNodes = [...document.querySelectorAll('[role="dialog"], [role="alertdialog"], .cd-sheet, .cd-palette')];
  const dialogsMissingRole = [];
  const dialogsMissingModal = [];
  for (const el of dialogNodes) {
    const role = el.getAttribute('role');
    if (role !== 'dialog' && role !== 'alertdialog') dialogsMissingRole.push(describe(el));
    if (el.getAttribute('aria-modal') !== 'true') dialogsMissingModal.push(describe(el));
  }

  const durations = [];
  const sampling = [...document.querySelectorAll('button, .cd-card, .cd-toast, .cd-menu')].slice(0, 40);
  for (const el of sampling) {
    const style = getComputedStyle(el);
    const transition = Math.max(
      0,
      ...style.transitionDuration.split(',').map((part) => parseFloat(part) * (part.includes('ms') ? 1 : 1000))
    );
    const animation = Math.max(
      0,
      ...style.animationDuration.split(',').map((part) => parseFloat(part) * (part.includes('ms') ? 1 : 1000))
    );
    if (transition > 20 || animation > 20) durations.push(describe(el) + ' transition=' + transition + 'ms');
  }

  // A cue that is nothing but a colour: a wash or chip element with no text and
  // no accessible name does not say what it means to anyone who cannot see it.
  const colourOnly = [];
  // The wash selectors are ANCHORED, and they have to be. The rule as it stood
  // was an attribute-contains selector on the two characters w and hyphen, which
  // matches any class containing that pair anywhere in it: tw-acts, tw-grip and
  // cd-row-flex all match. It reported twelve "colour-only" elements on Today and
  // eight on Plan, and every one was a container, or a decorative chevron inside
  // a button that already carried an accessible name. The tempting fix is
  // aria-hidden on the container, which turns the gate green and hides a row's
  // real Log button from every screen reader: a gate satisfied by breaking the
  // thing it measures.
  //
  // So the selector is corrected to what it means: a class that BEGINS with the
  // wash prefix, or the data-w attribute the system's identity mapping sets
  // (design/components.css:14). A substring match standing in for a naming
  // convention is the bug here; the app was right.
  //
  // (This file is itself a template literal, so a backtick in a comment ends the
  // string. That has now bitten three times in this file and it is written down
  // rather than left to the next person.)
  for (const el of document.querySelectorAll(
    '[class*="cd-chip"], [class*="cd-wash"], [class^="w-"], [class*=" w-"], [data-w], [data-status]',
  )) {
    if (el.offsetParent === null) continue;
    if ((el.textContent ?? '').trim().length > 0) continue;
    if (name(el)) continue;
    colourOnly.push(describe(el));
  }

  const parseColour = (text) => {
    const match = text.match(/rgba?\\(([^)]+)\\)/);
    if (!match) return null;
    const parts = match[1].split(/[,\\/\\s]+/).filter(Boolean).map(Number);
    if (parts.length < 3) return null;
    return { r: parts[0], g: parts[1], b: parts[2], a: parts.length > 3 ? parts[3] : 1 };
  };

  const chain = (el) => {
    let node = el;
    while (node) {
      const colour = parseColour(getComputedStyle(node).backgroundColor);
      if (colour && colour.a > 0.5) return colour;
      node = node.parentElement;
    }
    return { r: 255, g: 255, b: 255, a: 1 };
  };

  const linear = (channel) => {
    const c = channel / 255;
    return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  };
  const luminance = (colour) =>
    0.2126 * linear(colour.r) + 0.7152 * linear(colour.g) + 0.0722 * linear(colour.b);
  const ratio = (a, b) => {
    const first = luminance(a);
    const second = luminance(b);
    const [lighter, darker] = first >= second ? [first, second] : [second, first];
    return (lighter + 0.05) / (darker + 0.05);
  };

  const contrast = [];
  const samples = [
    ...document.querySelectorAll('h1, h2, h3, h4, p, span, code, button, td, th, li, label, kbd, a'),
  ].filter((el) => (el.textContent ?? '').trim().length > 1 && el.offsetParent !== null).slice(0, 400);
  for (const el of samples) {
    const style = getComputedStyle(el);
    const fg = parseColour(style.color);
    if (!fg || fg.a < 1) continue;
    const bg = chain(el);
    const value = ratio(fg, bg);
    const size = parseFloat(style.fontSize);
    const bold = Number(style.fontWeight) >= 700;
    const large = size >= 24 || (bold && size >= 18.66);
    const floor = large ? 3 : 4.5;
    if (value + 0.01 < floor) {
      contrast.push({
        element: describe(el),
        text: (el.textContent ?? '').trim().slice(0, 40),
        ratio: Math.round(value * 100) / 100,
        floor,
        colour: style.color,
        background: 'rgb(' + bg.r + ',' + bg.g + ',' + bg.b + ')',
      });
    }
  }

  return {
    url: location.href,
    theme: document.documentElement.getAttribute('data-theme'),
    themeId: document.documentElement.getAttribute('data-theme-id'),
    textScale: getComputedStyle(document.documentElement).getPropertyValue('--text-scale').trim(),
    interactive: interactive.length,
    withoutName,
    weakName,
    unfocusable,
    dialogs: dialogNodes.length,
    dialogsMissingRole,
    dialogsMissingModal,
    reducedMotion: durations,
    colourOnly,
    contrastChecked: samples.length,
    contrastFailures: contrast,
    overflow: document.documentElement.scrollWidth - window.innerWidth,
  };
})()`;
