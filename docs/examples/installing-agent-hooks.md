# Installing Agent Hooks

This example shows how to install PATH shims (hard intercept) and agent
skills.

## Install PATH shims (hard intercept)

```bash
$ apmw --install --intercept
Installed 4 intercept shim(s) to /Users/me/.local/share/apmw/shims

Add this directory to the FRONT of your PATH:
  export PATH="/Users/me/.local/share/apmw/shims:$PATH"

To remove the shims later: apmw --uninstall --intercept
```

The shims intercept package manager calls (e.g., `pip install foo`). When
intercepted, apmw:

1. **Evaluates governance rules** (prefer/force/block/eject) from the
   levonk-packages spec.
2. **Runs security scanning** if needed.
3. **Outputs the effective tool and args** (NUL-separated) for the shim to
   exec.

For example, if governance says "prefer uv over pip", running
`pip install foo` will actually exec `uv install foo`.

## Remove PATH shims

```bash
$ apmw --uninstall --intercept
Removed 4 intercept shim(s) from /Users/me/.local/share/apmw/shims
```

## How intercept works

When you run `pip install foo`:

1. The shim at `~/.local/share/apmw/shims/pip` intercepts the call.
2. The shim invokes `apmw intercept pip install foo`.
3. apmw evaluates governance (e.g., prefer `uv`), runs security scanning,
   and outputs: `uv\0install\0foo` (NUL-separated).
4. The shim execs `uv install foo`.

The `HookManager` in `src/agent/` manages shim installation and the
intercept logic. The `InterceptAction` and `InterceptDecision` types
represent the governance decision.

## Agent skills

apmw generates skill files for AI agent sessions. These describe apmw's
capabilities so agents know how to use it. The `SkillGenerator` in
`src/agent/` produces a `SKILL.md` file.

Session integrations install skills and MCP configs into agent-specific
locations:

- **Claude Code** — `.claude/` directory
- **Codex** — agent config
- **OpenCode** — agent config

Use `install_all_session_integrations` to install for all supported agents.

## Governance refresh

Keep the governance spec up to date:

```bash
$ apmw governance refresh
Governance spec refreshed (version: 2026.07).
Loaded 12 governance rule(s):
  prefer pip — Use uv instead of pip
  prefer npm — Use pnpm instead of npm
  ...
```
