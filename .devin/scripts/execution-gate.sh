#!/usr/bin/env bash
# execution-gate.sh — Pre-flight gate for execute-upsert subagent dispatch
#
# Creates a per-story git worktree + checkpoint commit BEFORE dispatching a subagent.
# Writes a gate-pass file that the PreToolUse hook on run_subagent checks.
# If this script does not exit 0, the dispatch MUST NOT proceed.
#
# This is the machine-enforced gate. The PreToolUse hook (check-subagent-gate.sh)
# blocks every run_subagent call until this script has run and written a gate-pass
# file. The agent cannot rationalize past it — it is a turnstile, not a sign.
#
# Worktree acquisition uses treehouse (https://github.com/kunchenguid/treehouse)
# when available — a pool manager that reuses worktrees with dependencies and
# build cache intact. When treehouse is not installed, falls back to manual
# `git worktree add`. After acquiring a worktree (either way), a named story
# branch is created inside it with `git checkout -b`.
#
# Usage:
#   bash .devin/scripts/execution-gate.sh <story-id-slug> [base-sha] [--story-type <trivial|standard|research>]
#
# Outputs (stdout): the worktree path (for the orchestrator to pass to the subagent)
# Outputs (stderr): progress/diagnostic messages
# Exit codes:
#   0 = gate passed, worktree ready
#   1 = usage error
#   2 = pre-condition failure (uncommitted changes on main, missing base SHA, etc.)
#   3 = worktree already exists (informational — see below)
set -euo pipefail

PROJECT_DIR="${DEVIN_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
PROJECT_NAME="$(basename "$PROJECT_DIR")"
GATE_DIR="/tmp/devin-execution-gates"
GATE_PASS="$GATE_DIR/current-gate-pass"
WORKTREE_BASE="/tmp/${PROJECT_NAME}-worktrees"

# Include shared treehouse helpers (treehouse_available, treehouse_acquire, etc.)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# treehouse-helpers.sh — shared helpers for treehouse worktree pool management
#
# Treehouse (https://github.com/kunchenguid/treehouse) manages a pool of
# reusable, isolated git worktrees so each agent gets its own environment
# instantly — no cloning, no conflicts, no coordination overhead.
#
# This include provides thin wrappers around the treehouse CLI for use by
# execution-gate.sh and other scripts that need to acquire or release
# worktrees. When treehouse is not available, callers fall back to manual
# `git worktree add` commands.
#
# Functions provided:
#   treehouse_available       — check if treehouse is on PATH (echoes 1/0)
#   treehouse_acquire         — lease a worktree, echo its path (sets TREEHOUSE_LEASE_ID)
#   treehouse_acquire_json    — lease a worktree, echo JSON {path, lease_id, lease_holder, leased_at}
#   treehouse_return          — release a lease and return worktree to pool
#   treehouse_status          — print pool status (JSON or text)
#   treehouse_is_managed      — check if a path is inside a treehouse-managed worktree
#
# Environment variables:
#   TREEHOUSE_LEASE_HOLDER   — label recorded as the lease holder (default: $USER or "agent")
#   TREEHOUSE_NO_FETCH       — if set to 1, pass --no-fetch to treehouse get
#
# Materialization: each consuming skill has a
# `scripts/treehouse-helpers.sh.tmpl` file containing a single include
# directive that pulls in this file. The templater inlines this file at build
# time. Scripts then `source` the materialized copy from the same `scripts/`
# directory.
#
# Consumers:
#   - execution/execute-upsert/scripts/execution-gate.sh.tmpl
#   - execution/execute-upsert/scripts/land-on-env-dev.sh.tmpl
#   - execution/execute-upsert/scripts/ship-pr.sh.tmpl

# Guard against double-sourcing
_TREEHOUSE_HELPERS_SOURCED="${_TREEHOUSE_HELPERS_SOURCED:-}"

