<!--
  TODAY-QUEUE · THE HOST — the day's queue as a widget the composer mounts
  (2026-09-29).

  This is the surface's own derivation moved out of `panels/Today.svelte`, not
  re-written: the same `QUEUE` order, the same walk over `todayFacts` groups
  (rows and their `count`/`sent` pair), the same five writes — Start,
  Stop, Log, Plan for today, Take off — and the same `today.add` sheet door.
  It takes no props and hands `<Variant>` exactly the object the panel used to
  spread.

  THE ONE THING IT SHARES. `targetMin` lives in `composer/shared.svelte.ts` —
  the length a Start or a Log from this card will use, and the number the
  duration capsule draws, is one value with the session card's own capsule, so
  the two widgets never disagree about how long a session runs.

  THE READ. Rows come from `app.todayFacts` and the running timer from
  `app.timer`; nothing here touches `app.outcome` or issues a read of its own.
-->
<script lang="ts">
  import Variant from '../../../variants/Variant.svelte';
  import type { Props as TodayQueueProps } from '../../../variants/today-queue/props';
  import { app } from '../../../session.svelte';
  import { shared } from '../../shared.svelte';
  import { todayGroup, type TodayGroupId } from '../../../types';

  /**
   * The queue's order, and the labels the screen states it with. Committed work
   * comes first because the student put it there; the rest is the engine's
   * declared order (late · due · next · stale).
   */
  const QUEUE: Array<{ id: TodayGroupId; label: string }> = [
    { id: 'committed', label: 'Planned for today' },
    { id: 'late', label: 'Late for review' },
    { id: 'due', label: 'Due today' },
    { id: 'next', label: 'Next in the plan' },
    { id: 'stale', label: 'Not touched in a while' },
  ];

  const rows = $derived(
    QUEUE.flatMap((group) =>
      (todayGroup(app.todayFacts, group.id)?.items ?? []).map((item) => ({
        item,
        group: group.id,
        label: group.label,
      })),
    ),
  );
  const counts = $derived(
    QUEUE.map((group) => {
      const fact = todayGroup(app.todayFacts, group.id);
      return { ...group, count: fact?.count ?? 0, sent: fact?.items.length ?? 0 };
    }),
  );

  /**
   * The write a Start performs: the plan's own time-logging kind. A plan
   * without one says so instead of writing to a guessed field.
   */
  const sessionCommand = $derived(app.sessionKind() ? `${app.sessionKind()}.new` : null);

  const queueProps = $derived<TodayQueueProps>({
    rows,
    groups: counts,
    sessionCommand,
    targetMin: shared.targetMin,
    runningId: app.timer?.item.id ?? null,
    runningSince: app.timer?.startedAt ?? null,
    onStart: (item) => void app.startSession(item, shared.targetMin),
    onStop: () => app.stopSession(),
    onLog: (item) => void app.logTime(item, shared.targetMin),
    onAdd: (item) => void app.addToToday(item.id),
    onRemove: (item) => void app.removeFromToday(item.id),
    onAddToToday: () => void app.openSheet({ kind: 'today.add' }),
  });
</script>

<Variant surface="today-queue" {...queueProps} />
