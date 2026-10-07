<!--
  Calendar record block: renders a monthly grid of records mapped to their
  date fields, displaying date markers and an undated record tally.
-->
<script lang="ts">
  import { blockRows, type RecordDoc } from '../types';
  import type { ShapeProps } from '../variants/views/props';

  let { node, facts, selected, onSelect }: ShapeProps = $props();

  const byId = $derived(new Map(facts.map((fact) => [fact.id, fact])));

  const records = $derived(blockRows(node));

  const MONTHS = [
    'January', 'February', 'March', 'April', 'May', 'June',
    'July', 'August', 'September', 'October', 'November', 'December',
  ];
  const DOW = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

  /** `YYYY-MM-DD` from local parts — never `toISOString`, which shifts the day. */
  function dayKey(date: Date): string {
    const parts = [date.getFullYear(), date.getMonth() + 1, date.getDate()].map((part) =>
      String(part).padStart(2, '0'),
    );
    return `${parts[0]}-${parts[1]}-${parts[2]}`;
  }

  const todayKey = dayKey(new Date());

  /** The engine's set, bucketed by the day each record's own date names. */
  const byDay = $derived.by(() => {
    const buckets = new Map<string, RecordDoc[]>();
    for (const record of records) {
      const day = byId.get(record.id)?.day ?? null;
      if (day === null) continue;
      const bucket = buckets.get(day);
      if (bucket) bucket.push(record);
      else buckets.set(day, [record]);
    }
    return buckets;
  });

  /** Count of records without dates that cannot be placed on the grid. */
  const undated = $derived(facts.filter((fact) => fact.day === null).length);

  /** The month the grid is paged from — today's, held once, not re-read per cell. */
  const anchor = new Date();
  let month = $state(0);
  const shown = $derived(new Date(anchor.getFullYear(), anchor.getMonth() + month, 1));

  /** Whole weeks from the Monday of the 1st to the Sunday that closes the month. */
  const cells = $derived.by(() => {
    const lead = (shown.getDay() + 6) % 7;
    const length = new Date(shown.getFullYear(), shown.getMonth() + 1, 0).getDate();
    const total = Math.ceil((lead + length) / 7) * 7;
    return Array.from({ length: total }, (_, index) => {
      const date = new Date(shown.getFullYear(), shown.getMonth(), 1 - lead + index);
      const key = dayKey(date);
      return {
        key,
        num: date.getDate(),
        out: date.getMonth() !== shown.getMonth(),
        today: key === todayKey,
        records: byDay.get(key) ?? [],
      };
    });
  });
</script>

<div class="vw-calhead">
  <button class="cd-iconbtn" type="button" aria-label="previous month" onclick={() => (month -= 1)}>
    ‹
  </button>
  <span class="vw-calmonth">{MONTHS[shown.getMonth()]} {shown.getFullYear()}</span>
  <button class="cd-iconbtn" type="button" aria-label="next month" onclick={() => (month += 1)}>
    ›
  </button>
  <span class="vw-spacer"></span>
  <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button" onclick={() => (month = 0)}>
    Today
  </button>
</div>

<div class="vw-cal">
  {#each DOW as day (day)}
    <span class="vw-dow">{day}</span>
  {/each}
  {#each cells as cell (cell.key)}
    <div
      class="vw-day"
      data-out={cell.out ? 'true' : undefined}
      data-today={cell.today ? 'true' : undefined}
    >
      <span class="vw-dnum">{cell.num}</span>
      {#each cell.records.slice(0, 3) as record (record.id)}
        {@const fact = byId.get(record.id)}
        <button
          class="vw-item"
          type="button"
          data-record-id={record.id}
          aria-pressed={selected === record.id}
          title={fact?.label}
          aria-label={fact?.sentence ?? ''}
          onclick={() => onSelect(record.id)}
        >
          <span>{fact?.label}</span>
        </button>
      {/each}
    </div>
  {/each}
</div>

{#if undated > 0}
  <p class="vw-note">{undated} records have no date</p>
{/if}

<style>
  .vw-calhead {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
    margin-bottom: var(--ui-gap-sm);
  }
  .vw-calmonth {
    font-size: var(--ui-text);
    font-weight: var(--weight-display);
    letter-spacing: var(--track-title);
  }
  .vw-spacer {
    flex: 1;
  }
  .vw-cal {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: var(--space-2xs);
    min-width: 0;
  }
  .vw-dow {
    font-size: var(--text-2xs);
    letter-spacing: var(--track-caps);
    color: var(--ink-3);
    text-transform: uppercase;
    text-align: center;
  }
  /* A fixed height, so a month cannot grow with its data. */
  .vw-day {
    min-width: 0;
    min-height: calc(76px * var(--ui-s));
    padding: var(--space-2xs);
    border-radius: var(--r-item);
    background: var(--well);
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-3xs);
    align-content: start;
  }
  /* An adjacent month's day keeps its number at full strength on no well at
     all: the cell reads as "not this month" by having no ground, and the
     numeral stays `--ink-3`. */
  .vw-day[data-out='true'] {
    background: transparent;
  }
  .vw-day[data-today='true'] {
    outline: 1px solid var(--rule-strong);
  }
  .vw-dnum {
    font-size: var(--text-2xs);
    color: var(--ink-3);
    font-variant-numeric: tabular-nums;
  }
  /* The item is a button and it is at the floor: 32px, not the app's 19px. */
  .vw-item {
    display: flex;
    align-items: center;
    width: 100%;
    min-width: 0;
    min-height: var(--hit);
    padding: 0 var(--space-2xs);
    border-radius: var(--r-item);
    background: var(--card);
    color: var(--ink);
    font-size: var(--text-2xs);
    text-align: left;
    overflow: hidden;
    transition: box-shadow var(--dur-1) var(--ease);
  }
  .vw-item > span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .vw-item:hover {
    box-shadow: var(--sh-1);
  }
  .vw-item:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .vw-item[aria-pressed='true'] {
    box-shadow: inset 0 0 0 2px var(--ink);
  }
  .vw-note {
    margin-top: var(--ui-gap-sm);
    font-size: var(--ui-text-sm);
    color: var(--ink-3);
  }

  /* A narrow container: the day keeps its numeral and its items, and the cell
     gets shorter rather than the grid scrolling sideways. */
  @container (max-width: 520px) {
    .vw-day {
      min-height: calc(64px * var(--ui-s));
      padding: var(--space-3xs);
    }
    .vw-day .vw-item {
      font-size: var(--text-2xs);
      padding: 0 var(--space-3xs);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .vw-item {
      transition: none;
    }
  }
</style>
