<!--
  COMING-UP · THE HOST — the dated rows as a widget the composer mounts
  (2026-09-29).

  This is the surface's own derivation moved out of `panels/Today.svelte`, not
  re-written: the same `todayFacts.upcoming` slice, the same naming and date
  work in the surface's own `rowsOf`, and the same two doors — `rail.select` to
  the plan (offered only when the plan's view is known) and `record.panel` for a
  row. It takes no props and hands `<Variant>` exactly the object the panel used
  to spread.

  Nothing is ephemeral here: this card reads and navigates, it writes nothing.
  The read is `app.todayFacts` and the plan lookup is `app.views`; nothing here
  touches `app.outcome`.
-->
<script lang="ts">
  import Variant from '../../../variants/Variant.svelte';
  import { rowsOf, type ComingUpProps } from '../../../variants/coming-up/props';
  import { app } from '../../../session.svelte';
  import { todayGroup } from '../../../types';

  const upcoming = $derived(todayGroup(app.todayFacts, 'upcoming')?.items ?? []);

  /** The dated rows: the screen's own naming and date work live in the
      surface's `rowsOf`, so Today and Mocks state a row the same way. */
  const comingUpRows = $derived(rowsOf(upcoming));

  const planView = $derived(
    Object.entries(app.views?.views ?? {}).find(([, def]) => def.panel === 'plan')?.[0] ?? null,
  );

  async function openPlan(): Promise<void> {
    if (planView) await app.run('rail.select', { view: planView });
  }

  /** The record door for a dated row: the app's own `record.panel`. */
  function openDated(id: string): void {
    const item = upcoming.find((row) => row.id === id);
    if (item) void app.run('record.panel', { id, type: item.kind });
  }

  const comingUpProps = $derived<ComingUpProps>({
    rows: comingUpRows,
    planReady: Boolean(planView),
    onOpenPlan: () => void openPlan(),
    onOpen: openDated,
  });
</script>

<Variant surface="coming-up" {...comingUpProps} />
