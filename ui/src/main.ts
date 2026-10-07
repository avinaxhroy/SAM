/**
 * The frontend's entry (§4.2): mount the app, and set the one thing only the
 * shell can know.
 *
 * No state-management library (§9): the engine publishes one snapshot per
 * accepted revision, and a store here would be a second source of truth to keep
 * in sync. What this file holds is the shell's own answer, once, at boot.
 */
import './styles/app.css';

import { mount } from 'svelte';

import App from './App.svelte';
import { shellInfo } from './ipc';
import { app } from './session.svelte';

const target = document.getElementById('app');
if (!target) throw new Error('index.html has no #app to mount into');

shellInfo().then((info) => {
  const os = info?.os ?? 'mac';

  // `data-os` is what the design system switches on — the insets, and the
  // shortcut glyphs, which are resolved from it rather than typed per platform.
  document.documentElement.dataset.os = os;

  // `data-titlebar` is the one fact about the window's furniture the page
  // cannot work out for itself, and the design system already has a rule for
  // each answer: "overlay" reserves `--titlebar-inset-mac` / `-win` so the
  // identity group starts clear of the platform's own controls, and "plain"
  // reserves nothing because they sit in their own bar above the content
  // (`design/components.css:50-51`). It is deliberately NOT derived from `os`:
  // the browser harness reports the same `os` with no traffic lights at all,
  // which is how 78px of empty sheet got reserved in every capture.
  document.documentElement.dataset.titlebar =
    info?.titlebarOverlay === true ? 'overlay' : 'plain';

  // §9/D21: which renderer path the shell's launch took. It is surfaced in
  // Preferences — a bug report can read it off the screen instead of guessing.
  if (info?.rendererPath) app.rendererPath = info.rendererPath;

  // The shell or the dev harness names the plan it was started for; a page with
  // neither boots into the picker (Appendix C.5's `noneExists`/`several`).
  //
  // `#preferences` is the OS's second window (§4.10: *"About and Preferences as
  // OS windows"*). It is the same app — the same session, the same engine, the
  // same settings screen — mounted in a window whose whole content is Settings;
  // the shell opens it with `open_preferences`.
  const preferences = location.hash === '#preferences';
  mount(App, { target, props: { os, plan: info?.plan ?? null, preferences } });
});
