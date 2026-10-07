/**
 * The one place a native menu choice lands.
 *
 * A menu is the OS's event, not an answer to a question: the shell shows the
 * menu the page asked for and reports the clicked id on one channel. Context
 * menus register an interest while they are open (their actions are closures the
 * page owns); anything nobody claims is dispatched as a registry command through
 * `app.run`, which is the same interception table the palette and the keyboard
 * use — so a menu-bar item and a palette row are one operation, not two.
 */
import { app } from '../session.svelte';

type Handler = (id: string) => boolean;
const handlers = new Set<Handler>();

/** Claim ids while mounted. Returns the release function. */
export function claimMenuChoices(handler: Handler): () => void {
  handlers.add(handler);
  return () => handlers.delete(handler);
}

export function dispatchMenuChoice(id: string): void {
  if (id.length === 0) return;
  for (const handler of handlers) {
    if (handler(id)) return;
  }
  void app.run(id);
}
