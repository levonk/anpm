# Cloning a Repository

This example shows how to use `apmw clone` to create historyless clones with
AST indexing.

## Synchronous clone

```bash
$ apmw clone https://github.com/owner/repo
```

Performs a historyless, branchless, tagless clone (no `.git` history) and
runs AST indexing for fast code search. The clone destination follows the
default convention (see `default_clone_dest`).

## Background clone (daemon mode)

```bash
$ apmw --daemon clone https://github.com/owner/repo
```

Submits the clone as a background job to the daemon and returns immediately
with a job ID. The clone proceeds in the background.

## List background jobs

```bash
$ apmw --list-jobs
JOB_ID           STATUS     KIND       DESCRIPTION
job-abc123       running    clone      clone https://github.com/owner/repo
```

## Cancel a background job

```bash
$ apmw --cancel-job job-abc123
Job job-abc123 cancelled (status: cancelled).
```

## Force synchronous mode

```bash
$ apmw --no-daemon clone https://github.com/owner/repo
```

Forces in-process operation (no daemon). Note: some operations require the
daemon and will error in `--no-daemon` mode.

## What happens during a clone

1. **Historyless fetch** — `git clone --depth 1` (no full history).
2. **`.gitignore`** — a local `.gitignore` is written to exclude build
   artifacts.
3. **AST indexing** — an AST tool (tree-sitter, ctags, or similar,
   auto-selected based on availability) indexes the source for fast code
   search.
4. **Audit log** — the clone is recorded in the audit log.

## JSON output

```bash
$ apmw clone https://github.com/owner/repo --json
```

Emits the clone result (repo, path, ast_tool, indexed, file_count) as JSON.
