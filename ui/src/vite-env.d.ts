/// <reference types="vite/client" />

/**
 * The app imports its stylesheet for its EFFECTS, not for a value:
 *
 *     import './styles/app.css';
 *
 * Vite's client types declare `*.css` as an empty module, which a bundler
 * accepts and a language service in a projectless context does not
 * (`ts:2882`). Stating the declaration here is the app's own contract rather
 * than a fix to Vite's: this project has exactly one stylesheet entry point and
 * no CSS module, so the shape is known and worth writing down.
 */
declare module '*.css';

/** Svelte 5 components are single-file; the compiler types them, not TS. */
declare module '*.svelte' {
  import type { Component } from 'svelte';

  const component: Component<Record<string, unknown>>;
  export default component;
}
