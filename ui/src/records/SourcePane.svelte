<!--
  Source pane inspector and direct editor (Appendix C.4, §4.1, §4.7, §4.8).
  Direct editor for plan source JSON and record JSONL documents.
-->
<script lang="ts">
  import { glyph, shortcutFor } from '../commands/registry';
  import { dispatch, type DispatchResult } from '../ipc';
  import { app, type SourceTarget } from '../session.svelte';
  import { recordLabel } from '../types';
  import Variant from '../variants/Variant.svelte';
  import type {
    SourceFinding,
    SourceIdentity,
    SourceOption,
    SourceStatus,
  } from '../variants/source-pane/props';
  import '../styles/source-pane.css';

  type SourceRead = { kind: 'record' | 'file'; file: string; id?: string; line?: number; text: string };

  let { os = '' }: { os?: string } = $props();

  // Platform glyph resolution (D14).
  const platform = $derived(os !== '' ? os : (document.documentElement.dataset.os ?? 'mac'));
  const commitKey = $derived(glyph('mod+enter', platform));
  const toggleKey = $derived(
    shortcutFor(app.keybindings, 'app.toggleSourcePane', platform) ?? glyph('mod+shift+j', platform),
  );

  let files = $state<string[]>([]);
  let target = $state<SourceTarget | null>(null);
  let file = $state('');
  let text = $state('');
  let findings = $state<SourceFinding[]>([]);
  let loading = $state(false);
  let status = $state<SourceStatus>('idle');
  let receipt = $state<string | null>(null);

  let committing = false;
  let seenRevision: string | null = null;
  let listed = false;
  // Currently flagged cell in table (§4.8 P5).
  let flagged: Element | null = null;
  let checking: number | undefined;
  let finishing: number | undefined;

  // ── target selection (Appendix C.4) ──────────────────────────────────
  function same(a: SourceTarget, b: SourceTarget): boolean {
    if (a.kind === 'record' && b.kind === 'record') return a.id === b.id;
    if (a.kind === 'file' && b.kind === 'file') return a.file === b.file;
    return false;
  }

  function keyOf(point: SourceTarget): string {
    return point.kind === 'record' ? `record:${point.id}` : `file:${point.file}`;
  }

  // Target precedence (Appendix C.4): active draft > explicit document pick > current selection.
  function wanted(): SourceTarget | null {
    if (app.draft) return app.draft.target;
    if (app.paneFile) return { kind: 'file', file: app.paneFile };
    if (target && target.kind === 'file') return target;
    if (app.selection) return { kind: 'record', id: app.selection };
    return target;
  }

  const pointed = $derived(app.draft?.target ?? target);
  const external = $derived(app.draft?.external === true);

  // ── identity & options ───────────────────────────────────────────────
  // Identity pair (D2): record uses label + id; document uses file path.
  const identity = $derived.by((): SourceIdentity | null => {
    const point = pointed;
    if (!point) return null;
    if (point.kind === 'file') return { label: '', key: point.file, address: point.file };
    return {
      label: labelOf(text, point.id),
      key: point.id,
      address: file === '' ? point.id : `${file}#${point.id}`,
    };
  });

  function labelOf(line: string, id: string): string {
    try {
      const value: unknown = JSON.parse(line);
      if (typeof value !== 'object' || value === null) return '';
      const fields = (value as { fields?: unknown }).fields;
      if (typeof fields !== 'object' || fields === null) return '';
      const label = recordLabel({ id, type: '', fields: fields as Record<string, unknown> });
      return label === id ? '' : label;
    } catch {
      return '';
    }
  }

  // Available targets: active target first, followed by deduplicated engine files.
  const picks = $derived.by((): Array<{ option: SourceOption; to: SourceTarget }> => {
    const rows: Array<{ option: SourceOption; to: SourceTarget }> = [];
    const seen = new Set<string>();
    const point = pointed;
    if (point) {
      const key = keyOf(point);
      seen.add(key);
      rows.push({
        option: {
          key,
          label: point.kind === 'record' ? (identity?.label ?? '') : point.file,
          tail: point.kind === 'record' ? `#${point.id}` : '',
          current: true,
        },
        to: point,
      });
    }
    for (const path of files) {
      const key = `file:${path}`;
      if (seen.has(key)) continue;
      seen.add(key);
      rows.push({ option: { key, label: path, tail: '', current: false }, to: { kind: 'file', file: path } });
    }
    return rows;
  });

  const options = $derived(picks.map((row) => row.option));

  // ── reading ────────────────────────────────────────────────────────────
  // Fetch available files from engine (Appendix C.4).
  async function listFiles(): Promise<void> {
    try {
      const result = await dispatch('source', {}, { plan: app.plan });
      files = (result.data as { files?: string[] }).files ?? [];
    } catch (failure) {
      app.report(failure);
    }
  }

  // Load target content; skips if an open draft matches (Appendix C.4).
  async function load(next: SourceTarget): Promise<void> {
    if (app.draft && same(app.draft.target, next)) return;
    if (!committing) {
      status = 'idle';
      receipt = null;
    }
    window.clearTimeout(checking);
    loading = true;
    try {
      // Direct `source` read bypasses `app.run` to prevent triggering post-write reload loops.
      const result =
        next.kind === 'record'
          ? await dispatch('source', { id: next.id }, { plan: app.plan })
          : await dispatch('source', { file: next.file }, { plan: app.plan });
      const data = result.data as SourceRead;
      target = next;
      if (app.paneFile) app.paneFile = null;
      file = data.file ?? (next.kind === 'file' ? next.file : '');
      app.draft = null;
      app.externalPending = false;
      findings = [];
      text = data.text;
    } catch (failure) {
      target = next;
      file = '';
      findings = [];
      text = '';
      app.report(failure);
    } finally {
      loading = false;
    }
  }

  function pick(key: string): void {
    const chosen = picks.find((row) => row.option.key === key);
    if (!chosen) return;
    if (app.draft) {
      app.notice('the open draft is what the pane holds — Commit it, or press Reload');
      return;
    }
    void load(chosen.to);
  }

  function onText(next: string): void {
    text = next;
    if (!committing) {
      status = 'idle';
      receipt = null;
    }
    const point = app.draft?.target ?? target;
    if (point) app.draft = { target: point, text: next, external: app.draft?.external === true };
    window.clearTimeout(checking);
    checking = window.setTimeout(() => void lint(), 400);
  }

  // ── syntax checking (source.check) ───────────────────────────────────
  async function check(where: string, content: string): Promise<SourceFinding[]> {
    if (where === '') return [];
    try {
      const result = await dispatch('source.check', { file: where, content }, { plan: app.plan });
      return (result.data as { diagnostics?: SourceFinding[] }).diagnostics ?? [];
    } catch (failure) {
      app.report(failure);
      return [];
    }
  }

  // Debounced syntax check (§4.8 P5).
  async function lint(): Promise<void> {
    if (file === '') return;
    const content = text;
    const list = await check(file, content);
    if (text === content) findings = list;
  }

  // ── committing ─────────────────────────────────────────────────────────
  /** Commit draft to engine. Draft remains open if commit fails. */
  async function commit(): Promise<void> {
    const point = app.draft?.target ?? target;
    if (!point || file === '') return;
    const draft = text;

    window.clearTimeout(checking);
    const found = await check(file, draft);
    findings = found;
    if (found.length > 0) {
      // Syntax errors found; stay idle without writing.
      status = 'idle';
      receipt = null;
      return;
    }

    committing = true;
    status = 'working';
    try {
      const wrote = point.kind === 'record' ? await commitRecord(point, draft) : await commitFile(draft);
      if (!wrote) {
        status = 'idle';
        return;
      }
      // Replace draft with canonical text and mark committed.
      app.draft = null;
      app.externalPending = false;
      findings = [];
      await load(point);
      receipt = wrote.summary ?? `source.apply ${file}`;
      status = 'done';
      window.clearTimeout(finishing);
      finishing = window.setTimeout(() => {
        status = 'idle';
        receipt = null;
      }, 2600);
    } finally {
      committing = false;
      seenRevision = app.revision;
    }
  }

  async function commitFile(draft: string): Promise<DispatchResult | null> {
    return app.run('source.apply', { file, content: draft });
  }

  /**
   * Commit a record draft. For new records (empty id), delegates to `record.paste`
   * (§4.7 rule 2). For existing records, replaces target line in source file and applies.
   */
  async function commitRecord(point: { kind: 'record'; id: string }, draft: string): Promise<DispatchResult | null> {
    let value: unknown;
    try {
      value = JSON.parse(draft);
    } catch (failure) {
      app.report(failure);
      return null;
    }
    if (typeof value !== 'object' || value === null || Array.isArray(value)) {
      app.report(new Error('the draft is not one JSON object — a record is one object, one line'));
      return null;
    }
    const record = value as Record<string, unknown>;
    const id = typeof record.id === 'string' ? record.id.trim() : '';

    if (id === '') {
      const type = typeOf(record);
      if (type === '') {
        app.report(new Error('the draft names no "type", so there is no kind to append it to'));
        return null;
      }
      return app.run('record.paste', { type, records: JSON.stringify([rowOf(record)]) });
    }

    const raw = await dispatch('source', { file }, { plan: app.plan });
    const swapped = swapLine((raw.data as SourceRead).text, point.id, draft.trim());
    if (swapped === null) {
      app.report(new Error(`“${point.id}” is not in ${file} — Reload to read the file as it is now`));
      return null;
    }
    return app.run('source.apply', { file, content: swapped });
  }

  /** The kind a draft belongs to: its own `type`, else the row the table shows. */
  function typeOf(record: Record<string, unknown>): string {
    if (typeof record.type === 'string' && record.type !== '') return record.type;
    return app.rows().find((row) => row.id === record.id)?.type ?? '';
  }

  /**
   * Normalize a record line for `record.paste` (§3.1, §4.4).
   * Lifts `fields` and `links` to top-level, removes envelope keys and non-persisted
   * derived fields (`formula`, `progress`).
   */
  function rowOf(record: Record<string, unknown>): Record<string, unknown> {
    const row: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(record)) {
      if (key === 'id' || key === 'type' || key === 'schemaVersion') continue;
      if (key === 'fields' || key === 'links') {
        for (const [name, item] of Object.entries((value ?? {}) as Record<string, unknown>)) row[name] = item;
        continue;
      }
      row[key] = value;
    }
    for (const field of app.type?.fields ?? []) {
      if (field.type === 'formula' || field.type === 'progress') delete row[field.key];
    }
    return row;
  }

  /** Replace a record's line matching `id` (§4.3). Returns null if not found. */
  function swapLine(content: string, id: string, line: string): string | null {
    const lines = content.replace(/\r\n/g, '\n').split('\n');
    for (let index = 0; index < lines.length; index += 1) {
      const raw = lines[index].trim();
      if (raw === '') continue;
      try {
        const value: unknown = JSON.parse(raw);
        if (typeof value === 'object' && value !== null && (value as { id?: unknown }).id === id) {
          lines[index] = line;
          return lines.join('\n');
        }
      } catch {
        // Skip unparseable lines.
        continue;
      }
    }
    return null;
  }

  // ── diagnostic highlights (§4.8 P5) ──────────────────────────────────
  /**
   * Resolve DOM cell element for a diagnostic finding (§4.3, §4.8 P5).
   * Locates row by record id, then finds matching column cell if available.
   */
  function cellOf(finding: SourceFinding): Element | null {
    const pointer = finding.path.split('#')[1] ?? '';
    const parts = pointer.split('/');
    const id = parts[0] || (pointed?.kind === 'record' ? pointed.id : recordIdOn(finding.line));
    if (!id) return null;
    const row = document.querySelector(`[data-record-id="${quote(id)}"]`);
    if (!row) return null;
    const at = parts.indexOf('fields');
    if (at >= 0 && parts[at + 1]) {
      const cell = row.querySelector(`[data-column="${quote(parts[at + 1])}"]`);
      if (cell) return cell;
    }
    return row.querySelector('[data-column]') ?? row;
  }

  /** Extract record id from the specified line of draft text. */
  function recordIdOn(line: number | null): string {
    if (file === '' || text === '') return '';
    const lines = text.replace(/\r\n/g, '\n').split('\n');
    const number = Math.min(Math.max(line ?? 1, 1), lines.length);
    try {
      const value: unknown = JSON.parse(lines[number - 1]);
      return typeof value === 'object' && value !== null ? String((value as { id?: unknown }).id ?? '') : '';
    } catch {
      return '';
    }
  }

  /** Quote string for CSS attribute selector. */
  function quote(value: string): string {
    return value.replace(/["\\]/g, '\\$&');
  }

  function flag(finding: SourceFinding | null): void {
    unflag();
    if (!finding) return;
    const cell = cellOf(finding);
    if (!cell) return;
    cell.classList.add('is-flagged');
    flagged = cell;
  }

  function unflag(): void {
    flagged?.classList.remove('is-flagged');
    flagged = null;
  }

  // ── controls ──────────────────────────────────────────────────────────
  /** Copy source target address to clipboard (D2). */
  async function copy(): Promise<void> {
    const where = identity?.address ?? '';
    if (where === '') return;
    try {
      await navigator.clipboard.writeText(where);
      app.notice('copied the address');
    } catch {
      // Clipboard API unavailable in current webview context.
      app.notice('cannot copy the address — the clipboard is unavailable here');
    }
  }

  /** Toggle source pane closed. */
  function close(): void {
    void app.run('app.toggleSourcePane');
  }

  /** Dismiss external change banner and reload target content. */
  function reload(): void {
    const point = app.draft?.target ?? target;
    app.draft = null;
    app.externalPending = false;
    if (point) void load(point);
  }

  // ── effects: list, target, revision ──────────────────────────────────
  $effect(() => {
    // Initial fetch of engine file list. Schema modifications refresh via revision effect.
    if (!listed) {
      listed = true;
      void listFiles();
    }
  });

  $effect(() => {
    // Synchronously track dependencies before async load to maintain effect reactivity.
    const next = wanted();
    if (!next || (target && same(target, next))) return;
    void load(next);
  });

  $effect(() => {
    // External write handling (§4.8 P6): re-read target if clean, or prompt reload if dirty.
    // Watches `app.externalPending` to detect disk changes while preserving active edits.
    const revision = app.revision;
    const pending = app.externalPending;
    const draft = app.draft;
    if (seenRevision === revision && !pending) return;
    if (seenRevision === null) {
      seenRevision = revision;
      return;
    }
    seenRevision = revision;
    if (committing) return;
    if (draft) {
      // Only update draft if not already marked external to prevent cascading effect re-runs.
      if (draft.external !== true) {
        app.draft = { target: draft.target, text: draft.text, external: true };
        app.externalPending = true;
      }
      return;
    }
    if (pending) {
      // No draft: re-read target on external change.
      void listFiles();
      const current = target ?? wanted();
      if (current) void load(current);
      return;
    }
    void listFiles();
    const current = target ?? wanted();
    if (current) void load(current);
  });
</script>

<Variant
  surface="source-pane"
  {options}
  {file}
  {identity}
  {text}
  {findings}
  {loading}
  pointed={pointed !== null}
  external={external}
  {status}
  {receipt}
  {commitKey}
  {toggleKey}
  onPick={pick}
  onText={onText}
  onCommit={() => void commit()}
  onReload={reload}
  onClose={close}
  onCopy={() => void copy()}
  onFlag={flag}
/>
