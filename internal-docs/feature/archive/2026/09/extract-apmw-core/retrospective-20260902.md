---
date: "2026-09-02"
feature: extract-apmw-core
status: Shipped (PR #12) — 1 story blocked
---

# Retrospective: extract-apmw-core

## Summary

Converted the single-crate `apmw` repository into a Cargo workspace with
`crates/apmw-core` (reusable library) and `crates/apmw` (binary). Added YAML
custom project types, workspace/monorepo detection, version info extraction,
and prepared `apmw-core` for crates.io publication.

**5 stories planned, 4 completed, 1 blocked.**

## What Went Well

- **Workspace split preserved all 1182 tests** with zero failures. The
  extraction was mechanical and the test suite caught no regressions.
- **`cargo publish --dry-run` passed on the first attempt** after metadata
  preparation — the crate is publication-ready.
- **Sequential story execution avoided gate-pass collisions**. The global
  gate-pass file is a single-writer resource; running stories 08-002 through
  08-004 sequentially was the right call.
- **The holistic review caught real integration issues** that per-story
  reviews missed — specifically the duplicate `Ecosystem`/`PackageManager`
  types and the manager-name alias gaps.
- **PRD and task index were pre-existing and well-structured**, which made
  story execution straightforward.

## What Didn't Go Well

- **Devbox/Nix environment was broken** due to a `github:levonk/nono` flake
  issue. This blocked `just validate`, clippy, and rustfmt for the entire
  session. Quality gates that depend on these tools could not be run.
- **Story 08-001's worktree had pre-existing uncommitted work** from a
  previous session. This required inspection and careful completion rather
  than a clean start.
- **Story 08-004 had a merge conflict** between an already-merged
  implementation and a newer subagent implementation. Manual conflict
  resolution was needed.
- **The extraction introduced duplicate `Ecosystem`/`PackageManager` types**
  in `ecosystem` and `detect`. The original code had one `Ecosystem` enum in
  `detect`; the extraction created a second, incompatible one in `ecosystem`.
  This is the most significant tech debt item.
- **Crates.io publication blocked** on missing API token. No
  `~/.cargo/credentials` file existed.

## Lessons Learned

1. **Extraction stories need an explicit type-unification step.** When
   splitting a module into a reusable crate, the canonical type source must
   be decided upfront. The extraction created a new `Ecosystem` enum in
   `ecosystem` without removing or reconciling the one in `detect`.
2. **The global gate-pass file limits parallelism.** Stories that could run
   in parallel must run sequentially because the gate uses a single
   `/tmp/devin-execution-gates/current-gate-pass` file. A per-story gate-pass
   would enable safe parallel execution.
3. **Pre-existing worktree state should be checked before dispatch.** The
   08-001 worktree had uncommitted work from a prior session. A pre-dispatch
   `git status` check would surface this earlier.
4. **Tool availability should be verified before starting.** The session
   discovered missing clippy, rustfmt, and broken devbox midway through.
   A `just doctor` at session start would have flagged this.
5. **Holistic review is essential after multi-story features.** Per-story
   reviews confirmed each story's scope but missed cross-story type
   divergence. The Phase 8.5 review caught 9 integration issues.

## Action Items

- [ ] **Unify `Ecosystem`/`PackageManager` types** in `apmw-core` — make
      `ecosystem` canonical, update `detect` and `custom_types` to consume
      it. Target: `apmw-core` v0.2.
- [ ] **Reconcile manager name aliases** across `detect`, `parse_manager`,
      and `VALID_MANAGERS`.
- [ ] **Fix devbox/Nix flake** for `github:levonk/nono` to restore
      `just validate` capability.
- [ ] **Provide crates.io token** and run `cargo publish -p apmw-core` to
      unblock story 08-005.
- [ ] **Consider per-story gate-pass files** to enable parallel story
      execution in future multi-story features.
