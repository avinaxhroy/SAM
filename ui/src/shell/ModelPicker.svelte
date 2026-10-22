<!--
  The model picker (§4.7's model pill; UI_SPEC "model pill").

  What is *model* about this file is only the content: rows grouped by the
  provider the id or label already carries (`provider/model`, `model · provider`),
  *Default* pinned first, *Custom…* last, and the line that says where the list
  came from. The listbox itself — search, arrows, Escape, the card that opens
  upward over the page — is `DeckPicker.svelte`, shared with the CLI pill beside
  this one.

  The list is read lazily: opening the card is what asks the CLI
  (`onOpen` → `agent.ensureModels`), so the deck itself never waits on
  `opencode models`' network round-trip.
-->
<script lang="ts">
  import DeckPicker from './DeckPicker.svelte';
  import type { PickGroup } from './picker';
  import type { AgentModel } from '../ipc';

  let {
    label,
    value,
    models,
    source,
    loading = false,
    disabled = false,
    onPick,
    onOpen,
  }: {
    /** What the closed pill shows (the parent owns how a choice reads). */
    label: string;
    /** `default` · a model id · `__custom__`. */
    value: string;
    models: AgentModel[];
    source: 'live' | 'fallback' | 'profile' | 'none' | null;
    loading?: boolean;
    disabled?: boolean;
    onPick: (option: string) => void;
    onOpen?: () => void;
  } = $props();

  const CUSTOM = '__custom__';

  /** The provider an id or label already names — the row's only grouping. */
  function providerOf(model: AgentModel): string {
    const slash = model.id.indexOf('/');
    if (slash > 0) return model.id.slice(0, slash);
    const dot = model.label.indexOf(' · ');
    if (dot > 0) return model.label.slice(dot + 3);
    return '';
  }

  /** CLI order, grouped by provider (first-seen first), ids kept as the note. */
  const groups = $derived.by((): PickGroup[] => {
    const order: string[] = [];
    const byProvider = new Map<string, PickGroup['rows']>();
    for (const model of models) {
      const provider = providerOf(model);
      if (!byProvider.has(provider)) {
        byProvider.set(provider, []);
        order.push(provider);
      }
      byProvider
        .get(provider)!
        .push({ id: model.id, label: model.label, note: model.label === model.id ? undefined : model.id });
    }
    return order.map((name) => ({ name, rows: byProvider.get(name)! }));
  });

  /** Where the rows came from — a list the student can trust or work around. */
  const hint = $derived.by(() => {
    if (loading) return 'Reading the models this CLI reports…';
    if (source === 'fallback') return 'The CLI’s documented set — Custom… takes anything newer.';
    if (source === 'live') return 'Read from the CLI itself.';
    if (source === 'profile') return 'The ids this profile declares.';
    if (models.length === 0) return 'This CLI lists no models — Custom… takes any id it accepts.';
    return '';
  });
</script>

<DeckPicker
  icon="cube"
  {label}
  {value}
  {groups}
  {hint}
  {disabled}
  {onPick}
  {onOpen}
  first={{ id: 'default', label: 'Default', note: 'the CLI’s own choice' }}
  last={{ id: CUSTOM, label: 'Custom…', note: 'any id the CLI accepts' }}
  placeholder="Search models…"
  listLabel="Which model the agent should use"
/>
