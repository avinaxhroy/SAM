/**
 * Pointer drag engine for screen composer frames (COMPOSER §4.4).
 * Uses pointer events rather than HTML5 drag-and-drop for WebKitGTK / desktop webview consistency.
 */

/** Required callbacks and DOM elements provided by the composer stack. */
export type ReorderHost = {
  frames: () => HTMLElement[];
  stack: () => HTMLElement | null;
  scroller: () => HTMLElement | null;
  drop: (from: number, to: number) => void;
};

/** Pointer movement threshold in pixels before drag activates (§4.4). */
const THRESHOLD = 4;
/** Scroller edge margin in pixels for auto-scrolling during drag. */
const EDGE = 64;
/** Total duration for the post-release settle animation class. */
const SETTLE = 220;

/**
 * State for landing animation: tracks remaining transform offset while the
 * dropped element transitions into its final slot in the flow.
 */
export type Settle = {
  frame: HTMLElement;
  x: number;
  y: number;
  glide: boolean;
};

export class Reorder {
  from = $state<number | null>(null);
  to = $state<number | null>(null);
  /** Drop indicator line position and dimensions inside the stack container. */
  line = $state({ x: 0, y: 0, h: 0 });
  /** Active pointer delta in pixels. Applied directly to the frame's inline transform. */
  dx = $state(0);
  dy = $state(0);
  /** Settle transition state for landing frame animation. */
  settle = $state.raw<Settle | null>(null);

  private host: ReorderHost;
  private settleFrame: number | null = null;
  private settleTimer: number | null = null;
  private pending: { index: number; origin: HTMLElement; pointerId: number; x: number; y: number } | null = null;
  private pointerX: number | null = null;
  private pointerY: number | null = null;
  private scrollFrame: number | null = null;

  constructor(host: ReorderHost) {
    this.host = host;
  }

  get dragging(): boolean {
    return this.from !== null;
  }

  /**
   * Pointer down handler. Records initial position and captures pointer;
   * drag begins only after pointer delta exceeds THRESHOLD.
   */
  begin(index: number, event: PointerEvent): void {
    if (this.dragging) return;
    if (this.pending) {
      if (this.pending.origin.isConnected) return;
      this.release(false);
    }
    if (event.pointerType === 'mouse' && event.button !== 0) return;
    if (this.host.frames().length < 1) return;
    const origin = event.currentTarget as HTMLElement | null;
    if (!origin) return;

    this.clearSettle();
    this.dx = 0;
    this.dy = 0;
    this.pending = { index, origin, pointerId: event.pointerId, x: event.clientX, y: event.clientY };

    // Capture immediately so fast pointer sweeps don't lose the target before crossing threshold.
    try {
      origin.setPointerCapture(event.pointerId);
    } catch {
      /* ignore if pointer is already invalid */
    }
    origin.addEventListener('pointermove', this.onMove);
    origin.addEventListener('pointerup', this.onUp);
    origin.addEventListener('pointercancel', this.onCancel);
    origin.addEventListener('lostpointercapture', this.onCancel);
  }

  private onMove = (event: PointerEvent): void => {
    const pending = this.pending;
    if (pending && event.pointerId !== pending.pointerId) return;
    if (this.from === null) {
      if (!pending) return;
      // Start drag once euclidean distance exceeds threshold in any direction.
      if (Math.hypot(event.clientX - pending.x, event.clientY - pending.y) < THRESHOLD) return;
      this.lift(pending);
    }
    // Absolute delta from press origin avoids accumulated rounding errors.
    if (pending) {
      this.dx = event.clientX - pending.x;
      this.dy = event.clientY - pending.y;
    }
    this.pointerX = event.clientX;
    this.pointerY = event.clientY;
    this.measure();
  };

  private onUp = (event: PointerEvent): void => {
    if (this.pending && event.pointerId !== this.pending.pointerId) return;
    if (this.from !== null) {
      const from = this.from;
      const slot = this.to;
      if (slot !== null) {
        const to = slot > from ? slot - 1 : slot;
        if (to !== from) this.host.drop(from, to);
      }
    }
    this.release(true);
  };

  private onCancel = (event: PointerEvent): void => {
    if (this.pending && event.pointerId !== this.pending.pointerId) return;
    this.release(false);
  };

  private onKey = (event: KeyboardEvent): void => {
    if (event.key !== 'Escape' || this.from === null) return;
    event.preventDefault();
    // Stop propagation so canceling drag does not bubble to close edit mode.
    event.stopPropagation();
    this.release(false);
  };

  /** Activate drag state and attach keyboard cancel listener. */
  private lift(pending: { index: number; origin: HTMLElement; pointerId: number; x: number; y: number }): void {
    this.from = pending.index;
    window.addEventListener('keydown', this.onKey, true);
    this.measure();
    this.autoScroll();
  }

