/**
 * THE VARIANT REGISTRY — surface key in, component out (2026-09-29).
 *
 * Each surface is a folder (`./<key>/index.ts`) whose default export is a map
 * of `{ a, b, c }` to its designs. `import.meta.glob` is the one import list:
 * a folder that exists is a folder that ships, so adding a surface is adding a
 * folder — there is no second registry to keep in sync, and no import to forget.
 *
 * The glob is **lazy**: the shipped app draws one design per surface, so eagerly
 * importing all 78 designs put ~880 KB of component JS and ~388 KB of CSS on the
 * boot path for code most windows never mount. Each loader is a chunk Vite emits
 * on first use; the cache keeps a variant mounted while its screen stays open.
 *
 * A missing variant falls back to the surface's default, and a missing surface
 * draws nothing: the app never shows a placeholder where a design should be.
 */
import type { Component } from 'svelte';
import type { VariantId } from './catalog';

export type VariantComponents = Partial<Record<VariantId, Component<Record<string, unknown>>>>;

type VariantModule = { default: VariantComponents };

const modules = import.meta.glob<VariantModule>('./*/index.ts');

/** `./courses/index.ts` → courses. */
const LOADERS: Record<string, () => Promise<VariantModule>> = Object.fromEntries(
  Object.entries(modules).map(([path, load]) => [path.slice(2, -'/index.ts'.length), load]),
);

const CACHE = new Map<string, Component<Record<string, unknown>> | null>();

/** Resolve one design; the loader runs once per surface. */
export async function loadComponent(
  surface: string,
  variant: VariantId,
): Promise<Component<Record<string, unknown>> | null> {
  const key = `${surface}:${variant}`;
  const cached = CACHE.get(key);
  if (cached !== undefined) return cached;
  const load = LOADERS[surface];
  if (!load) {
    CACHE.set(key, null);
    return null;
  }
  const variants = (await load()).default;
  const chosen = variants[variant] ?? variants.a ?? null;
  CACHE.set(key, chosen);
  return chosen;
}
