<!--
  WEEK-CHART · THE HOST — the seven days as a widget the composer mounts
  (2026-09-29).

  This is the surface's own derivation moved out of `panels/Today.svelte`, not
  re-written: the same `todayFacts.days` verbatim, the same week total, and the
  same two strings the screen chose — the heading and its one-line fact. It
  takes no props and hands `<Variant>` exactly the object the panel used to
  spread.

  The caption is Today's own wording (`This week`); Progress mounts the same
  surface with its own heading and a host of its own. Nothing is ephemeral and
  no write belongs to this surface; the read is `app.todayFacts`, and nothing
  here touches `app.outcome`.
-->
<script lang="ts">
  import Variant from '../../../variants/Variant.svelte';
  import type { WeekChartProps } from '../../../variants/week-chart/props';
  import { app } from '../../../session.svelte';
  import { durationText } from '../../../types';

  const days = $derived(app.todayFacts?.days ?? []);
  const totals = $derived(app.todayFacts?.totals ?? null);
  const weekTotal = $derived(days.reduce((sum, day) => sum + day.loggedMin, 0));

  const chartProps = $derived<WeekChartProps>({
    title: 'This week',
    caption: `${durationText(weekTotal)} across the last seven days`,
    days,
    targetMin: totals?.targetMin ?? null,
  });
</script>

<Variant surface="week-chart" {...chartProps} />
