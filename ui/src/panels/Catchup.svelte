<!--
  COME BACK AFTER A GAP (F13, `UX_FLOWS.md` §3.4; UI P3 / U6).
  Sheet for rescheduling an overdue backlog. Dispatches `record.defer` in one
  undoable transaction. Items derive course code, wash, and lateness from
  `today.view`'s late group.
-->
<script lang="ts">
  import { app } from '../session.svelte';
  import { todayGroup } from '../types';
  import Variant from '../variants/Variant.svelte';
  import { datesFrom, lateness, type CatchupItem } from '../variants/sheet-catchup/props';

  let {
    items,
    label,
    onclose,
  }: { items: Array<{ id: string; label: string }>; label: string; onclose: () => void } = $props();

  /** The plan's own today. The engine owns the timezone, so nothing is guessed. */
  const today = $derived(app.todayFacts?.date ?? app.today?.date ?? null);
  const dates = $derived(today ? datesFrom(today) : []);
  /** The day's own late group — the same read the Today screen draws. */
  const late = $derived(todayGroup(app.todayFacts, 'late')?.items ?? []);

  /** The backlog, each record carrying at most two facts: its course and how
      late it is. Nothing is invented for a record the projection omits. */
  const backlog = $derived<CatchupItem[]>(
    items.map((item) => {
      const fact = late.find((row) => row.id === item.id);
      return {
        id: item.id,
        label: fact?.label ?? item.label,
        course: fact?.course?.label ?? null,
        wash: fact?.course?.wash ?? null,
        late: fact && fact.lateDays > 0 ? lateness(fact.lateDays) : null,
      };
    }),
  );

  /**
   * The one write. It answers whether the batch landed, because a design shows
   * its receipt only for a write that happened; a refusal stays in the surface
   * the student is looking at (§11) rather than becoming a toast behind it.
   */
  async function commit(ids: string[], iso: string): Promise<boolean> {
    const result = await app.run('record.defer', { ids, date: iso, reviews: true });
    if (!result) {
      app.notice(app.toast ?? 'the move was refused');
      app.toast = null;
      return false;
    }
    const data = result.data as { moved?: number; reviews?: number } | undefined;
    const count = data?.moved ?? ids.length;
    const when = dates.find((date) => date.iso === iso)?.full ?? 'the day you chose';
    app.notice(
      `${count} ${count === 1 ? 'record' : 'records'} moved to ${when}` +
        ((data?.reviews ?? 0) > 0
          ? ` — ${data?.reviews} review${(data?.reviews ?? 0) === 1 ? '' : 's'} came with ${count === 1 ? 'it' : 'them'}`
          : ''),
      { label: 'Undo', run: () => void app.undo() },
    );
    return true;
  }

  /** Undoes the deferral transaction. */
  function undo(): void {
    void app.undo();
  }
</script>

{#if dates.length > 0}
  <Variant
    surface="sheet-catchup"
    title="Move a backlog"
    note={label}
    items={backlog}
    {dates}
    onCommit={commit}
    onUndo={undo}
    onClose={onclose}
  />
{:else}
  <!-- The plan has not said what today is yet (a failed `today.view`): there is
       no date to move anything to, and a sheet with invented offsets would be a
       write to the wrong day. One sentence, no surface. -->
  <p class="cd-sr" role="status">The plan has not said what today is yet.</p>
{/if}
