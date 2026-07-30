read ~/p/gh/levonk/apmw/AGENTS.md
Run ~/p/gh/levonk/skills-src/build/current/skills/execution/execute-upsert
on ~/p/gh/levonk/apmw/internal-docs/feature/2026/07/apmw/tasks/index-apmw.md
do not use `npx` or `npm` we use `devbox run -- pnpm dlx` or `devbox run -- pnpm` 
execute-upsert says to not stop processing, via subagents, until all the stories and tasks are addressed, this means marking thingss blocked until there is no more work you can do