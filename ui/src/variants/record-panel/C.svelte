<!--
  Record panel variant C: The Band.
  Difference chart visualization plotting estimated vs logged minutes and planned vs spent days.
  Entity ID is hidden in accordance with D2.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import Control from './Control.svelte';
  import { CHART, gapBands, roundUp, ticks, type Point } from './chart';
  import {
    dayDistance,
    dayMonth,
    minutes,
    plusDays,
    relative,
    stageWord,
    type RecordPanelProps,
  } from './props';

  let {
    kind,
    address,
    today,
    quantities,
    dates,
    links,
    history,
    logged,
    planned,
    onSet,
    onOpen,
    onReveal,
    onDelete,
  }: RecordPanelProps = $props();

  /** The two editors this plate carries: the estimate and the plan date. */
  let openKey = $state<string | null>(null);

  const estimate = $derived(
    quantities.find((field) => field.type === 'duration' && field.amount !== null) ?? null,
  );
  /** The day the plate is measured to — the kind's own plan date, whether or
   *  not the plan holds one yet (an empty day must still be writable here). */
  const planField = $derived(
    dates.find((field) => field.key === 'focus') ?? dates[0] ?? null,
  );

  function r2(value: number): number {
    return Math.round(value * 100) / 100;
  }

  /** The plate's own numbers: every one of them a stored fact. */
  const data = $derived.by(() => {
    if (!estimate || !planned || !history?.first) return null;
    const from = history.first.slice(0, 10);
    const span = dayDistance(from, planned);
    if (span === null || span <= 0) return null;
    const elapsed = Math.max(0, Math.min(span, dayDistance(from, today) ?? 0));
    const loggedAt =
      logged.on === null
        ? null
        : Math.max(0, Math.min(span, dayDistance(from, logged.on) ?? 0));
    return {
      from,
      span,
      elapsed,
      loggedAt,
      est: estimate.amount ?? 0,
      logged: logged.minutes,
      planKey: planField?.key ?? '',
    };
  });

  /** The plate drawn: the two pairs, their washes, and the round-number grid. */
  const drawing = $derived.by(() => {
    if (!data) return null;
    const { from, span, elapsed, loggedAt, est, logged } = data;
    const x = (day: number): number => CHART.left + (day / span) * (CHART.right - CHART.left);
    const max = roundUp(Math.max(est, logged), 3);
    const yMin = (value: number): number => CHART.mid - (value / max) * (CHART.mid - CHART.top);
    const planAt = (day: number): number => (day / span) * est;
    const loggedY = yMin(Math.min(logged, max));

    const points: Point[] = [{ x: r2(x(0)), a: r2(yMin(0)), b: r2(yMin(0)) }];
    if (loggedAt !== null && loggedAt > 0) {
      points.push({ x: r2(x(loggedAt)), a: r2(yMin(planAt(loggedAt))), b: r2(yMin(0)) });
      points.push({
        x: r2(x(loggedAt)),
        a: r2(yMin(planAt(loggedAt))),
        b: r2(yMin(Math.min(logged, max))),
      });
    }
    points.push({
      x: r2(x(span)),
      a: r2(yMin(est)),
      b: r2(yMin(Math.min(logged, max))),
    });
    const bands = gapBands(points);

    const daysTop = (value: number): number =>
      CHART.bot - (value / Math.max(span, 1)) * (CHART.bot - CHART.days);
    const dayBands = gapBands([
      { x: r2(x(0)), a: r2(daysTop(span)), b: r2(daysTop(elapsed)) },
      { x: r2(x(span)), a: r2(daysTop(span)), b: r2(daysTop(elapsed)) },
    ]);

    return {
      max,
      bands,
      dayBands,
      planLine: `M ${r2(x(0))} ${r2(yMin(0))} L ${r2(x(span))} ${r2(yMin(est))}`,
      logLine:
        loggedAt !== null
          ? `M ${r2(x(0))} ${r2(yMin(0))} L ${r2(x(loggedAt))} ${r2(yMin(0))} L ${r2(x(loggedAt))} ${r2(loggedY)} L ${r2(x(span))} ${r2(loggedY)}`
          : `M ${r2(x(0))} ${r2(yMin(0))} L ${r2(x(span))} ${r2(yMin(0))}`,
      logDot: loggedAt === null ? null : { x: r2(x(loggedAt)), y: r2(loggedY) },
      estDot: { x: r2(x(span)), y: r2(yMin(est)) },
      spanLine: `M ${r2(x(0))} ${r2(daysTop(span))} L ${r2(x(span))} ${r2(daysTop(span))}`,
      elapsedLine: `M ${r2(x(0))} ${r2(daysTop(elapsed))} L ${r2(x(span))} ${r2(daysTop(elapsed))}`,
      spanY: r2(daysTop(span)),
      elapsedY: r2(daysTop(elapsed)),
      grid: ticks(0, max, 3).map((value) => ({ value, y: r2(yMin(value)) })),
      axis: ticks(0, span, 5).map((day) => ({
        day,
        x: r2(x(day)),
        label: day === 0 ? dayMonth(from) : dayMonth(plusDays(from, day)),
      })),
      // The one instant the projection states: when the record was last
      // reviewed. It is annotated where it is, as the lab annotates its rungs.
      reviewed:
        history?.reviewed == null
          ? null
          : {
              x: r2(x(Math.max(0, Math.min(span, dayDistance(from, history.reviewed) ?? 0)))),
            },
    };
  });

  /** One dispatch: the plate is re-inked by the record the write returns. */
  async function write(key: string, value: string | string[] | null): Promise<string | null> {
    const why = await onSet(key, value);
    if (why === null) openKey = null;
    return why;
  }
