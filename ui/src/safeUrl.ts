/**
 * One place decides whether a string may become a link (§3.4: *"the scheme is
 * checked before a link is offered, let alone opened"*). Plan and profile
 * records carry free-text `url` fields, so every anchor bound to record data
 * and every `window.open` goes through here — a `javascript:` or `data:`
 * value is text, never a destination. Callers hand it whatever the record
 * carried (including null after their own string checks), so a non-string is
 * simply not a link.
 */
export function safeUrl(text: unknown): boolean {
  return typeof text === 'string' && /^https?:\/\//i.test(text.trim());
}
