<!--
  Graph / Map record block: renders self-referential relation edges among records
  in a radial layout with hit targets and edge path lines (§3.6).
-->
<script lang="ts">
  import { blockRows, type RecordDoc } from '../types';
  import type { ShapeProps } from '../variants/views/props';

  let { node, facts, selected, onSelect }: ShapeProps = $props();

  const byId = $derived(new Map(facts.map((fact) => [fact.id, fact])));

  /** The plot is a 480×320 viewBox; the element's real size is the CSS one. */
  const VIEW_W = 480;
  const VIEW_H = 320;
  const RADIUS = 120;
  const DOT = 8;
  /** The system's own `--hit`, in the figure's units (a 32-unit disc). */
  const HIT = 16;
  /** Past this many nodes the diagram is noise, and the note below says so. */
  const MAX_NODES = 40;
  const LABEL_CHARS = 14;

  type Point = { record: RecordDoc; label: string; x: number; y: number };

  const records = $derived(blockRows(node));
  const drawn = $derived(records.slice(0, MAX_NODES));

  const points = $derived(
    drawn.map((record, index): Point => {
      // Index 0 sits at the top, the rest clockwise: the engine's order is the
      // reading order, and a circle that starts at the top reads the same twice.
      const angle = (index / Math.max(1, drawn.length)) * Math.PI * 2 - Math.PI / 2;
      return {
        record,
        label: byId.get(record.id)?.label ?? record.id,
        x: VIEW_W / 2 + RADIUS * Math.cos(angle),
        y: VIEW_H / 2 + RADIUS * Math.sin(angle),
      };
    }),
  );

  const placed = $derived(new Map(points.map((point): [string, Point] => [point.record.id, point])));

  /** Relation links whose target is drawn — both ends must be in the picture. */
  const edges = $derived.by(() => {
    const out: Array<{ key: string; from: Point; to: Point }> = [];
    for (const point of points) {
      for (const [relation, targets] of Object.entries(point.record.links ?? {})) {
        for (const target of targets) {
          const other = placed.get(target);
          if (other) {
            out.push({
              key: `${point.record.id}→${target}·${relation}·${out.length}`,
              from: point,
              to: other,
            });
          }
        }
      }
    }
    return out;
  });

  /** What a 480-wide plot fits without two nodes' labels colliding. */
  function short(label: string): string {
    return label.length > LABEL_CHARS ? `${label.slice(0, LABEL_CHARS - 1)}…` : label;
  }

  /** The label's anchor: the side the node stands on, so two can never overlap. */
  function labelAt(point: Point): { x: number; y: number; anchor: 'start' | 'middle' | 'end' } {
    if (point.x < VIEW_W / 2 - 8) {
      return { x: point.x - DOT - 6, y: point.y + 4, anchor: 'end' };
    }
    if (point.x > VIEW_W / 2 + 8) {
      return { x: point.x + DOT + 6, y: point.y + 4, anchor: 'start' };
    }
    return {
      x: point.x,
      y: point.y + (point.y < VIEW_H / 2 ? -DOT - 8 : DOT + 16),
      anchor: 'middle',
    };
  }
</script>

<div class="vw-map cd-scroll">
  <svg
    viewBox={`0 0 ${VIEW_W} ${VIEW_H}`}
    role="group"
    aria-label={`relation graph — ${drawn.length} of ${records.length} records`}
  >
    {#each edges as edge (edge.key)}
      <line class="vw-edge" x1={edge.from.x} y1={edge.from.y} x2={edge.to.x} y2={edge.to.y} />
    {/each}
    {#each points as point (point.record.id)}
      {@const at = labelAt(point)}
      <!-- The group is the control: the circle is 8 units across, too small to
           be a hit target on its own, and the label belongs with it. -->
      <g
        class="vw-node"
        tabindex="0"
        role="button"
        data-record-id={point.record.id}
        aria-pressed={selected === point.record.id}
        aria-label={byId.get(point.record.id)?.sentence ?? point.label}
        onclick={() => onSelect(point.record.id)}
        onkeydown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            onSelect(point.record.id);
          }
        }}
      >
        <circle class="vw-hit" cx={point.x} cy={point.y} r={HIT} />
        <circle class="vw-dot" cx={point.x} cy={point.y} r={DOT} />
        <text class="vw-dotlabel" x={at.x} y={at.y} text-anchor={at.anchor}>
          {short(point.label)}
        </text>
      </g>
    {/each}
  </svg>
</div>
{#if records.length > drawn.length}
  <p class="vw-note">now drawn: first {MAX_NODES} of {records.length}</p>
{/if}

<style>
  /* The figure is capped at its own natural scale and scrolls inside its own
     port when the container is narrower — never the page. */
  .vw-map {
    max-width: min(560px, 100%);
    margin: 0 auto;
    overflow-x: auto;
    overflow-y: hidden;
  }
  .vw-map svg {
    display: block;
    width: 100%;
    min-width: 480px;
    height: auto;
  }
  .vw-edge {
    stroke: var(--rule-strong);
    stroke-width: 1;
    fill: none;
  }
  .vw-hit {
    fill: transparent;
  }
  .vw-dot {
    fill: var(--well-2);
    stroke: var(--rule-strong);
    transition: fill var(--dur-1) var(--ease);
  }
  .vw-node {
    cursor: pointer;
  }
  /* The one mark in ink is the record you are on — the token model's own
     "you are here". */
  .vw-node[aria-pressed='true'] .vw-dot {
    fill: var(--ink);
    stroke: var(--ink);
  }
  .vw-dotlabel {
    font-family: var(--font-body);
    font-size: var(--text-2xs);
    fill: var(--ink-2);
  }
  .vw-node:focus-visible {
    outline: none;
  }
  .vw-node:focus-visible .vw-dot {
    stroke: var(--focus);
    stroke-width: 2.5;
  }
  .vw-note {
    margin-top: var(--ui-gap-sm);
    text-align: center;
    font-size: var(--ui-text-sm);
    color: var(--ink-3);
  }

  @media (prefers-reduced-motion: reduce) {
    .vw-dot {
      transition: none;
    }
  }
</style>
