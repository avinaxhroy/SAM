/**
 * THE VARIANT REGISTRY — surface key in, component out (2026-09-29).
 *
 * Each surface is a folder (`./<key>/index.ts`) whose default export is a map
 * of `{ a, b, c }` to its designs. `import.meta.glob` is the one import list:
 * a folder that exists is a folder that ships, so adding a surface is adding a
 * folder — there is no second registry to keep in sync, and no import to forget.
 *
 * A missing variant falls back to the surface's default, and a missing surface
 * draws nothing: the app never shows a placeholder where a design should be.
 */
import type { Component } from 'svelte';
import type { VariantId } from './catalog';

export type VariantComponents = Partial<Record<VariantId, Component<Record<string, unknown>>>>;

const modules = import.meta.glob<{ default: VariantComponents }>('./*/index.ts', { eager: true });

/** `./courses/index.ts` → `courses`. */
const BY_SURFACE: Record<string, VariantComponents> = Object.fromEntries(
  Object.entries(modules).map(([path, module]) => [path.slice(2, -'/index.ts'.length), module.default]),
);

export function componentFor(
  surface: string,
  variant: VariantId,
): Component<Record<string, unknown>> | null {
  const variants = BY_SURFACE[surface];
  if (!variants) return null;
  return variants[variant] ?? variants.a ?? null;
}
