<!--
  The icon family (§2.2): one set of glyphs on a 16px box at a 1.5px stroke,
  monochrome, `currentColor`. The vocabulary is the plan's own — the names in
  `types.json`'s `icon` field and `shell.json`'s navigation entries (bolt,
  book, calendar, clock, checklist, …) — plus the handful of chrome glyphs the
  frame needs (search, code, gear, plus, close, check). An unknown name draws
  nothing but its box, which is a visible fallback rather than a wrong picture.

  Icons never carry meaning alone: the rail's discs, the titlebar's tools and
  the table's row actions all carry accessible names, and the pixels here are
  `aria-hidden`.
-->
<script module lang="ts">
  /**
   * The one thing about this component another file needs: **which names it can
   * draw**. The composer's screen-creation sheet offers an icon per screen
   * (`COMPOSER.md` §4.7), and a picker built from any other list would offer
   * names this build renders as an empty box — a picker that lies.
   *
   * `ICON_NAMES` is derived from `GLYPHS`, not written beside it, so adding a
   * glyph adds it to the picker and there is no second list to forget.
   */
  export const GLYPHS: Record<string, string[]> = {
    bolt: ['M8.8 2 4 9h3.2L6.8 14 12 7H8.7z'],
    book: [
      'M8 4.3C6.9 3.3 5.3 3 3.5 3H2.5v9h1c1.8 0 3.4.4 4.5 1.4',
      'M8 4.3C9.1 3.3 10.7 3 12.5 3h1v9h-1c-1.8 0-3.4.4-4.5 1.4',
      'M8 4.3v9.1',
    ],
    bubble: ['M3 3.5h10v6.5H6.6L4 12.4V10H3z'],
    calendar: ['M3 4.5h10v8.5H3z', 'M3 7h10', 'M5.8 3v2.6', 'M10.2 3v2.6'],
    character: ['M4.2 12.5 8 3.5l3.8 9', 'M5.6 9.6h4.8'],
    checklist: ['M2.5 4.4l1.4 1.4 2.3-2.4', 'M2.5 11.4l1.4 1.4 2.3-2.4', 'M8.5 4.6h5', 'M8.5 11.6h5'],
    clock: ['M8 3a5 5 0 1 0 0 10A5 5 0 0 0 8 3z', 'M8 5.4v2.9l2 1.2'],
    flag: ['M4 13V3.2', 'M4 4h7.2l-1.5 2 1.5 2H4z'],
    globe: ['M8 2.8a5.2 5.2 0 1 0 0 10.4A5.2 5.2 0 0 0 8 2.8z', 'M2.8 8h10.4', 'M8 2.8c2 1.9 2 8.5 0 10.4', 'M8 2.8c-2 1.9-2 8.5 0 10.4'],
    graduationcap: ['M2 6.4 8 3.4l6 3-6 3z', 'M4.6 8.1v2.7c0 1 1.5 1.7 3.4 1.7s3.4-.7 3.4-1.7V8.1', 'M14 6.9v2.6'],
    hammer: ['M3.2 12.8l4.2-4.2', 'M5.6 6.6 3.2 4.2 5.4 2l1.8 1.8L8.6 2.4l2.4 2.4-1.4 1.4z'],
    pencil: ['M3 13l1-3.4 6.8-6.8a1.4 1.4 0 0 1 2 2L6 12z', 'M10.2 4.4l1.4 1.4'],
    squareStack: ['M2.5 2.5h6v6h-6z', 'M7.5 7.5h6v6h-6z'],
    textQuote: ['M3 4.8h8.5', 'M3 8h6', 'M3 11.2h4.5'],
    trayFull: ['M2.6 9.4V5.2L4.2 3h7.6l1.6 2.2v4.2', 'M2.6 9.4h3.3l.9 1.4h2.4l.9-1.4h3.3v3.2H2.6z'],
    waveform: ['M2.5 8h1.4', 'M6 4.4v7.2', 'M9.6 6v4', 'M13 7.2v1.6'],
    dot: ['M8 5.4a2.6 2.6 0 1 0 0 5.2 2.6 2.6 0 0 0 0-5.2z'],
    search: ['M7.2 2.8a4.4 4.4 0 1 0 0 8.8 4.4 4.4 0 0 0 0-8.8z', 'M10.5 10.5 13.8 13.8'],
    code: ['M6 4.5 2.6 8 6 11.5', 'M10 4.5 13.4 8 10 11.5'],
    /* The titlebar's plan-folder door (`shell/Titlebar.svelte`): a tab and a
       body, drawn from one outline like `calendar`'s rectangle, so it reads as
       the folder the window is working in rather than a second app mark. */
    folder: ['M2.7 3.9h4.1l1.3 1.7h5.2v7.3H2.7z'],
    /* Settings. A closed 8-tooth outline, **generated** — tip r6.5, root r4.4,
       tooth ±7°, valley ±15° — plus the hole. Do not hand-edit the pairs: the
       first version was written by hand, its x and y came from different
       radii, and the outline zigzagged into a blob that read as a broken icon.
       The family is stroke-only, so the teeth live in the outline itself;
       detached rays are the sun this glyph was replaced for. */
    gear: [
      'M12.25 6.86 12.37 7.46 14.45 7.21 14.45 8.79 12.37 8.54 12.25 9.14 11.81 10.20 11.47 10.71 13.12 12.00 12.00 13.12 10.71 11.47 10.20 11.81 9.14 12.25 8.54 12.37 8.79 14.45 7.21 14.45 7.46 12.37 6.86 12.25 5.80 11.81 5.29 11.47 4.00 13.12 2.88 12.00 4.53 10.71 4.19 10.20 3.75 9.14 3.63 8.54 1.55 8.79 1.55 7.21 3.63 7.46 3.75 6.86 4.19 5.80 4.53 5.29 2.88 4.00 4.00 2.88 5.29 4.53 5.80 4.19 6.86 3.75 7.46 3.63 7.21 1.55 8.79 1.55 8.54 3.63 9.14 3.75 10.20 4.19 10.71 4.53 12.00 2.88 13.12 4.00 11.47 5.29 11.81 5.80Z',
      'M8 5.6a2.4 2.4 0 1 0 0 4.8 2.4 2.4 0 0 0 0-4.8z',
    ],
    plus: ['M8 3.4v9.2', 'M3.4 8h9.2'],
    close: ['M4.2 4.2l7.6 7.6', 'M11.8 4.2l-7.6 7.6'],
    check: ['M3.4 8.4l3 3 6.2-7'],
    refresh: ['M13 8a5 5 0 1 1-1.5-3.6', 'M13.6 2.6v3h-3'],
    chart: ['M3 13V9.4', 'M6.4 13V6', 'M9.8 13V7.8', 'M13.2 13V3.6'],
    play: ['M5.6 3.2 12.6 8l-7 4.8z'],
    route: ['M3.6 11a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2z', 'M12.4 2.8a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2z', 'M5.2 12.6h3.4a3 3 0 0 0 3-3V6'],
    // The machine's own chrome (UI·P3/U6): the System disc, its section tiles,
    // a pipeline's stages, recovery, and the column tree's own two moves.
    cube: ['M8 2.4 12.8 5v6L8 13.6 3.2 11V5z', 'M8 2.4v5.2', 'M3.2 5 8 7.6 12.8 5'],
    layout: ['M2.8 3.4h10.4v9.2H2.8z', 'M6.6 3.4v9.2', 'M6.6 7.6h6.6'],
    stairs: ['M2.6 13h10.8', 'M3.4 13v-2.4h2.8V8h2.8V5.4h3.4'],
    undo: ['M13 8a5 5 0 1 1-1.5-3.6', 'M13.6 2.6v3h-3', 'M2.6 5.6 5.6 8 2.6 10.4'],
    minus: ['M3.4 8h9.2'],
    arrowup: ['M8 12.8V3.4', 'M4.4 7 8 3.4 11.6 7'],
    arrowdown: ['M8 3.2v9.4', 'M4.4 9 8 12.6 11.6 9'],
    /* The composer frame's design switch (`composer/Frame.svelte`): one chevron
       each, drawn at the set's own 16-unit grid, so both stand on the
       component's axis with no text glyph's side bearing to nudge them off it. */
    chevronleft: ['M10 3.2 5.2 8 10 12.8'],
    chevronright: ['M6 3.2 10.8 8 6 12.8'],
    // The rail's theme disc. Each glyph is the mode the press **switches to**,
    // so the pair reads as an instruction and never as a mirror of what is
    // already on screen. The sun is a closed ring with four rays and the moon an
    // open crescent, which is the only distinction they need at 16px. Stroke
    // only, like the rest of the family: the system has exactly one filled
    // shape in it and that is the ticked day (`design.md` §7.4).
    sun: [
      'M8 5.2a2.8 2.8 0 1 0 0 5.6 2.8 2.8 0 0 0 0-5.6z',
      'M8 1.9v1.3M8 12.8v1.3M1.9 8h1.3M12.8 8h1.3M3.6 3.6l.9.9M11.5 11.5l.9.9M12.4 3.6l-.9.9M4.5 11.5l-.9.9',
    ],
    moon: ['M12.9 9.8A5.5 5.5 0 0 1 6.2 3.1 5.5 5.5 0 1 0 12.9 9.8z'],
  };

  /** Every name this build can draw, in the family's own order. */
  export const ICON_NAMES: string[] = Object.keys(GLYPHS);
</script>

<script lang="ts">
  let {
    name,
    size = 16,
  }: { name: string; size?: number } = $props();

  const paths = $derived(GLYPHS[name] ?? []);
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 16 16"
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {#each paths as d (d)}
    <path {d} />
  {/each}
</svg>
