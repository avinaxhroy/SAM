<!-- Record panel variant B: The Counter Bank. Mechanical counter wheels for quantity, duration, and dates (D2). -->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import IdPair from '../../shell/IdPair.svelte';
  import Control from './Control.svelte';
  import Drum from './Drum.svelte';
  import { dayMonth, minutes, relative, stamp, stageWord, type FieldFact, type RecordPanelProps } from './props';

  let {
    kind,
    address,
    today,
    quantities,
    dates,
    links,
    history,
    logged,
    onSet,
    onOpen,
    onReveal,
    onDelete,
  }: RecordPanelProps = $props();

  /** One wheel of a counter: a drum, its unit, and what its key line says. */
  type Cell = {
    id: string;
    key: string;
    field: FieldFact | null;
    text: string;
    unit: string;
    read: boolean;
    mark: string;
    aria: string;
  };
  type Counter = { id: string; cells: Cell[] };

  /** The one editor that is open, if any — opening one folds the other. */
  let openKey = $state<string | null>(null);
  /** How many writes have landed at each value: the drum rolls on a write, and
   *  nothing else makes it move. */
  let rolls = $state<Record<string, number>>({});

  const MONTH_NAMES = ['JAN', 'FEB', 'MAR', 'APR', 'MAY', 'JUN', 'JUL', 'AUG', 'SEP', 'OCT', 'NOV', 'DEC'];

  function cellOf(field: FieldFact): Cell {
    return {
      id: field.key,
      key: field.key,
      field,
      text: field.amount === null ? '' : String(field.amount),
      unit: field.type === 'duration' ? 'min' : '',
      read: false,
      mark: '',
      aria: `${field.label}: ${field.reading}`,
    };
  }

  /** The bank: the kind's own columns, window by window. */
  const counters = $derived.by<Counter[]>(() => {
    const out: Counter[] = [];
    const numbers = quantities.filter((field) => field.type === 'number' && field.amount !== null);
    // `solved · attempted · total` are one question between them, so they share
    // one window — the columns the kind declares, never a rewritten pair.
    const shared = ['solved', 'attempted', 'total'].filter((key) =>
      numbers.some((field) => field.key === key),
    );
    const questions = shared
      .map((key) => numbers.find((field) => field.key === key))
      .filter((field): field is FieldFact => field !== undefined);
    if (questions.length > 0) out.push({ id: 'questions', cells: questions.map(cellOf) });
    for (const field of numbers) {
      if (shared.includes(field.key)) continue;
      out.push({ id: field.key, cells: [cellOf(field)] });
    }
    for (const field of quantities) {
      if (field.type === 'duration') {
        const est = cellOf(field);
        const cells: Cell[] = [{ ...est, unit: 'min' }];
        if (logged.minutes > 0) {
          cells.push({
            id: 'logged',
            key: 'logged',
            field: null,
            text: String(logged.minutes),
            unit: 'min',
            read: true,
            mark: 'sum',
            aria: `time the plan logged: ${minutes(logged.minutes)}${logged.on ? ` on ${dayMonth(logged.on)}` : ''}`,
          });
        }
        out.push({ id: field.key, cells });
      } else if (field.type === 'formula' || field.type === 'progress') {
        // A computed column whose value the engine did not hand back reads
        // `not worked out yet` — it gets no drum, and says so where it stands.
        if (field.amount === null) continue;
        out.push({
          id: field.key,
          cells: [
            {
              id: field.key,
              key: field.key,
              field,
              text: field.reading,
              unit: '',
              read: true,
              mark: 'ƒ',
              aria: `${field.label}: ${field.reading} — computed by the kind`,
            },
          ],
        });
      }
    }
    for (const field of dates) {
      const hasDay = field.raw.length >= 10;
      const day = hasDay ? Number(field.raw.slice(8, 10)) : null;
      const month = hasDay ? MONTH_NAMES[Number(field.raw.slice(5, 7)) - 1] : '';
      out.push({
        id: field.key,
        cells: [
          {
            id: field.key,
            key: field.key,
            field,
            text: day === null ? '' : String(day),
            unit: month ?? '',
            read: false,
            mark: '',
            aria: hasDay
              ? `${field.label}: ${stamp(field.raw)}, ${relative(field.raw, today)}`
              : `${field.label}: empty`,
          },
        ],
      });
    }
    return out;
  });

  /** One dispatch, then the wheel for that value rolls to what the write left. */
  async function write(key: string, value: string | string[] | null): Promise<string | null> {
    const why = await onSet(key, value);
    if (why === null) {
      rolls = { ...rolls, [key]: (rolls[key] ?? 0) + 1 };
      openKey = null;
    }
    return why;
  }
