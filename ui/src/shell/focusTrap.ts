/**
 * THE MODAL'S KEYBOARD CONTRACT, in one place (Appendix C.9, §11).
 *
 * The app has three modal surfaces and they behave the same way to a keyboard:
 * `shell/Sheet.svelte` (a form, a confirm, the paste door), the command palette,
 * and `onboarding/Onboarding.svelte`'s room when the document is re-read over a
 * working plan. Focus lands inside on open, Tab cycles within the boundary,
 * Escape leaves, and the keyboard goes back to whatever held it before.
 *
 * The rules, each one learned rather than chosen:
 *
 * 1 · **A field first, then any control, else the boundary itself.** Landing a
 *     form on its own close button teaches the wrong first move; a body that
 *     already took the keyboard is left alone (the inserter's search field is
 *     focused by its own action, and an action runs before this).
 * 2 · **The boundary is the boundary, not an interior stop.** Focus on the node
 *     itself — or gone to `<body>` — wraps to the first or last control, so the
 *     browser's own sequential navigation can never step out on either side.
 * 3 · **Return is a fallback, not a second owner.** It is checked one microtask
 *     after teardown, when the browser has settled where focus went: if anything
 *     other than `<body>` holds it, the surface that took it keeps it.
 *
 * The listener is registered synchronously at mount, exactly as
 * `<svelte:window onkeydown={…}>` registered it before this was extracted: an
 * *Escape* that mounts a modal is not a gesture this app has, and arming late
 * would delay the key the contract exists for. (The scrim's click dismissal
 * does need arming — mount happens inside the opening click's own dispatch — and
 * that stays in the sheet, where the click is tested for containment.)
 */
export function modal(
  node: HTMLElement,
  params: { onclose: () => void },
): { update: (next: { onclose: () => void }) => void; destroy: () => void } {
  let onclose = params.onclose;

  /** Everything a Tab can reach inside the boundary, in DOM order. */
  function reachable(): HTMLElement[] {
    return Array.from(
      node.querySelectorAll<HTMLElement>(
        'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ),
    ).filter((el) => el.getClientRects().length > 0);
  }

  function onkeydown(event: KeyboardEvent): void {
    // Escape abandons the surface; a form never traps the user in it.
    if (event.key === 'Escape') {
      event.preventDefault();
      onclose();
      return;
    }
    if (event.key !== 'Tab') return;
    const items = reachable();
    if (items.length === 0) {
      // Nothing to cycle: hold the keyboard on the boundary itself.
      event.preventDefault();
      node.focus();
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement;
    const inside = active instanceof HTMLElement && active !== node && node.contains(active);
    if (event.shiftKey ? !inside || active === first : !inside || active === last) {
      event.preventDefault();
      (event.shiftKey ? last : first).focus();
    }
  }

  /**
   * The control that held the keyboard when this mounted. Read at init rather
   * than in an effect: by the time an effect runs the surface is in the DOM and
   * a body may already have moved focus, so the departure point would be gone.
   */
  const opener: HTMLElement | null =
    typeof document === 'undefined' ? null : (document.activeElement as HTMLElement | null);

  if (!node.contains(document.activeElement)) {
    const field = node.querySelector<HTMLElement>(
      'input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [contenteditable="true"]',
    );
    const first = field?.getClientRects().length ? field : (reachable()[0] ?? node);
    first.focus();
  }

  window.addEventListener('keydown', onkeydown);

  return {
    update(next) {
      onclose = next.onclose;
    },
    destroy() {
      window.removeEventListener('keydown', onkeydown);
      if (!opener || opener === document.body) return;
      queueMicrotask(() => {
        const active = document.activeElement;
        if (active !== null && active !== document.body) return;
        if (opener.isConnected) opener.focus();
      });
    },
  };
}