# treehouse_available — check if treehouse CLI is available.
# Echoes 1 if available, 0 if not. Does not use `return` so it works both
# when sourced and when inlined.
treehouse_available() {
	if command -v treehouse >/dev/null 2>&1; then
		echo 1
	else
		echo 0
	fi
}

# treehouse_acquire — lease a worktree from the pool.
# Echoes the absolute path of the leased worktree to stdout.
# Sets TREEHOUSE_LEASE_ID and TREEHOUSE_LEASE_PATH in the environment.
# Returns 0 on success, 1 on failure.
#
# Optional args:
#   $1 — lease holder label (defaults to $TREEHOUSE_LEASE_HOLDER or $USER or "agent")
treehouse_acquire() {
	local holder="${1:-${TREEHOUSE_LEASE_HOLDER:-${USER:-agent}}}"
	local extra_args=()
	if [[ "${TREEHOUSE_NO_FETCH:-0}" == "1" ]]; then
		extra_args+=(--no-fetch)
	fi

	# treehouse get --lease prints the path to stdout, banners to stderr.
	# --json gives us path + lease_id for programmatic use.
	local json_out
	json_out="$(treehouse get --lease --lease-holder "$holder" --json "${extra_args[@]}" 2>/dev/null)" || {
		echo "TREEHOUSE: failed to acquire worktree" >&2
		return 1
	}

	# Parse JSON output (path, lease_id, lease_holder, leased_at)
	# Use jq if available, otherwise fall back to grep/sed
	if command -v jq >/dev/null 2>&1; then
		TREEHOUSE_LEASE_PATH="$(printf '%s' "$json_out" | jq -r '.path // empty')"
		TREEHOUSE_LEASE_ID="$(printf '%s' "$json_out" | jq -r '.lease_id // empty')"
		TREEHOUSE_LEASE_HOLDER="$(printf '%s' "$json_out" | jq -r '.lease_holder // empty')"
	else
		TREEHOUSE_LEASE_PATH="$(printf '%s' "$json_out" | grep -o '"path":"[^"]*"' | head -1 | sed 's/"path":"//;s/"//')"
		TREEHOUSE_LEASE_ID="$(printf '%s' "$json_out" | grep -o '"lease_id":"[^"]*"' | head -1 | sed 's/"lease_id":"//;s/"//')"
		TREEHOUSE_LEASE_HOLDER="$(printf '%s' "$json_out" | grep -o '"lease_holder":"[^"]*"' | head -1 | sed 's/"lease_holder":"//;s/"//')"
	fi

	if [[ -z "$TREEHOUSE_LEASE_PATH" ]]; then
		echo "TREEHOUSE: no path in lease output" >&2
		return 1
	fi

	export TREEHOUSE_LEASE_PATH
	export TREEHOUSE_LEASE_ID
	export TREEHOUSE_LEASE_HOLDER
	echo "$TREEHOUSE_LEASE_PATH"
}

# treehouse_return — release a lease and return a worktree to the pool.
# Terminates lingering processes, verifies no foreign process remains,
# resets the worktree, and clears the lease.
#
# Args:
#   $1 — worktree path (required)
#   $2 — lease ID (optional, for ABA-safe return via --if-lease-id)
#   $3 — lease holder (optional, for --if-lease-holder)
# Returns 0 on success, 1 on failure.
treehouse_return() {
	local path="$1"
	local lease_id="${2:-}"
	local holder="${3:-}"

	if [[ -z "$path" ]]; then
		echo "TREEHOUSE: return requires a worktree path" >&2
		return 1
	fi

	local args=(--force)
	if [[ -n "$lease_id" ]]; then
		args+=(--if-lease-id "$lease_id")
	fi
	if [[ -n "$holder" ]]; then
		args+=(--if-lease-holder "$holder")
	fi

	treehouse return "${args[@]}" "$path" 2>&1 || {
		echo "TREEHOUSE: failed to return worktree at $path" >&2
		return 1
	}
}

# treehouse_status — print pool status.
# Args:
#   --json — print JSON status (default is human-readable text)
treehouse_status() {
	if [[ "${1:-}" == "--json" ]]; then
		treehouse status --json 2>/dev/null
	else
		treehouse status 2>&1
	fi
}

# treehouse_is_managed — check if a path is a treehouse-managed worktree.
#
# This is the authoritative check: it asks treehouse itself via
# `treehouse status --json`, which lists all worktrees in the pool with their
# paths. This works regardless of where the treehouse root is configured
# (default ~/.treehouse, repo-level treehouse.toml, user-level
# ~/.config/treehouse/config.toml, --root flag, or environment variables).
#
# Echoes 1 if the path matches a treehouse-managed worktree, 0 if not.
treehouse_is_managed() {
	local path="${1:-$PWD}"
	local abs_path
	abs_path="$(cd "$path" 2>/dev/null && pwd)" || { echo 0; return 0; }

	# If treehouse is not available, the path cannot be treehouse-managed.
	if ! command -v treehouse >/dev/null 2>&1; then
		echo 0
		return 0
	fi

	# Ask treehouse for all managed worktree paths.
	# `treehouse status --json` returns a JSON array with objects containing
	# "path" fields. We extract the paths and check if ours is among them.
	# This is authoritative — no hard-coded root paths, no config file parsing.
	local status_json
	status_json="$(treehouse status --json 2>/dev/null)" || { echo 0; return 0; }

	if [[ -z "$status_json" ]] || [[ "$status_json" == "null" ]]; then
		echo 0
		return 0
	fi

	# Extract paths from JSON. Use jq when available, otherwise grep/sed.
	local managed_paths
	if command -v jq >/dev/null 2>&1; then
		managed_paths="$(printf '%s' "$status_json" | jq -r '.[].path // empty' 2>/dev/null)"
	else
		# Fallback: extract "path":"..." values from the JSON array
		managed_paths="$(printf '%s' "$status_json" | grep -o '"path":"[^"]*"' | sed 's/"path":"//;s/"//')"
	fi

	if [[ -z "$managed_paths" ]]; then
		echo 0
		return 0
	fi

	# Check if abs_path matches or is inside any managed worktree path.
	# We check both exact match and prefix (in case the caller is in a
	# subdirectory of the worktree).
	local mp
	while IFS= read -r mp; do
		[[ -z "$mp" ]] && continue
		if [[ "$abs_path" == "$mp" ]] || [[ "$abs_path" == "$mp"/* ]]; then
			echo 1
			return 0
		fi
	done <<< "$managed_paths"

	echo 0
}


# --- Args ---
# Parse positional args + optional --story-type flag.
# Usage: execution-gate.sh <story-id-slug> [base-sha] [--story-type <type>]
# The --story-type flag is a forward-compatible metadata tag (trivial|standard|research).
# Today all types are treated identically — the value is stored in the gate-pass
# file for future divergence. If divergence is needed, the behavior change is a
# single case statement here, not a handoff document re-edit.
STORY_TYPE="standard"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --story-type)
      STORY_TYPE="$2"
      shift 2
      ;;
    --story-type=*)
      STORY_TYPE="${1#--story-type=}"
      shift
      ;;
    *)
      if [[ -z "${STORY_SLUG:-}" ]]; then
        STORY_SLUG="$1"
      elif [[ -z "${BASE_SHA:-}" ]]; then
        BASE_SHA="$1"
      else
        echo "Unexpected argument: $1" >&2
        echo "Usage: execution-gate.sh <story-id-slug> [base-sha] [--story-type <trivial|standard|research>]" >&2
        exit 1
      fi
      shift
      ;;
  esac
done

if [[ -z "${STORY_SLUG:-}" ]]; then
  echo "Usage: execution-gate.sh <story-id-slug> [base-sha] [--story-type <trivial|standard|research>]" >&2
  exit 1
fi

# Validate story type
case "$STORY_TYPE" in
  trivial|standard|research) ;;
  *)
    echo "BLOCK: Invalid story type '$STORY_TYPE'. Must be one of: trivial, standard, research" >&2
    exit 1
    ;;
esac

BASE_SHA="${BASE_SHA:-HEAD}"
WORKTREE_PATH="$WORKTREE_BASE/$STORY_SLUG"
STORY_BRANCH="feature/current/execute-upsert/$STORY_SLUG"

mkdir -p "$GATE_DIR"

# --- Check 1: not on main with uncommitted changes ---
CURRENT_BRANCH=$(git -C "$PROJECT_DIR" rev-parse --abbrev-ref HEAD)
if [[ "$CURRENT_BRANCH" == "main" || "$CURRENT_BRANCH" == "master" ]]; then
  DIRTY=$(git -C "$PROJECT_DIR" status --porcelain | wc -l | tr -d ' ')
  if [[ "$DIRTY" -gt 0 ]]; then
    echo "BLOCK: You are on $CURRENT_BRANCH with $DIRTY uncommitted changes." >&2
    echo "Create a checkpoint commit or stash before dispatching subagents." >&2
    echo "Run: git -C \"$PROJECT_DIR\" stash or commit your changes first." >&2
    exit 2
  fi
fi

# --- Check 2: base SHA exists ---
if ! git -C "$PROJECT_DIR" rev-parse --verify "$BASE_SHA" >/dev/null 2>&1; then
  echo "BLOCK: Base SHA '$BASE_SHA' does not exist." >&2
  exit 2
fi

# --- Check 3: worktree already exists (resume case) ---
# For treehouse-managed worktrees, we check the gate-pass file for a lease path.
# For manual worktrees, we check git worktree list.
if [[ -f "$GATE_PASS" ]]; then
  EXISTING_PATH="$(cat "$GATE_PASS")"
  if [[ -d "$EXISTING_PATH" ]]; then
    # Verify it's still a valid worktree
    if git -C "$PROJECT_DIR" worktree list 2>/dev/null | grep -q "$EXISTING_PATH"; then
      echo "RESUME: Worktree already exists at $EXISTING_PATH" >&2
      echo "$EXISTING_PATH"
      exit 0
    fi
    # Treehouse worktree — check if lease is still active
    if [[ "$(treehouse_available)" == "1" ]] && [[ "$(treehouse_is_managed "$EXISTING_PATH")" == "1" ]]; then
      echo "RESUME: Treehouse worktree still leased at $EXISTING_PATH" >&2
      echo "$EXISTING_PATH"
      exit 0
    fi
  fi
fi

# Also check legacy manual worktree path
if git -C "$PROJECT_DIR" worktree list | grep -q "$WORKTREE_PATH"; then
  echo "RESUME: Worktree already exists at $WORKTREE_PATH" >&2
  echo "$WORKTREE_PATH" > "$GATE_PASS"
  echo "$WORKTREE_PATH"
  exit 0
fi

# --- Acquire worktree ---
# Treehouse is required (not optional). Treehouse makes worktrees cheap —
# reusable, cache-warmed, pool-managed. The manual `git worktree add`
# fallback was removed because it created a two-path maintenance burden and
# the manual path lacked lease tracking, cache warming, and pool reuse.
# If treehouse is not installed, the gate fails with a clear install
# instruction rather than silently degrading.
TREEHOUSE_USED=0

if [[ "$(treehouse_available)" != "1" ]]; then
  echo "BLOCK: treehouse is not installed but is required for worktree isolation." >&2
  echo "Treehouse makes worktrees cheap (reusable, cache-warmed, pool-managed)." >&2
  echo "Install: https://github.com/kunchenguid/treehouse" >&2
  echo "Or: brew install treehouse (if available on tap)" >&2
  echo "Or: cargo install treehouse (if Rust toolchain is available)" >&2
  exit 2
fi

echo "[gate] Acquiring worktree via treehouse pool..." >&2
TREEHOUSE_LEASE_HOLDER="execute-upsert/${STORY_SLUG}"
ACQUIRED_PATH="$(treehouse_acquire "execute-upsert/${STORY_SLUG}" 2>&1)" || {
  echo "[gate] Treehouse acquire failed" >&2
  echo "$ACQUIRED_PATH" >&2
  exit 2
}

if [[ -n "$ACQUIRED_PATH" ]]; then
  WORKTREE_PATH="$ACQUIRED_PATH"
  TREEHOUSE_USED=1
  echo "[gate] Treehouse worktree leased: $WORKTREE_PATH" >&2
  echo "[gate] Lease ID: ${TREEHOUSE_LEASE_ID:-unknown}" >&2

  # Record the lease info for later return
  cat > "$GATE_DIR/lease-${STORY_SLUG}" <<EOF
${TREEHOUSE_LEASE_ID:-}
${TREEHOUSE_LEASE_HOLDER:-}
${WORKTREE_PATH}
EOF
else
  echo "BLOCK: treehouse acquire returned empty path" >&2
  exit 2
fi

# --- Create named story branch inside the worktree ---
# Treehouse worktrees start in detached HEAD at the default branch tip.
# We need a named branch for the story so pushes, PRs, and land-on-env-dev work.
if [[ "$TREEHOUSE_USED" -eq 1 ]]; then
  # The worktree is at detached HEAD at the default branch tip.
  # Check out the base SHA, then create the story branch.
  git -C "$WORKTREE_PATH" checkout "$BASE_SHA" 2>&1 | sed 's/^/[gate] /' >&2 || {
    echo "[gate] WARNING: could not checkout $BASE_SHA, staying at default branch tip" >&2
  }
  git -C "$WORKTREE_PATH" checkout -b "$STORY_BRANCH" 2>&1 | sed 's/^/[gate] /' >&2 || {
    # Branch may already exist if this is a re-acquired worktree
    echo "[gate] Branch $STORY_BRANCH may already exist, switching to it" >&2
    git -C "$WORKTREE_PATH" checkout "$STORY_BRANCH" 2>&1 | sed 's/^/[gate] /' >&2 || true
  }
fi

# --- Symlink node_modules if it exists (so the worktree can build/test) ---
# Treehouse worktrees may already have this from pool reuse, but symlink if missing.
if [[ -d "$PROJECT_DIR/node_modules" ]] && [[ ! -e "$WORKTREE_PATH/node_modules" ]]; then
  ln -sfn "$PROJECT_DIR/node_modules" "$WORKTREE_PATH/node_modules" 2>/dev/null || true
fi

# --- Create checkpoint commit in the worktree ---
# (The worktree starts clean from BASE_SHA, so the checkpoint is implicit.
#  But we record the checkpoint SHA for rollback.)
CHECKPOINT_SHA=$(git -C "$WORKTREE_PATH" rev-parse HEAD)
echo "$CHECKPOINT_SHA" > "$GATE_DIR/checkpoint-$STORY_SLUG"

# --- Write gate pass file ---
echo "$WORKTREE_PATH" > "$GATE_PASS"

# Record whether treehouse was used (for land-on-env-dev to know whether to return the lease)
echo "$TREEHOUSE_USED" > "$GATE_DIR/treehouse-used-${STORY_SLUG}"

# Record story type as metadata (forward-compatible tag — currently unused behaviorally)
echo "$STORY_TYPE" > "$GATE_DIR/story-type-${STORY_SLUG}"

echo "GATE PASSED: worktree=$WORKTREE_PATH branch=$STORY_BRANCH checkpoint=$CHECKPOINT_SHA treehouse=$TREEHOUSE_USED story-type=$STORY_TYPE" >&2
echo "$WORKTREE_PATH"
