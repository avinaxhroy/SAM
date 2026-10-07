# Porting Redesign Variants into the App

Surfaces live in \`ui/src/variants/<surface>/\` as three Svelte components (A, B, C) mounted via \`<Variant surface="<surface>">\`.

## 1 · Architecture

- \`ui/src/variants/catalog.ts\`: Surface definitions, variant options, and default styles.
- \`ui/src/variants/Variant.svelte\`: Host component mounting selected variant.
- \`ui/src/variants/registry.ts\`: Dynamic glob loader (\`import.meta.glob('./*/index.ts')\`).
- \`ui/src/variants/styles.svelte.ts\`: Reactive preferences (\`variantOf\`, \`setVariant\`, \`size\`).
- \`ui/src/variants/variants.css\`: Dynamic size tokens (\`--ui-s\`, \`--ui-row-h\`, \`--ui-pad\`, \`--ui-gap\`, \`--ui-text\`).

## 2 · Surface Contract

Each surface provides:
\`\`\`text
ui/src/variants/<surface>/index.ts     Component map and shared Props contract
ui/src/variants/<surface>/A.svelte     Variant A
ui/src/variants/<surface>/B.svelte     Variant B (single-design surfaces: A only)
ui/src/variants/<surface>/C.svelte     Variant C
\`\`\`

\`index.ts\`:
\`\`\`ts
import type { VariantComponents } from '../registry';
import A from './A.svelte';
import B from './B.svelte';
import C from './C.svelte';

export type Props = {
  items: Item[];
  opened: string | null;
  onOpen: (id: string) => void;
  onClose: () => void;
};

export default { a: A, b: B, c: C } satisfies VariantComponents;
\`\`\`

Mounting in panel:
\`\`\`svelte
<Variant surface="courses" {subjects} opened={opened} onOpen={(id) => (opened = id)} />
\`\`\`

**State ownership:** Panels own and fetch data; variants are purely presentational receivers of props and callbacks.

## 3 · Implementation Rules

1. **Markup & CSS only:** Replace hard-coded lab records with props. Format strings via app helpers (\`recordLabel\`, \`nameOf\`, \`durationText\`). Raw IDs, keys, or ISO dates are forbidden.
2. **Preserve \`data-command\`:** Retain all \`data-command\` attributes from original screens to pass \`tools/uicheck-diff.mjs\` parity gates.
3. **Uniform props:** All three variants share the same \`Props\` interface. Variant-specific UI state must not use hidden global stores.
4. **No independent data fetching:** Never invoke \`readPlanModel\`, \`app.recordsOf\`, or \`app.run\` directly inside variants.
5. **Accessibility:** Semantic HTML elements, accessible names, ARIA states (\`aria-expanded\`, \`aria-pressed\`), keyboard focus, hit targets >= 32px (\`--hit\`), and \`prefers-reduced-motion\` support.
6. **Token compliance:** Zero literal colors, radii, or durations; use \`design/tokens.css\` (\`node tools/tokenlint.mjs\` must pass).
7. **Keyed \`{#each}\`:** Keys must be globally unique per collection to prevent render errors.
8. **Ink budget:** Maximum one solid-ink container per screen.

## 4 · Responsive Layout & Sizing

- **Container queries:** Use \`container-type: inline-size\` on wrapper and \`@container\` rules. Avoid containment on elements with \`position: fixed\` children.
- **Fluid sizing:** Measure relative to container (\`width: 100%\`, \`max-width: min(720px, 100%)\`).
- **Scale register:** Layout dimensions must reference \`var(--ui-*)\` or \`calc(<n>px * var(--ui-s))\`.
- **Reflow:** At < 520px inline width, right-hand clusters stack and columns fold without horizontal page overflow.

## 5 · Verification

\`\`\`bash
pnpm -C ui exec vite build --outDir /tmp/<unit>-dist
cd ui && npx tsc --noEmit -p tsconfig.json && cd ..
node tools/seed-plan.mjs --plan /tmp/<unit>-plan --port <PORT>
node tools/web-ipc.mjs --plan /tmp/<unit>-plan --port <PORT> --dist /tmp/<unit>-dist
node tools/shot-variants.mjs --port <PORT> --variant a --dest <DEST> --out .captures/ports/<unit>
node tools/shot-variants.mjs --port <PORT> --variant b --dest <DEST> --out .captures/ports/<unit>
node tools/shot-variants.mjs --port <PORT> --variant c --dest <DEST> --out .captures/ports/<unit>
node tools/gates.mjs --plan /tmp/<unit>-plan --port <PORT> --out .captures/ports/<unit>-gates
\`\`\`
