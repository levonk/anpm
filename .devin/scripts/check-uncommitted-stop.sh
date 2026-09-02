#!/usr/bin/env bash
# check-uncommitted-stop.sh — Stop hook
#
# Fires when the agent wants to stop. Checks for uncommitted changes on main.
# If found, blocks with a message telling the agent to commit first.
# Uses decision=block (not exit 2) to allow the agent to retry.
#
# To prevent infinite loops, only blocks once per session (tracks via marker file).
set -euo pipefail

PROJECT_DIR="${DEVIN_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
LOOP_GUARD="/tmp/devin-execution-gates/stop-hook-fired"

# If we already fired this session, don't block again (avoid loops)
if [[ -f "$LOOP_GUARD" ]]; then
  exit 0
fi

# Check if we're in a git repo
if ! git -C "$PROJECT_DIR" rev-parse --git-dir >/dev/null 2>&1; then
  exit 0
fi

CURRENT_BRANCH=$(git -C "$PROJECT_DIR" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

# Only enforce on main/master
if [[ "$CURRENT_BRANCH" != "main" && "$CURRENT_BRANCH" != "master" ]]; then
  exit 0
fi

DIRTY=$(git -C "$PROJECT_DIR" status --porcelain 2>/dev/null | wc -l | tr -d ' ')
if [[ "$DIRTY" -gt 0 ]]; then
  mkdir -p /tmp/devin-execution-gates
  touch "$LOOP_GUARD"
  cat <<JSON
{"decision": "block", "reason": "UNCOMMITTED CHANGES ON MAIN: You have $DIRTY uncommitted file(s) on the $CURRENT_BRANCH branch. The execute-upsert protocol requires committing at story boundaries. Either: (1) commit your changes with a conventional commit message, or (2) if these are not your changes, note them and stash. Do not stop with a dirty main branch."}
JSON
  exit 2
fi

exit 0
