#!/usr/bin/env bash
# inject-binding-contract.sh — UserPromptSubmit hook
#
# Reads the user's prompt from stdin. If it mentions execute-upsert, execution,
# "run the workflow", or similar execution triggers, injects the binding contract
# as additionalContext. This keeps the non-negotiable rules in active attention
# at the start of every execution session.
#
# The contract source of truth is references/execute-upsert.contract.yaml
# (an AgentContract .contract.yaml file). This script is the renderer: it reads
# the YAML and formats it into the additionalContext string. A short inline
# fallback is kept for standalone installs where references/ is not materialized.
set -euo pipefail

INPUT=$(cat)
PROMPT=$(echo "$INPUT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('prompt','').lower())" 2>/dev/null || echo "")

# Check if this prompt is about execution
KEYWORDS="execute-upsert|execute.upsert|run.*workflow|implement.*feature|drive.*to.completion|run.*prd|execute.*story|dispatch.*subagent"
if ! echo "$PROMPT" | grep -qE "$KEYWORDS"; then
  exit 0
fi

# Resolve the contract YAML relative to this script's location.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONTRACT_FILE="$SCRIPT_DIR/../references/execute-upsert.contract.yaml"

# Render the contract YAML into a prose block for additionalContext.
# Falls back to an inline summary if the YAML file is missing (standalone installs).
render_contract() {
  if [ -f "$CONTRACT_FILE" ]; then
    python3 - "$CONTRACT_FILE" <<'PYEOF'
import sys, re, yaml

path = sys.argv[1]
with open(path) as f:
    doc = yaml.safe_load(f)

lines = []
lines.append("=== EXECUTE-UPSERT BINDING CONTRACT (NON-NEGOTIABLE) ===")
lines.append("")
desc = (doc.get("description") or "").strip().replace("\n", " ")
if desc:
    lines.append(desc)
    lines.append("")
lines.append("Machine-enforced via .devin/hooks.v1.json + execution-gate.sh.")
lines.append("")

def render_section(key, label):
    items = doc.get(key, [])
    if not items:
        return
    lines.append(label)
    for i, item in enumerate(items, 1):
        sev = item.get("severity", "")
        enf = item.get("enforcement", "")
        clause = item.get("clause", "")
        check = item.get("check", "")
        meta = f" [{sev}/{enf}]"
        lines.append(f"{i}. {clause}{meta}")
        if check:
            lines.append(f"   enforcement: {check}")
    lines.append("")

render_section("must", "MUST:")
render_section("must not", "MUST NOT:")
render_section("may", "MAY:")

lines.append("The full INSTRUCTIONS.md is reference material. THIS CONTRACT is the active law.")
lines.append("=== END BINDING CONTRACT ===")

# Escape newlines for JSON string embedding
text = "\n".join(lines)
print(text.replace("\\", "\\\\").replace("\n", "\\n").replace("\"", "\\\""))
PYEOF
  else
    # Inline fallback — kept short for standalone installs without references/
    cat <<'FALLBACK'
=== EXECUTE-UPSERT BINDING CONTRACT (NON-NEGOTIABLE) ===\n\nMachine-enforced via .devin/hooks.v1.json + execution-gate.sh.\n\nMUST:\n1. Every subagent dispatch MUST go through execution-gate.sh FIRST [block/machine]\n2. The subagent MUST work ONLY in the worktree path returned by the gate [block/machine]\n3. BEFORE dispatching: create a checkpoint commit on the current branch [rollback/machine]\n4. AFTER each story completes: commit on story branch, merge back with --no-ff, remove worktree [block/llm-judged]\n5. If a subagent fails, roll back to the checkpoint [rollback/llm-judged]\n\nMUST NOT:\n1. NEVER commit directly on main/master with uncommitted changes from a prior session [block/machine]\n2. NEVER work on stories directly in the main checkout [block/machine]\n\nMAY:\n1. Up to 5 simultaneous subagents (API rate-limit safety) [warn/llm-judged]\n2. Gate may be bypassed with SKILL_BYPASS_GATE=1 [warn/machine]\n\nThe full INSTRUCTIONS.md is reference material. THIS CONTRACT is the active law.\n=== END BINDING CONTRACT ===
FALLBACK
  fi
}

CONTEXT=$(render_contract)

cat <<JSON
{
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": "$CONTEXT"
  }
}
JSON