</script>

<div class="rp-fit rp-panel">
  <section class="rp-vplate" aria-label={`${kind}: measured against the plan`}>
    <div class="rp-vplate__top">
      <span class="rp-rule__k">the gap</span>
      <span class="rp-rule__u">
        {data ? `${dayMonth(data.from)} → ${dayMonth(planned)}` : kind}
      </span>
    </div>

    {#if links.length > 0}
      <div class="rp-tags">
        {#each links as link (link.key)}
          {#each link.targets as target (target.id)}
            {#if target.label}
              <button
                class="rp-term rp-term--tag"
                type="button"
                data-term={target.id}
                aria-label={`Open ${target.label} — the record’s ${link.label.toLowerCase()}`}
                onclick={() => onOpen(target.id, link.to)}
              >
                <span class="rp-term__s">{link.label}</span>
                <span class="rp-term__t">{target.code ?? target.label}</span>
              </button>
            {:else}
              <span class="rp-after">a link to something not here</span>
            {/if}
          {/each}
        {/each}
      </div>
    {/if}

    <svg
      class="rp-chart"
      viewBox={`0 0 ${CHART.w} ${CHART.h}`}
      role="img"
      aria-label="The record’s estimate against the time the plan logged, and its elapsed days against the interval"
    >
      {#if !drawing}
        <rect
          class="rp-axis"
          x={CHART.left}
          y={CHART.top}
          width={CHART.right - CHART.left}
          height={CHART.bot - CHART.top}
          fill="none"
        />
        <text class="rp-tick" x={(CHART.left + CHART.right) / 2} y={(CHART.top + CHART.bot) / 2} text-anchor="middle">
          —
        </text>
      {:else}
        {#each drawing.grid as line (line.value)}
          <line class="rp-grid-line" x1={CHART.left} y1={line.y} x2={CHART.right} y2={line.y} />
          <text class="rp-tick" x={CHART.left - 5} y={line.y + 3} text-anchor="end">{line.value}</text>
        {/each}

        {#if drawing.bands.surplus}<path class="rp-wash-up" d={drawing.bands.surplus} />{/if}
        {#if drawing.bands.deficit}<path class="rp-wash-down" d={drawing.bands.deficit} />{/if}
        <path class="rp-plan-line" d={drawing.planLine} />
        <path class="rp-log-line" d={drawing.logLine} />
        {#if drawing.logDot}
          <circle class="rp-dot" cx={drawing.logDot.x} cy={drawing.logDot.y} r="3" />
        {/if}
        <circle class="rp-dot--open rp-dot" cx={drawing.estDot.x} cy={drawing.estDot.y} r="3" />
        <line class="rp-axis" x1={CHART.left} y1={CHART.mid} x2={CHART.right} y2={CHART.mid} />

        {#if drawing.dayBands.surplus}<path class="rp-wash-up" d={drawing.dayBands.surplus} />{/if}
        {#if drawing.dayBands.deficit}<path class="rp-wash-down" d={drawing.dayBands.deficit} />{/if}
        <path class="rp-span-line" d={drawing.spanLine} />
        <path class="rp-elapsed-line" d={drawing.elapsedLine} />
        <text class="rp-tick" x={CHART.left - 5} y={drawing.spanY + 3} text-anchor="end">{data?.span}d</text>
        <text class="rp-tick" x={CHART.left - 5} y={drawing.elapsedY + 3} text-anchor="end">{data?.elapsed}d</text>

        {#each drawing.axis as mark (mark.day)}
          <line class="rp-axis" x1={mark.x} y1={CHART.bot} x2={mark.x} y2={CHART.bot + 4} />
          <text class="rp-tick" x={mark.x} y={CHART.bot + 14} text-anchor="middle">{mark.label}</text>
        {/each}

        {#if drawing.reviewed}
          <line class="rp-flag-line" x1={drawing.reviewed.x} y1={CHART.top} x2={drawing.reviewed.x} y2={CHART.bot} />
          <text class="rp-flag-t" x={drawing.reviewed.x + 3} y={CHART.bot - 4}>reviewed</text>
        {/if}
      {/if}
    </svg>

    {#if !drawing}
      <p class="rp-after">This kind keeps no pair to measure.</p>
    {/if}

    <div class="rp-legend">
      {#if logged.minutes > 0 || drawing}
        <div class="rp-item rp-item--read">
          <span class="rp-item__s rp-item__s--ink"></span>
          <span class="rp-item__n">logged</span>
          <span class="rp-item__k">session</span>
          <span class="rp-item__ro">
            {logged.minutes === 0
              ? 'nothing logged yet'
              : `${minutes(logged.minutes)}${logged.on ? ` · ${dayMonth(logged.on)}` : ''}`}
          </span>
        </div>
      {/if}

      {#if estimate}
        <div class="rp-item">
          <span class="rp-item__s rp-item__s--plan"></span>
          <span class="rp-item__n">estimated</span>
          <span class="rp-item__k" data-identity>{estimate.key}</span>
          <button
            class="rp-item__v"
            type="button"
            data-command="record.setField"
            data-field-key={estimate.key}
            aria-expanded={openKey === estimate.key}
            aria-label={`${estimate.label}: ${estimate.reading} — activate to edit`}
            onclick={() => (openKey = openKey === estimate.key ? null : estimate.key)}
          >
            {estimate.reading}
          </button>
        </div>
        {#if openKey === estimate.key}
          <Control field={estimate} {address} {today} open onSet={write} />
        {/if}
      {/if}

      {#if drawing && data}
        <div class="rp-item rp-item--read">
          <span class="rp-item__s rp-item__s--ink"></span>
          <span class="rp-item__n">elapsed</span>
          <span class="rp-item__k">interval</span>
          <span class="rp-item__ro">{data.elapsed} of {data.span} days</span>
        </div>
      {/if}

      {#if planField}
        <div class="rp-item">
          <span class="rp-item__s rp-item__s--plan"></span>
          <span class="rp-item__n">planned for</span>
          <span class="rp-item__k" data-identity>{planField.key}</span>
          <button
            class="rp-item__v"
            type="button"
            data-command="record.setField"
            data-field-key={planField.key}
            aria-expanded={openKey === planField.key}
            aria-label={`${planField.label}: ${planField.reading} — activate to edit`}
            onclick={() => (openKey = openKey === planField.key ? null : planField.key)}
          >
            {planField.reading}
          </button>
        </div>
        {#if openKey === planField.key}
          <Control field={planField} {address} {today} open onSet={write} />
        {/if}
      {/if}

      {#if drawing}
        <div class="rp-inkkey">
          <span class="rp-flagin rp-flagin--mint">surplus ink</span>
          <span class="rp-flagin rp-flagin--overdue">shortfall ink</span>
        </div>
      {/if}
    </div>

    {#if history && history.rungs.length > 0}
      <div class="rp-flags">
        {#each history.rungs as rung (rung.name)}
          <span class="rp-flagin" data-state={rung.state === 'done' ? 'done' : 'todo'}>
            <b>{rung.name}</b> {stageWord(rung.state)}
          </span>
        {/each}
      </div>
      {#if history.complete}
        <p class="rp-after">finished — the last stage is passed</p>
      {/if}
      {#if history.asks.length > 0}
        <p class="rp-after">waiting on {history.asks.join(' · ')}</p>
      {/if}
    {/if}

    <div class="rp-acts">
      <button class="rp-ink" type="button" data-command="record.reveal" onclick={onReveal}>
        <Icon name="code" size={13} />
        Edit as JSON
      </button>
      <button class="rp-quiet rp-danger" type="button" data-command="record.delete" onclick={onDelete}>
        <Icon name="close" size={13} />
        Delete this record…
      </button>
    </div>
    <p class="rp-addr" data-developer>{address}</p>
  </section>
</div>

<style>
  /* The plate's own top line: what the band measures, and the surface's design
     switch at the line's right end. */
</style>
