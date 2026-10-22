/** One row in a deck pill's listbox. `note` is the quiet right-hand text (an id). */
export type PickRow = { id: string; label: string; note?: string };

/** Rows a provider mark can head; the mark is drawn above the group's first row. */
export type PickGroup = { name: string; rows: PickRow[] };