  /**
   * Calculates the target insertion index from pointer coordinates.
   * Checks reading order across grid rows and sub-slot boundaries,
   * excluding the active dragged frame from the target set.
   */
  private measure(): void {
    const y = this.pointerY;
    const x = this.pointerX;
    const stack = this.host.stack();
    const frames = this.host.frames();
    if (y === null || !stack || frames.length === 0) return;
    const from = this.from;
    const anchors = from === null ? frames : frames.filter((_, at) => at !== from);
    if (anchors.length === 0) {
      this.to = null;
      return;
    }
    let slot = anchors.length;
    for (let at = 0; at < anchors.length; at += 1) {
      const box = anchors[at].getBoundingClientRect();
      if (y < box.top) {
        slot = at;
        break;
      }
      if (y <= box.bottom) {
        slot = x !== null && x > box.left + box.width / 2 ? at + 1 : at;
        break;
      }
    }
    this.to = from === null || slot <= from ? slot : slot + 1;
    this.line = this.edgeFor(slot, anchors, stack);
  }

  /** Compute drop indicator position centered in the column gutter. */
  private edgeFor(slot: number, anchors: HTMLElement[], stack: HTMLElement): { x: number; y: number; h: number } {
    const stackBox = stack.getBoundingClientRect();
    const before = slot < anchors.length ? anchors[slot].getBoundingClientRect() : null;
    const anchor = before ?? anchors[anchors.length - 1].getBoundingClientRect();
    const gutter = Number.parseFloat(getComputedStyle(stack).columnGap) || 8;
    const x = (before ? anchor.left - gutter / 2 : anchor.right + gutter / 2) - stackBox.left;
    return { x, y: anchor.top - stackBox.top, h: anchor.height };
  }

  /** Auto-scroll container when dragging near viewport edges. */
  private autoScroll = (): void => {
    if (this.from === null) return;
    const scroller = this.host.scroller();
    const y = this.pointerY;
    if (scroller && y !== null) {
      const box = scroller.getBoundingClientRect();
      let by = 0;
      if (y < box.top + EDGE) by = -Math.ceil(Math.min(EDGE, box.top + EDGE - y) / 4);
      else if (y > box.bottom - EDGE) by = Math.ceil(Math.min(EDGE, y - (box.bottom - EDGE)) / 4);
      if (by !== 0) {
        scroller.scrollTop += by;
        this.measure();
      }
    }
    this.scrollFrame = requestAnimationFrame(this.autoScroll);
  };

  /** Clean up event listeners and settle dropped frame or cancel drag. */
  private release(land: boolean): void {
    const pending = this.pending;
    if (pending) {
      pending.origin.removeEventListener('pointermove', this.onMove);
      pending.origin.removeEventListener('pointerup', this.onUp);
      pending.origin.removeEventListener('pointercancel', this.onCancel);
      pending.origin.removeEventListener('lostpointercapture', this.onCancel);
      if (pending.origin.hasPointerCapture(pending.pointerId)) {
        pending.origin.releasePointerCapture(pending.pointerId);
      }
    }
    if (this.from !== null) window.removeEventListener('keydown', this.onKey, true);
    if (this.scrollFrame !== null) {
      cancelAnimationFrame(this.scrollFrame);
      this.scrollFrame = null;
    }
    const landed = land && this.from !== null ? (this.host.frames()[this.from] ?? null) : null;
    this.pending = null;
    this.pointerX = null;
    this.pointerY = null;
    this.from = null;
    this.to = null;
    if (landed) this.holdSettle(landed);
    this.dx = 0;
    this.dy = 0;
  }

  /**
   * Animate dropped frame into final slot over two animation frames.
   * Frame 1 measures the DOM position after layout and offsets the element to its release position.
   * Frame 2 zeroes the transform to trigger the settle transition without snap.
   */
  private holdSettle(frame: HTMLElement): void {
    this.clearSettle();
    const box = frame.getBoundingClientRect();
    const released = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
    this.settle = { frame, x: this.dx, y: this.dy, glide: false };
    this.settleFrame = requestAnimationFrame(() => {
      this.settleFrame = null;
      const held = this.settle;
      if (held?.frame !== frame || !frame.isConnected) {
        this.settle = null;
        return;
      }
      const now = frame.getBoundingClientRect();
      const x = held.x + (released.x - (now.x + now.width / 2));
      const y = held.y + (released.y - (now.y + now.height / 2));
      this.settle = { frame, x, y, glide: false };
      this.settleFrame = requestAnimationFrame(() => {
        this.settleFrame = null;
        if (this.settle?.frame !== frame || !frame.isConnected) {
          this.settle = null;
          return;
        }
        this.settle = { frame, x: 0, y: 0, glide: true };
        this.settleTimer = window.setTimeout(() => {
          this.settleTimer = null;
          this.settle = null;
        }, SETTLE);
      });
    });
  }

  private clearSettle(): void {
    if (this.settleFrame !== null) {
      cancelAnimationFrame(this.settleFrame);
      this.settleFrame = null;
    }
    if (this.settleTimer !== null) {
      clearTimeout(this.settleTimer);
      this.settleTimer = null;
    }
    this.settle = null;
  }
}