</script>

<div class="rp-fit rp-panel">
  <section class="rp-bank" aria-label={`${kind}: a bank of counters`}>
    <div class="rp-bank__id">
      <span class="rp-bank__s">{kind}</span>
    </div>

    {#if links.length > 0}
      <div class="rp-bank__tags">
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
              <span class="rp-term--tag rp-after">a link to something not here</span>
            {/if}
          {/each}
        {/each}
      </div>
    {/if}

    <div class="rp-bank__grid">
      {#if counters.length === 0}
        <p class="rp-bank__none">This kind keeps no quantity for a counter.</p>
      {:else}
        {#each counters as counter (counter.id)}
          <div class="rp-counter" data-span={counter.cells.length >= 2 ? '2' : '1'}>
            <div class="rp-counter__win">
              {#each counter.cells as cell (cell.id)}
                {#if cell.field && !cell.read && !cell.field.derived}
                  <button
                    class="rp-wheel"
                    type="button"
                    data-command="record.setField"
                    data-field-key={cell.key}
                    aria-expanded={openKey === cell.key}
                    aria-label={`${cell.aria} — activate to edit`}
                    onclick={() => (openKey = openKey === cell.key ? null : cell.key)}
                  >
                    {#if cell.text === ''}
                      <span class="rp-wheel__none">—</span>
                    {:else}
                      <Drum text={cell.text} roll={rolls[cell.key] ?? 0} />
                    {/if}
                    {#if cell.unit}<span class="rp-wheel__u">{cell.unit}</span>{/if}
                  </button>
                {:else}
                  <span class="rp-wheel" role="img" aria-label={cell.aria}>
                    {#if cell.text === ''}
                      <span class="rp-wheel__none">—</span>
                    {:else}
                      <Drum text={cell.text} roll={rolls[cell.key] ?? 0} />
                    {/if}
                    {#if cell.unit}<span class="rp-wheel__u">{cell.unit}</span>{/if}
                  </span>
                {/if}
              {/each}
            </div>
            <div class="rp-counter__keys">
              {#each counter.cells as cell (cell.id)}
                <span class="rp-counter__key">
                  {#if cell.field}
                    <IdPair label={null} value={cell.key} />
                  {:else}
                    <span>{cell.key}</span>
                  {/if}
                  {#if cell.mark}<span class="rp-fn">{cell.mark}</span>{/if}
                </span>
              {/each}
            </div>
            {#each counter.cells as cell (cell.id)}
              {#if cell.field && !cell.read && !cell.field.derived}
                <Control
                  field={cell.field}
                  {address}
                  {today}
                  open={openKey === cell.key}
                  onSet={write}
                />
              {/if}
            {/each}
          </div>
        {/each}
      {/if}
    </div>

    {#if history}
      <div class="rp-hist">
        <div class="rp-hist__row">
          <div class="rp-hist__c">
            <span class="rp-hist__k">first</span>
            <span class="rp-hist__v">
              {history.first ? `${dayMonth(history.first)} · ${relative(history.first, today)}` : '—'}
            </span>
          </div>
          <div class="rp-hist__c">
            <span class="rp-hist__k">reviewed</span>
            <span class="rp-hist__v">
              {history.reviewed
                ? `${dayMonth(history.reviewed)} · ${history.rating ?? relative(history.reviewed, today)}`
                : 'not yet'}
            </span>
          </div>
          <div class="rp-hist__c">
            <span class="rp-hist__k">next</span>
            <span class="rp-hist__v">
              {history.due
                ? `${dayMonth(history.due)} · ${relative(history.due, today)}${history.late > 0 ? ` · ${history.late} days late` : ''}`
                : 'nothing yet'}
            </span>
          </div>
        </div>
        {#if history.rungs.length > 0}
          <ol class="rp-rungs">
            {#each history.rungs as rung (rung.name)}
              <li class="rp-rung" data-state={rung.state}>
                <span class="rp-rung__n">{rung.name}</span>
                <span class="rp-rung__s">{stageWord(rung.state)}</span>
              </li>
            {/each}
          </ol>
        {/if}
        {#if history.complete}
          <p class="rp-after">finished — the last stage is passed</p>
        {/if}
        {#if history.asks.length > 0}
          <p class="rp-after">waiting on {history.asks.join(' · ')}</p>
        {/if}
      </div>
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
  /* The bank's own identity line: the kind stamps it, and the surface's design
     switch takes the line's right end. */
</style>
