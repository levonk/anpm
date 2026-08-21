---
title: "Outdated dependencies detected by weekly check"
labels: ["dependencies", "maintenance"]
---

The weekly `cargo outdated` check found dependencies that are out of date.

## Run

- Workflow: [Outdated dependencies check]({{ run_url }})
- Branch: `main`
- Triggered by: {{ trigger }}

## Action needed

Run `just outdated` locally for the full report, then update `Cargo.toml`
and run `just validate` to confirm fmt + clippy + test + doc + audit still
pass.

## Notes

- This issue is auto-created by the weekly scheduled CI job and updated on
  subsequent failures. Close it once the dependencies are updated.
- If a dependency cannot be updated (MSRV conflict, breaking change, etc.),
  document the reason in a comment and close the issue.
