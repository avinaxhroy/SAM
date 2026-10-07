<!--
  Active study session controller.
  Derives session elapsed time, manages the 1s tick, dispatches record writes on stop,
  and renders the active timer-chrome variant.
-->
<script lang="ts">
  import Variant from '../variants/Variant.svelte';
  import type { TimerProps } from '../variants/timer-chrome/props';
  import { styles } from '../variants/styles.svelte';
  import { app, elapsedMs, type TimerState } from '../session.svelte';

  /** The session that just ended, kept only as long as its receipt is on screen. */
  type Receipt = {
    label: string;
    course: string | null;
    clock: string;
    started: string;
    targetMin: number;
    elapsedMin: number;
  };

  const RECEIPT_MS = 8000; // the app's own lifetime for a receipt with an action

  let now = $state(Date.now());
  $effect(() => {
    if (!app.timer) return;
    now = Date.now();
    const tick = setInterval(() => {
      now = Date.now();
    }, 1000);
    return () => clearInterval(tick);
  });

  const chosen = $derived(styles.variantOf('timer-chrome'));

  let receipt = $state<Receipt | null>(null);
  let endTimer: ReturnType<typeof setTimeout> | null = null;

  /** `mm:ss`, the widest string the 5–180 clamp can produce. */
  function clockOf(timer: TimerState, at: number): string {
    const total = Math.floor(elapsedMs(timer, at) / 1000);
    return `${String(Math.floor(total / 60)).padStart(2, '0')}:${String(total % 60).padStart(2, '0')}`;
  }

  /** The run's wall-clock start, for the facts panel — a time, never a timestamp. */
  function startLabelOf(timer: TimerState): string {
    const at = new Date(timer.startedAt);
    return `${String(at.getHours()).padStart(2, '0')}:${String(at.getMinutes()).padStart(2, '0')}`;
  }

  /** The write Stop performs: the plan's own kind that records minutes. */
  const command = $derived(app.sessionKind() ? `${app.sessionKind()}.new` : null);

  const chrome = $derived.by((): TimerProps | null => {
    const live = app.timer;
    if (live) {
      const elapsed = elapsedMs(live, now);
      return {
        label: live.item.label,
        course: live.item.course?.label ?? null,
        clock: clockOf(live, now),
        started: startLabelOf(live),
        paused: live.pausedAt !== null,
        targetMin: live.targetMin,
        elapsedMin: elapsed / 60_000,
        command,
        placement: 'today.screen',
        ended: false,
        onPause: () => app.togglePause(),
        onStop: () => void stop(),
        onMinutes: (minutes: number) => {
          const timer = app.timer;
          if (timer) app.timer = { ...timer, targetMin: minutes };
        },
        onUndo: () => void app.undo(),
      };
    }
    if (!receipt) return null;
    return {
      label: receipt.label,
      course: receipt.course,
      clock: receipt.clock,
      started: receipt.started,
      paused: false,
      targetMin: receipt.targetMin,
      elapsedMin: receipt.elapsedMin,
      command,
      placement: 'today.screen',
      ended: true,
      onPause: () => {},
      onStop: () => {},
      onMinutes: () => {},
      onUndo: () => void app.undo(),
    };
  });

  function endReceipt(): void {
    if (endTimer !== null) clearTimeout(endTimer);
    endTimer = setTimeout(() => {
      endTimer = null;
      receipt = null;
    }, RECEIPT_MS);
  }

  async function stop(): Promise<void> {
    const timer = app.timer;
    if (!timer) return;
    // Capture elapsed time before clearing state to prevent ticking during write.
    const elapsed = elapsedMs(timer, Date.now());
    const keep = chosen === 'b';
    await app.stopSession();
    // Sessions under 1 minute or plans without a time-logging kind record no entry.
    if (elapsed < 60_000 || command === null) return;
    receipt = {
      label: timer.item.label,
      course: timer.item.course?.label ?? null,
      clock: clockOf(timer, Date.now()),
      started: startLabelOf(timer),
      targetMin: timer.targetMin,
      elapsedMin: elapsed / 60_000,
    };
    if (keep) {
      // Variant B displays inline receipt, suppressing shell toast.
      app.toast = null;
      app.toastAction = null;
    }
    endReceipt();
  }
</script>

{#if chrome}
  {@const current = chrome}
  <Variant surface="timer-chrome" {...current} />
{/if}
