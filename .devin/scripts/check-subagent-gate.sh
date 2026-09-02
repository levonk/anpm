#!/usr/bin/env bash
# check-subagent-gate.sh — PreToolUse hook for run_subagent
#
# Reads stdin (hook event JSON), checks if a gate-pass file exists.
# If it does, approves. If not, blocks with a message telling the agent
# to run execution-gate.sh first.
#
# This enforces: NO write-capable subagent dispatch without a worktree +
# checkpoint. Read-only research subagents (subagent_explore) are exempt —
# they cannot mutate the working directory (profile-enforced tool access:
# grep, glob, read, web_search only), so the worktree protection rationale
# does not apply. Exempting them prevents false-positive blocks on research
# dispatches, which otherwise motivates agents to abandon subagents and do
# research inline (a circumvention of the gate, not compliance).
#
# Bypass: SKILL_BYPASS_GATE=1 environment variable. This is session-level —
# set it when launching the Devin session (e.g. `SKILL_BYPASS_GATE=1 devin`),
# not from an agent command. The hook inherits the session environment, so
# the agent cannot inject it mid-session. This gives the user an asymmetric
# bypass: the user controls it at session start, the agent cannot.
#
# Exit code 2 = block. Exit code 0 = allow.
set -euo pipefail

# --- Bypass: session-level env var (user-only, not agent-injectable) ---
if [[ "${SKILL_BYPASS_GATE:-0}" == "1" ]]; then
  exit 0
fi

GATE_DIR="/tmp/devin-execution-gates"
GATE_PASS="$GATE_DIR/current-gate-pass"

# Read hook event from stdin
INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('tool_name',''))" 2>/dev/null || echo "")

# Only enforce for run_subagent
if [[ "$TOOL_NAME" != "run_subagent" ]]; then
  exit 0
fi

# --- Read-only profile exemption ---
# subagent_explore has profile-enforced read-only tool access (grep, glob,
# read, web_search — no edit/write/exec). It cannot mutate the working
# directory, so the worktree gate does not apply. The profile is enforced
# at the tool layer, not the prompt layer — an agent cannot escalate a
# subagent_explore dispatch to write capability by prompt content alone.
PROFILE=$(echo "$INPUT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('tool_input',{}).get('profile',''))" 2>/dev/null || echo "")
if [[ "$PROFILE" == "subagent_explore" ]]; then
  exit 0
fi

# Check for gate pass
if [[ -f "$GATE_PASS" ]]; then
  WORKTREE_PATH=$(cat "$GATE_PASS")
  # Verify the worktree still exists
  if [[ -d "$WORKTREE_PATH" ]]; then
    exit 0
  else
    echo "{\"decision\": \"block\", \"reason\": \"Execution gate: worktree at $WORKTREE_PATH no longer exists. Run execution-gate.sh again before dispatching.\"}"
    exit 2
  fi
fi

# No gate pass — block
cat <<'JSON'
{"decision": "block", "reason": "EXECUTION GATE: You are about to dispatch a write-capable subagent WITHOUT a worktree. This violates the execute-upsert binding contract. Run: bash .devin/scripts/execution-gate.sh <story-id-slug> [base-sha] — this creates a per-story git worktree + checkpoint commit. The subagent MUST work in that worktree, not in the main checkout. After the gate passes, re-dispatch. This hook will block every write-capable run_subagent call until the gate is satisfied. NOTE: read-only research subagents (profile=subagent_explore) are exempt and do not require a gate — do NOT abandon subagent dispatch to avoid this gate; that is circumvention, not compliance. Bypass (user only): launch the session with SKILL_BYPASS_GATE=1."}
JSON
exit 2
