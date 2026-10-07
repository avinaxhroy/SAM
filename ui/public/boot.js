/**
 * Pre-paint bootstrap, served as a plain static file (`/boot.js`) rather than
 * an inline script: the shell's CSP allows no inline scripts, and it must run
 * before the app bundle paints anything (see index.html's own comment).
 * Kept in sync with the module copy at ../src/boot.ts? No — this IS the copy;
 * `src/boot.ts` was removed in favour of this file.
 */

(function () {
  var root = document.documentElement;
  var cachedAppearance = false;
  try {
    var cached = localStorage.getItem('cadence.appearance');
    cachedAppearance = Boolean(cached);
    if (cached) {
      var appearance = JSON.parse(cached);
      if (appearance.mode === 'light' || appearance.mode === 'dark') {
        root.setAttribute('data-theme', appearance.mode);
      }
      if (appearance.theme) root.setAttribute('data-theme-id', appearance.theme);
      var css = appearance.css || {};
      for (var name in css) {
        if (Object.prototype.hasOwnProperty.call(css, name)) {
          // The cache is written by the app, but it is storage: apply only
          // design-token names with plain css values, exactly as
          // appearance.ts does on the resolved path.
          if (!/^--[\w$.-]+$/i.test(name)) continue;
          if (/url\(|expression\(|javascript:/i.test(css[name])) continue;
          // `(structure)` is the engine's placeholder for a value it could
          // not serialise as one text (an array token — the font stack,
          // `--ease`). Writing it makes every declaration that reads the
          // token invalid at computed-value time — no eased motion at all
          // — so the design tokens' own value stands instead.
          if (css[name] === '(structure)') continue;
          root.style.setProperty(name, css[name]);
        }
      }
    }
  } catch (e) {
    /* a blocked or malformed cache is not a reason to paint nothing */
  }
  if (!cachedAppearance) {
  try {
    var t = localStorage.getItem('cadence.theme');
    if (t !== 'light' && t !== 'dark') {
      t =
        window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches
          ? 'dark'
          : 'light';
    }
    root.setAttribute('data-theme', t);
  } catch (e) {
    /* storage or matchMedia can be missing in an embedded view */
  }
  }
  try {
    var size = 'regular';
    var stored = localStorage.getItem('cadence.ui-size');
    if (stored === 'compact' || stored === 'regular' || stored === 'spacious') size = stored;
    document.documentElement.setAttribute('data-ui-size', size);
  } catch (e) {
    /* a blocked store keeps the default */
  }
})();
