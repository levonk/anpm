---
story_id: "04-004"
story_title: "Container package support (docker/podman image pull/scan/list, compose detection)"
story_name: "container-packages"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 4
parallel_id: 4
branch: "feature/current/apmw/story-04-004-container-packages"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["02-001", "03-003"]
parallel_safe: true
modules: ["src/containers/"]
priority: "SHOULD"
risk_level: "medium"
tags: ["feat", "containers", "docker", "podman", "scan"]
due: "2026-09-30"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Implement container package support. `apmw add <image>` pulls a container image (docker pull / podman pull). `apmw add <image> --dev` for dev-only images. `apmw scan <image>` runs security scanning via trivy/grype. `apmw list --manager docker` lists local images. Detect container usage from Dockerfile, docker-compose.yml, docker-compose.*.yml, Containerfile, compose.yaml. Support docker-compose as orchestration package manager. Support governance rules for containers (prefer-podman over docker).

## Current State

- **Relevant files and their roles:**
  - PRD FR-7 — Container package support
  - `src/detect/` — Detection engine (from story 02-001) for container manager detection
  - `src/security/plugins/` — Scanner plugins including trivy/grype (from story 03-003)
  - `src/governance/` — Governance engine (from story 04-003) for prefer-podman rules
- **Repository conventions:** Module by feature/domain. Use tokio for async subprocess calls.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/containers/mod.rs` — Container package support engine
- Create `src/containers/pull.rs` — Image pull (docker pull / podman pull)
- Create `src/containers/scan.rs` — Container image scanning (trivy/grype)
- Create `src/containers/list.rs` — List local images (docker images / podman images)
- Create `src/containers/detect.rs` — Container usage detection from project files
- Implement `apmw add <image>` — pull a container image via docker or podman
- Implement `apmw add <image> --dev` — dev-only container image
- Implement `apmw scan <image>` — run security scanning via trivy/grype
- Implement `apmw list --manager docker` — list local container images
- Detect container usage from: Dockerfile, docker-compose.yml, docker-compose.*.yml, Containerfile, compose.yaml
- Support docker-compose as an orchestration package manager
- Support governance rules for containers (prefer-podman over docker via governance engine)
- Add unit tests for pull, scan, list, and detection
- Add integration tests with mock docker/podman commands

**Out of scope:**
- Scanner plugin implementation (story 03-003)
- Governance engine (story 04-003)
- Add engine for non-container packages (story 04-001)

## Sub-Tasks

- [ ] Create `src/containers/mod.rs` with ContainerEngine
  **Verify**: `cargo check` → exit 0
- [ ] Create `src/containers/pull.rs` with image pull (docker pull / podman pull)
  **Verify**: `cargo test container_pull` → tests pass
- [ ] Create `src/containers/scan.rs` with image scanning (trivy/grype)
  **Verify**: `cargo test container_scan` → tests pass
- [ ] Create `src/containers/list.rs` with local image listing
  **Verify**: `cargo test container_list` → tests pass
- [ ] Create `src/containers/detect.rs` with container usage detection
  **Verify**: `cargo test container_detect` → tests pass
- [ ] Wire `apmw add <image>` to container pull
  **Verify**: `cargo test add_container` → tests pass
- [ ] Wire `apmw add <image> --dev` for dev-only images
  **Verify**: `cargo test add_container_dev` → tests pass
- [ ] Wire `apmw scan <image>` to trivy/grype scanning
  **Verify**: `cargo test scan_container` → tests pass
- [ ] Wire `apmw list --manager docker` to local image listing
  **Verify**: `cargo test list_containers` → tests pass
- [ ] Implement Dockerfile detection
  **Verify**: `cargo test detect_dockerfile` → tests pass
- [ ] Implement docker-compose.yml / docker-compose.*.yml / compose.yaml detection
  **Verify**: `cargo test detect_compose` → tests pass
- [ ] Implement Containerfile detection
  **Verify**: `cargo test detect_containerfile` → tests pass
- [ ] Support docker-compose as orchestration package manager
  **Verify**: `cargo test docker_compose` → tests pass
- [ ] Support governance prefer-podman over docker
  **Verify**: `cargo test governance_podman` → tests pass
- [ ] Add integration tests with mock docker/podman commands
  **Verify**: `just test` → all pass
- [ ] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/containers/mod.rs` — Container engine
- `src/containers/pull.rs` — Image pull
- `src/containers/scan.rs` — Image scanning
- `src/containers/list.rs` — Local image listing
- `src/containers/detect.rs` — Container usage detection
- `src/lib.rs` — Add `pub mod containers;`

## Acceptance Criteria

- [ ] `apmw add nginx:latest --manager docker` pulls the image via docker
- [ ] `apmw add <image> --dev` marks the image as dev-only
- [ ] `apmw scan <image>` runs trivy/grype security scanning
- [ ] `apmw list --manager docker` lists local container images
- [ ] Dockerfile detection works (detects container usage)
- [ ] docker-compose.yml detection works (detects orchestration usage)
- [ ] docker-compose.*.yml and compose.yaml detection works
- [ ] Containerfile detection works
- [ ] docker-compose is supported as an orchestration package manager
- [ ] Governance prefer-podman routes docker calls through podman
- [ ] All unit and integration tests pass
- [ ] `just validate` passes

## Test Plan

- Unit: `cargo test containers` — all container support tests
- Integration: Mock docker/podman subprocess calls
- Lint: `just lint`

## Observability

- Container pulls should be logged at `info` level
- Container scan results should be logged at `warn` level for vulnerable images
- Container detection results should be logged at `debug` level
- Governance routing (prefer-podman) should be logged at `info` level

## Compliance

- PRD FR-7 (container package support)

## Risks & Mitigations

- Risk: docker/podman may not be installed — Mitigation: Check availability before proceeding; error with a clear message listing available container runtimes
- Risk: Large image pulls may take a long time — Mitigation: Support daemon mode for background pulls; show progress

## Dependencies & Sequencing

- Depends on: 02-001 (detection engine), 03-003 (scanner plugins including trivy/grype)
- Unblocks: 06-001 (MCP server — container scan tool)

## Definition of Done

- [ ] All verification commands pass
- [ ] Code, tests, docs updated; CI green; story file updated
- [ ] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- docker and podman are both unavailable in the test environment
- trivy/grype cannot scan container images as expected

## Maintenance Notes

- New container runtimes can be added by extending the runtime detection
- Reviewers should check that governance prefer-podman works correctly
- Compose file format variations should be handled gracefully

## Commit Conventions

- `feat(containers): add container package support with docker/podman pull, scan, list, and compose detection`
