/**
 * SAM frontend build (D19).
 *
 * Two decisions are wired here rather than in Phase 0B, because they are
 * already fixed and both affect where assets resolve:
 *
 * - `base: './'` — the bundle is read from the app's resource directory, not
 *   from a server root, so asset URLs must be relative.
 * - no SSR metaframework — Tauri serves static assets and does not support
 *   server-based solutions, so SvelteKit would exist to be disabled (§9).
 */
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  // `design/tailwind.css` maps the tokens onto Tailwind's theme layer, so the
  // utility layer and the component layer speak one vocabulary (§5, D19).
  plugins: [tailwindcss(), svelte()],
  base: './',
  // `devUrl` in src-tauri/tauri.conf.json is this port (Tauri's own default),
  // and `strictPort` is what stops Vite from silently moving to 1421 and
  // leaving the shell pointed at nothing.
  server: { port: 1420, strictPort: true },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
});
