---
story_id: "08-005"
story_title: "Publish apmw-core to crates.io"
story_name: "publish-apmw-core"
prd_name: "extract-apmw-core"
prd_file: "internal-docs/feature/todo/extract-apmw-core/feat-202609021309-extract-apmw-core.md"
phase: 8
parallel_id: 5
branch: "feature/current/extract-apmw-core/story-08-005-publish-apmw-core"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["08-001", "08-002", "08-003", "08-004"]
parallel_safe: false
modules: ["crates/apmw-core/Cargo.toml", "crates/apmw/Cargo.toml"]
priority: "MUST"
risk_level: "high"
tags: ["publish", "crates-io", "release"]
due: ""
create-date: "2026-09-02"
update-date: "2026-09-02"
---

## Summary

Verify `apmw-core` is ready for publishing with `cargo publish --dry-run`,
fix any packaging issues, then publish `apmw-core` to crates.io. Update the
binary's `Cargo.toml` to reference the published version. This story requires
crates.io authentication (API token) — if the token is not available, mark
the story blocked with the question for the user.

## Sub-Tasks

- [ ] Run `cargo publish --dry-run -p apmw-core` and fix any errors:
  - Missing `description`, `license`, `repository` in `Cargo.toml`.
  - Path dependencies that aren't allowed in published crates (verify
    `apmw-core` has no path deps on local code — only published crates).
  - Missing `README` field if one is referenced.
  - Include/exclude patterns for the package.
- [ ] Verify `crates/apmw-core/Cargo.toml` has all required publish fields:
  `name`, `version`, `edition`, `authors`, `description`, `license`,
  `repository`, `keywords` (optional but recommended), `categories`
  (optional but recommended).
- [ ] Add `README.md` to `crates/apmw-core/` if not present (referenced by
  `readme = "README.md"` in Cargo.toml).
- [ ] Run `cargo publish --dry-run -p apmw-core` again until it passes with
  no errors and no warnings.
- [ ] Check for crates.io authentication:
  `cargo login` status or `~/.cargo/credentials` existence.
  - If authenticated: proceed to publish.
  - If NOT authenticated: mark story `[!] Blocked` with question for user
    (need crates.io API token).
- [ ] If authenticated, run `cargo publish -p apmw-core` to publish.
- [ ] Verify the crate is downloadable: `cargo search apmw-core` or
  `cargo add apmw-core --dry-run` in a temp directory.
- [ ] Update `crates/apmw/Cargo.toml` to confirm the `apmw-core` dependency
  uses `version = "0.1.0"` alongside `path = "../apmw-core"` (already set
  in 08-001, just verify).
- [ ] Run `cargo test --workspace` and `just validate` one final time.

## Relevant Files

- `crates/apmw-core/Cargo.toml` — publish metadata
- `crates/apmw-core/README.md` — crate README (if needed)
- `crates/apmw/Cargo.toml` — verify apmw-core dependency

## Acceptance Criteria (Gherkin)

- Given `apmw-core` is fully implemented, When
  `cargo publish --dry-run -p apmw-core` is run, Then it succeeds with no
  errors and no warnings.
- Given crates.io authentication is available, When
  `cargo publish -p apmw-core` is run, Then the crate is published
  successfully.
- Given `apmw-core` is published, When `cargo search apmw-core` is run,
  Then the crate appears in search results.
- Given `apmw-core` is published, When inspecting `crates/apmw/Cargo.toml`,
  Then the dependency uses `{ path = "../apmw-core", version = "0.1.0" }`.

## Test Plan

- `cargo publish --dry-run -p apmw-core` — passes
- `cargo test --workspace` — all tests pass
- `just validate` — all quality gates pass
- `cargo search apmw-core` — crate found (after publish)

## Risks & Mitigations

- **Risk**: `apmw-core` name is already taken on crates.io.
  **Mitigation**: Check with `cargo search apmw-core` first. If taken,
  fall back to `apmw_detect` or `project-detect-core` per the PRD decisions.
- **Risk**: No crates.io API token available.
  **Mitigation**: Mark story `[!] Blocked` with the question for the user
  (they need to provide a crates.io API token via `cargo login`).
- **Risk**: Path dependencies in `apmw-core` that aren't allowed in
  published crates.
  **Mitigation**: Verify `apmw-core` only depends on published crates (no
  `path = "..."` deps except dev-dependencies). The `apmw` binary's path dep
  on `apmw-core` is fine because `apmw` is not being published.

## Dependencies & Sequencing

- **Depends on**: 08-001 (workspace), 08-002 (YAML types), 08-003 (workspace
  detection), 08-004 (version info). All features must be in `apmw-core`
  before publishing.
- **Unblocks**: nothing (this is the final story).

## Definition of Done

- `cargo publish --dry-run -p apmw-core` passes with no errors/warnings
- `apmw-core` is published to crates.io (or blocked if no auth token)
- `cargo search apmw-core` finds the crate
- `just validate` passes

## Blocker (if no crates.io auth)

If `cargo login` has not been run and `~/.cargo/credentials` does not exist:

**Question for user**: crates.io authentication is required to publish
`apmw-core`. Do you have a crates.io API token?

**Options**:
1. Provide the token now — I'll run `cargo login` and publish.
2. Skip publishing for now — mark as blocked, publish manually later.
3. Publish under a different name — if `apmw-core` is taken.

**Recommendation**: Option 1 (provide token) — publishing completes the
feature's Definition of Done.
