/**
 * The resolved appearance (§6 Phase 7): the one place the engine's resolution
 * becomes CSS.
 *
 * The engine resolves the theme, the mode and the text scale into a complete
 * custom-property set (`appearance.resolve`); this module applies it to
 * `:root` and caches it so the **next** launch paints it before the first frame
 * — the same trick `design/demo/index.html` uses for light/dark, extended to a
 * whole theme. Inline custom properties beat the stylesheet's `[data-theme]`
 * block by specificity, so the resolved set is what renders.
 */

export type ContrastPair = {
  fg: string;
  bg: string;
  ratio: number;
  required: number;
  ok: boolean;
  computed: boolean;
  note?: string;
};

export type AppearanceRead = {
  theme: string;
  themeName: string;
  mode: 'light' | 'dark' | 'auto';
  resolvedMode: 'light' | 'dark';
  textScale: number;
  css: Record<string, string>;
  contrast: ContrastPair[];
  warnings: string[];
};

export type ThemeRead = {
  id: string;
  name: string;
  mode: 'light' | 'dark' | 'auto';
  description?: string | null;
};

const THEME_KEY = 'cadence.theme';
const APPEARANCE_KEY = 'cadence.appearance';

/**
 * Stored light/dark preference from localStorage, falling back to system media query.
 */
export function storedMode(): 'light' | 'dark' {
  try {
    const stored = localStorage.getItem(THEME_KEY);
    if (stored === 'light' || stored === 'dark') return stored;
  } catch {
    /* storage can be blocked in an embedded view */
  }
  return globalThis.matchMedia?.('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

export function rememberMode(mode: 'light' | 'dark'): void {
  try {
    localStorage.setItem(THEME_KEY, mode);
  } catch {
    /* storage can be blocked */
  }
}

/**
 * The engine serializes non-scalar tokens (such as font stacks and ease curves)
 * as `'(structure)'`. Writing this literal to CSS custom properties produces
 * invalid computed values at runtime, so skip it and keep the token stylesheet defaults.
 */
const NOT_A_CSS_VALUE = '(structure)';

/**
 * Appearance css travels with the plan (and with an imported profile), so the
 * names are allowlisted to the design-token vocabulary and the values to plain
 * css text: a token name can never be anything but `--`-prefixed identifier
 * characters, and a value can never carry a `url()` (or the long-dead
 * `expression()`) — a custom property is styling, not a fetch or a script.
 */
function safeToken(name: string, value: string): boolean {
  return (
    /^--[\w$.-]+$/i.test(name) &&
    !/url\(|expression\(|javascript:/i.test(value)
  );
}

/** Apply a resolved appearance to the document, and cache it for first paint. */
export function applyAppearance(appearance: AppearanceRead): void {
  const root = document.documentElement;
  root.setAttribute('data-theme', appearance.resolvedMode);
  root.setAttribute('data-theme-id', appearance.theme);
  for (const [name, value] of Object.entries(appearance.css)) {
    if (value === NOT_A_CSS_VALUE || !safeToken(name, value)) continue;
    root.style.setProperty(name, value);
  }
  root.style.setProperty('--text-scale', String(appearance.textScale));
  try {
    localStorage.setItem(
      APPEARANCE_KEY,
      JSON.stringify({ mode: appearance.resolvedMode, theme: appearance.theme, css: appearance.css }),
    );
  } catch {
    /* storage can be blocked; the next launch paints the floor */
  }
}

/**
 * The cached appearance, for the pre-paint script in `index.html`. Exported so
 * the script and this module cannot disagree about the key or the shape.
 */
export function cachedAppearance(): { mode?: string; css?: Record<string, string> } | null {
  try {
    const raw = localStorage.getItem(APPEARANCE_KEY);
    return raw ? (JSON.parse(raw) as { mode?: string; css?: Record<string, string> }) : null;
  } catch {
    return null;
  }
}

/** One finding from `design.check` (§5): its rule, its severity and its path. */
export type DesignFinding = {
  rule: string;
  severity: string;
  path: string;
  message: string;
  waived: boolean;
  reason?: string | null;
};

/** One rule's report: whether the engine could look, and what it found. */
export type DesignRuleReport = {
  id: string;
  severity: string;
  checked: boolean;
  note: string;
  findings: DesignFinding[];
};

/** `SAM design.check --json` — §5's lint table over the resolved appearance. */
export type DesignReport = {
  theme: AppearanceRead;
  rules: DesignRuleReport[];
  errors: number;
  advisories: number;
  waived: number;
  unobservable: string[];
};

/** The token paths the override editor offers first — the ones a student changes. */
export const OVERRIDE_SUGGESTIONS = [
  'color.surface.backdrop',
  'color.surface.card',
  'color.ink-level.ink',
  'color.identity.mint.surface',
  'color.identity.mint.ink',
  'color.identity.lilac.surface',
  'color.identity.butter.surface',
  'color.identity.sky.surface',
  'fontSize.base',
  'radius.card',
] as const;
