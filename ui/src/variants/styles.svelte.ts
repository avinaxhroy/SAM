/**
 * COMPONENT STYLES — the student's own choice, per component (2026-09-29).
 *
 * The lab carries three designs per surface and none of them is a draft of the
 * others: they are three styles of the same component, and this module is where
 * the kept one lives. The choice is a preference, not plan data — the engine
 * has no business knowing which table design a student likes — so it is stored
 * beside the theme (`cadence.*`, the app's own keys) and survives a relaunch.
 *
 * The **size** is the second half of the same idea: ported components measure
 * themselves from `--ui-s` (`variants.css`), so one choice moves every row,
 * pad, gap and type step together — compact for a small window, spacious for a
 * large one — and every ported component reflows again on its own container
 * query. Nothing here is a plan write and nothing here is a reload: the store
 * is reactive and the screens redraw where they stand.
 *
 * A stored value that is no longer in the catalog (a surface renamed, a variant
 * retired in a later review) is dropped at read, so a stale key can never make
 * a screen fail to draw: the fallback is the surface's declared default.
 */
import { SURFACES, SURFACE_BY_KEY, type VariantId } from './catalog';
import { sendPreference } from '../ipc';

export type UiSize = 'compact' | 'regular' | 'spacious';

const CHOICES_KEY = 'cadence.component-styles';
const SIZE_KEY = 'cadence.ui-size';

function isSize(value: unknown): value is UiSize {
  return value === 'compact' || value === 'regular' || value === 'spacious';
}

/** Keep only the surfaces and variants the catalog still declares. */
function sanitizeChoices(raw: unknown): Record<string, VariantId> {
  const kept: Record<string, VariantId> = {};
  if (typeof raw !== 'object' || raw === null) return kept;
  for (const [key, value] of Object.entries(raw as Record<string, unknown>)) {
    const surface = SURFACE_BY_KEY[key];
    if (!surface) continue;
    if (surface.variants.some((variant) => variant.id === value)) kept[key] = value as VariantId;
  }
  return kept;
}

/** Read and sanitize the stored choices: only surfaces and variants that exist. */
function readChoices(): Record<string, VariantId> {
  try {
    const raw = localStorage.getItem(CHOICES_KEY);
    return raw ? sanitizeChoices(JSON.parse(raw)) : {};
  } catch {
    /* a blocked or malformed store paints the defaults, never nothing */
    return {};
  }
}

function readSize(): UiSize {
  try {
    const raw = localStorage.getItem(SIZE_KEY);
    if (isSize(raw)) return raw;
  } catch {
    /* same floor as above */
  }
  return 'regular';
}

class ComponentStyles {
  /** The kept style per surface; a surface absent here wears its default. */
  choices = $state<Record<string, VariantId>>({});
  /** Compact · regular · spacious — the multiplier every ported component reads. */
  size = $state<UiSize>('regular');

  constructor() {
    this.choices = readChoices();
    this.size = readSize();
    this.apply();
  }

  /** The style a surface draws with: the kept one, else the catalog's default. */
  variantOf(surface: string): VariantId {
    const def = SURFACE_BY_KEY[surface];
    const chosen = this.choices[surface];
    if (def && chosen && def.variants.some((variant) => variant.id === chosen)) return chosen;
    return def?.default ?? 'a';
  }

  setVariant(surface: string, variant: VariantId): void {
    const def = SURFACE_BY_KEY[surface];
    if (!def || !def.variants.some((entry) => entry.id === variant)) return;
    // One key, written in place: a new map would invalidate every reader of the
    // store, including the hosts drawing designs this press did not touch.
    this.choices[surface] = variant;
    try {
      localStorage.setItem(CHOICES_KEY, JSON.stringify(this.choices));
    } catch {
      /* the choice stands for this session even when storage refuses */
    }
    this.publish();
  }

  /** One choice for every surface that offers it — the "match every component" row. */
  setEveryVariant(variant: VariantId): void {
    const next: Record<string, VariantId> = { ...this.choices };
    for (const surface of SURFACES) {
      if (surface.variants.some((entry) => entry.id === variant)) next[surface.key] = variant;
    }
    this.choices = next;
    try {
      localStorage.setItem(CHOICES_KEY, JSON.stringify(this.choices));
    } catch {
      /* as above */
    }
    this.publish();
  }

  setSize(size: UiSize): void {
    this.size = size;
    try {
      localStorage.setItem(SIZE_KEY, size);
    } catch {
      /* as above */
    }
    this.apply();
    this.publish();
  }

  /** Every surface back to the lab's own default for it. */
  resetVariants(): void {
    this.choices = {};
    try {
      localStorage.removeItem(CHOICES_KEY);
    } catch {
      /* as above */
    }
    this.publish();
  }

  /**
   * Apply a snapshot made in the other window (the Preferences window is a
   * second webview with its own copy of this store). No publish: adopting a
   * choice the student already made must not send it back.
   */
  adopt(snapshot: string): void {
    let parsed: unknown;
    try {
      parsed = JSON.parse(snapshot);
    } catch {
      return;
    }
    if (typeof parsed !== 'object' || parsed === null) return;
    const { choices, size } = parsed as { choices?: unknown; size?: unknown };
    this.choices = sanitizeChoices(choices);
    if (isSize(size) && size !== this.size) {
      this.size = size;
      this.apply();
    }
    try {
      localStorage.setItem(CHOICES_KEY, JSON.stringify(this.choices));
      localStorage.setItem(SIZE_KEY, this.size);
    } catch {
      /* the memory still holds it for this session */
    }
  }

  /** Hand the whole store to the window that draws the components. */
  private publish(): void {
    sendPreference({
      key: 'component-styles',
      value: JSON.stringify({ choices: this.choices, size: this.size }),
    });
  }

  /**
   * The one DOM write this store owns: `data-ui-size` on the document root.
   * The same attribute is set by the pre-paint script in `index.html`, so the
   * first frame already measures what the student chose.
   */
  private apply(): void {
    document.documentElement.dataset.uiSize = this.size;
  }
}

export const styles = new ComponentStyles();
