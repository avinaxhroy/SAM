# Command-line interface (Surface D)

The `sam` command-line binary exposes all engine capabilities directly in the terminal, without launching a graphical window or running a background daemon.

---

## Global options

Every invocation of `sam` accepts these standard flags:

```bash
sam <command-id> [arguments...] [options]
```

| Flag | Description |
|---|---|
| `--plan <path>` | Path to the plan directory. If omitted, targets the machine's default active plan directory. |
| `--json` | Output machine-readable JSON. JSON is also enabled automatically when standard output is not a terminal (e.g. piped to another process). |
| `--if-revision <hash>` | Enforces optimistic concurrency. Aborts with exit code `3` (`CONFLICT`) if the current plan revision hash does not match. |
| `--dry-run` | Validates the mutation and computes changes without writing bytes to disk. |

---

## Exit codes

The CLI reports process exit codes according to §4.9 of the specification:

| Code | Name | Meaning |
|---|---|---|
| `0` | `OK` | Command completed successfully. |
| `1` | `INVALID` | Schema validation error, malformed input, or unfulfilled proof gate. |
| `2` | `USAGE` | Missing required argument, unrecognized command, or invalid flag. |
| `3` | `CONFLICT` | Concurrent modification conflict or mismatched `--if-revision`. |
| `4` | `IO` | File system read/write failure, missing directory, or permission error. |
| `5` | `PRESENTATION` | Desktop windowing or shell-specific error. |

---

## Plan management

### Create a new plan

Create an empty plan from the `blank` preset:
```bash
sam plan.new --name "Computer Systems" --preset blank
```

Create a plan from a bundled starter:
```bash
sam plan.new --name "Calculus Term" --preset university-term
```

### Inspect paths and revisions

Print paths for the plan root, documents, SQLite cache, and current revision hash:
```bash
sam paths --plan ./my-plan --json
```

### Validate plan files

Check the entire plan configuration and record tree for schema violations or broken links:
```bash
sam configcheck --plan ./my-plan
```

---

## Record operations

### Add a record

Create a record by providing JSON-encoded fields:
```bash
sam record.new \
  --type topic \
  --fields '{"title":"Binary arithmetic","kind":"watch","est":30}' \
  --links '{"course":["c.cs.101"]}' \
  --plan ./my-plan
```

### Update a field

Modify a single field on an existing record:
```bash
sam record.setField \
  --id topic.r1 \
  --field title \
  --value "Advanced binary arithmetic" \
  --plan ./my-plan
```

### Delete a record

Remove a record and clean up relational references pointing to it:
```bash
sam record.delete --id topic.r1 --plan ./my-plan
```

---

## Study loop and reviews

### Advance a pipeline stage

Move a record along its progression state machine. When advancing under the `flip` pipeline:

```bash
# Advance to proved, meeting the 2-problem minimum
sam record.advanceStage \
  --id topic.r1 \
  --stage proved \
  --problems 2 \
  --plan ./my-plan

# Advance to anchored with an explicit bypass reason
sam record.advanceStage \
  --id topic.r1 \
  --stage anchored \
  --reason "Verified during seminar" \
  --plan ./my-plan
```

### Log a spaced repetition review

Record a review rating (`again`, `hard`, `good`, `easy`):
```bash
sam record.logReview \
  --id topic.r1 \
  --rating good \
  --plan ./my-plan
```

The review updates stability, calculates the next due date via the plan's configured scheduler (`fixed`, `sm2`, or `fsrs`), and commits the event to `state/state.json`.

---

## Batch mutations (`sam apply`)

For bulk updates, migrations, or agent scripts, `sam apply` processes multiple record mutations in a single atomic transaction.

### Batch format

A batch file is line-delimited JSON where each line defines a canonical record with `schemaVersion: 1`:

```jsonl
{"schemaVersion":1,"id":"c.cs.101","type":"course","fields":{"code":"CS101","name":"Data Structures","credits":4},"links":{}}
{"schemaVersion":1,"id":"t.01","type":"topic","fields":{"title":"Karnaugh maps","kind":"practice"},"links":{"course":["c.cs.101"]}}
{"schemaVersion":1,"id":"t.02","type":"topic","fields":{"title":"Sequential logic","kind":"watch"},"links":{"course":["c.cs.101"]}}
```

### Running a batch

Apply from a file:
```bash
sam apply update.jsonl --plan ./my-plan
```

Apply from standard input:
```bash
cat update.jsonl | sam apply - --plan ./my-plan
```

If any line in the batch fails lexical parsing or whole-schema validation, the entire transaction rolls back with exit code `1`, leaving zero bytes modified on disk.

For AI agent integration patterns and the complete prompt template, see [`docs/ai-agents.md`](ai-agents.md).

---

## Queries and inspection

### Read today's queue

Fetch current metrics, active reviews, and queued study topics:
```bash
sam today.view --plan ./my-plan --json
```

### Full-text search

Search records across all fields using the derived SQLite index:
```bash
sam search "floating point" --plan ./my-plan --json
```

### Evaluate an expression

Test an expression against a record:
```bash
sam expr.eval \
  --expr "est * 2" \
  --id topic.r1 \
  --plan ./my-plan
```

---

## Recovery and profiles

### Inspect transaction history

List past mutations and pre-image backup snapshots:
```bash
sam tx.list --plan ./my-plan --json
```

### Restore a backup

Roll back the plan to a previous transaction snapshot:
```bash
sam tx.restore --txid 20261007-011500-123 --plan ./my-plan
```

### Export a profile

Export a shareable profile containing the schema, views, rules, and public records:
```bash
sam profile.export --out curriculum.samprofile --plan ./my-plan
```

To include private records and personal review progress, pass `--personal`:
```bash
sam profile.export --out backup.samprofile --personal --plan ./my-plan
```

### Import a profile

Instantiate a new plan from a profile file:
```bash
sam profile.import --file curriculum.samprofile --name "Imported Course"
```
