#compdef apmw

autoload -U is-at-least

_apmw() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" : \
'--shell=[Shell to generate completions for (used with --install)]:SHELL:_default' \
'--color=[Color output\: auto, always, never]:WHEN:_default' \
'--on-risk=[Action to take when a security scan detects risk]:ACTION:_default' \
'--fields=[Comma-separated list of fields to include in AXI minimal output]:LIST:_default' \
'--cancel-job=[Cancel a background job by ID]:ID:_default' \
'--manager=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--use=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--install[Generate shell completions and initialize the config file]' \
'--uninstall[Remove generated completions and config (best-effort cleanup)]' \
'--intercept[Install or remove PATH shims that intercept package manager calls (used with \`--install\` or \`--uninstall\`)]' \
'--man[Print the man page to stdout (groff/troff format) and exit]' \
'--usage[Print a brief usage summary and exit]' \
'--json[Emit output as JSON (machine-readable)]' \
'--human[Human-readable output (escape hatch for AXI/TOON mode)]' \
'*-v[Increase verbosity (can be repeated\: -v, -vv)]' \
'*--verbose[Increase verbosity (can be repeated\: -v, -vv)]' \
'-q[Suppress non-error output]' \
'--quiet[Suppress non-error output]' \
'--debug[Enable debug-level diagnostics]' \
'--dry-run[Show what would happen without making changes]' \
'--force[Force the operation, bypassing confirmations]' \
'--interactive[Enable interactive/TUI mode]' \
'--tui[Enable interactive/TUI mode]' \
'--no-pager[Disable pager output]' \
'--no-scan[Skip security scanning]' \
'--scan-only[Only run the security scan, then exit]' \
'--update-security-db[Update the security vulnerability database before scanning]' \
'--full[Show full content (escape hatch for AXI truncation)]' \
'--daemon[Run in daemon mode (long-running background process)]' \
'--no-daemon[Disable daemon mode (force synchronous operation)]' \
'--list-jobs[List background jobs]' \
'--no-telemetry[Disable telemetry collection for this invocation]' \
'--telemetry-preview[Print the telemetry payload that would be sent without actually sending it]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
'-V[Print version]' \
'--version[Print version]' \
":: :_apmw_commands" \
"*::: :->apmw" \
&& ret=0
    case $state in
    (apmw)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:apmw-command-$line[1]:"
        case $line[1] in
            (install)
_arguments "${_arguments_options[@]}" : \
'--color=[Color output\: auto, always, never]:WHEN:_default' \
'--on-risk=[Action to take when a security scan detects risk]:ACTION:_default' \
'--fields=[Comma-separated list of fields to include in AXI minimal output]:LIST:_default' \
'--cancel-job=[Cancel a background job by ID]:ID:_default' \
'--manager=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--use=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--dev[Install as a development/build-time dependency]' \
'--json[Emit output as JSON (machine-readable)]' \
'--human[Human-readable output (escape hatch for AXI/TOON mode)]' \
'*-v[Increase verbosity (can be repeated\: -v, -vv)]' \
'*--verbose[Increase verbosity (can be repeated\: -v, -vv)]' \
'-q[Suppress non-error output]' \
'--quiet[Suppress non-error output]' \
'--debug[Enable debug-level diagnostics]' \
'--dry-run[Show what would happen without making changes]' \
'--force[Force the operation, bypassing confirmations]' \
'--interactive[Enable interactive/TUI mode]' \
'--tui[Enable interactive/TUI mode]' \
'--no-pager[Disable pager output]' \
'--no-scan[Skip security scanning]' \
'--scan-only[Only run the security scan, then exit]' \
'--update-security-db[Update the security vulnerability database before scanning]' \
'--full[Show full content (escape hatch for AXI truncation)]' \
'--daemon[Run in daemon mode (long-running background process)]' \
'--no-daemon[Disable daemon mode (force synchronous operation)]' \
'--list-jobs[List background jobs]' \
'--no-telemetry[Disable telemetry collection for this invocation]' \
'--telemetry-preview[Print the telemetry payload that would be sent without actually sending it]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
':package -- Package name or specifier to install:_default' \
&& ret=0
;;
(detect)
_arguments "${_arguments_options[@]}" : \
'--color=[Color output\: auto, always, never]:WHEN:_default' \
'--on-risk=[Action to take when a security scan detects risk]:ACTION:_default' \
'--fields=[Comma-separated list of fields to include in AXI minimal output]:LIST:_default' \
'--cancel-job=[Cancel a background job by ID]:ID:_default' \
'--manager=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--use=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--json[Emit output as JSON (machine-readable)]' \
'--human[Human-readable output (escape hatch for AXI/TOON mode)]' \
'*-v[Increase verbosity (can be repeated\: -v, -vv)]' \
'*--verbose[Increase verbosity (can be repeated\: -v, -vv)]' \
'-q[Suppress non-error output]' \
'--quiet[Suppress non-error output]' \
'--debug[Enable debug-level diagnostics]' \
'--dry-run[Show what would happen without making changes]' \
'--force[Force the operation, bypassing confirmations]' \
'--interactive[Enable interactive/TUI mode]' \
'--tui[Enable interactive/TUI mode]' \
'--no-pager[Disable pager output]' \
'--no-scan[Skip security scanning]' \
'--scan-only[Only run the security scan, then exit]' \
'--update-security-db[Update the security vulnerability database before scanning]' \
'--full[Show full content (escape hatch for AXI truncation)]' \
'--daemon[Run in daemon mode (long-running background process)]' \
'--no-daemon[Disable daemon mode (force synchronous operation)]' \
'--list-jobs[List background jobs]' \
'--no-telemetry[Disable telemetry collection for this invocation]' \
'--telemetry-preview[Print the telemetry payload that would be sent without actually sending it]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
'--color=[Color output\: auto, always, never]:WHEN:_default' \
'--on-risk=[Action to take when a security scan detects risk]:ACTION:_default' \
'--fields=[Comma-separated list of fields to include in AXI minimal output]:LIST:_default' \
'--cancel-job=[Cancel a background job by ID]:ID:_default' \
'--manager=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--use=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--json[Emit output as JSON (machine-readable)]' \
'--human[Human-readable output (escape hatch for AXI/TOON mode)]' \
'*-v[Increase verbosity (can be repeated\: -v, -vv)]' \
'*--verbose[Increase verbosity (can be repeated\: -v, -vv)]' \
'-q[Suppress non-error output]' \
'--quiet[Suppress non-error output]' \
'--debug[Enable debug-level diagnostics]' \
'--dry-run[Show what would happen without making changes]' \
'--force[Force the operation, bypassing confirmations]' \
'--interactive[Enable interactive/TUI mode]' \
'--tui[Enable interactive/TUI mode]' \
'--no-pager[Disable pager output]' \
'--no-scan[Skip security scanning]' \
'--scan-only[Only run the security scan, then exit]' \
'--update-security-db[Update the security vulnerability database before scanning]' \
'--full[Show full content (escape hatch for AXI truncation)]' \
'--daemon[Run in daemon mode (long-running background process)]' \
'--no-daemon[Disable daemon mode (force synchronous operation)]' \
'--list-jobs[List background jobs]' \
'--no-telemetry[Disable telemetry collection for this invocation]' \
'--telemetry-preview[Print the telemetry payload that would be sent without actually sending it]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
&& ret=0
;;
(clone)
_arguments "${_arguments_options[@]}" : \
'--color=[Color output\: auto, always, never]:WHEN:_default' \
'--on-risk=[Action to take when a security scan detects risk]:ACTION:_default' \
'--fields=[Comma-separated list of fields to include in AXI minimal output]:LIST:_default' \
'--cancel-job=[Cancel a background job by ID]:ID:_default' \
'--manager=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--use=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--json[Emit output as JSON (machine-readable)]' \
'--human[Human-readable output (escape hatch for AXI/TOON mode)]' \
'*-v[Increase verbosity (can be repeated\: -v, -vv)]' \
'*--verbose[Increase verbosity (can be repeated\: -v, -vv)]' \
'-q[Suppress non-error output]' \
'--quiet[Suppress non-error output]' \
'--debug[Enable debug-level diagnostics]' \
'--dry-run[Show what would happen without making changes]' \
'--force[Force the operation, bypassing confirmations]' \
'--interactive[Enable interactive/TUI mode]' \
'--tui[Enable interactive/TUI mode]' \
'--no-pager[Disable pager output]' \
'--no-scan[Skip security scanning]' \
'--scan-only[Only run the security scan, then exit]' \
'--update-security-db[Update the security vulnerability database before scanning]' \
'--full[Show full content (escape hatch for AXI truncation)]' \
'--daemon[Run in daemon mode (long-running background process)]' \
'--no-daemon[Disable daemon mode (force synchronous operation)]' \
'--list-jobs[List background jobs]' \
'--no-telemetry[Disable telemetry collection for this invocation]' \
'--telemetry-preview[Print the telemetry payload that would be sent without actually sending it]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
':package -- Repository URL or package name to clone:_default' \
&& ret=0
;;
(scan)
_arguments "${_arguments_options[@]}" : \
'--color=[Color output\: auto, always, never]:WHEN:_default' \
'--on-risk=[Action to take when a security scan detects risk]:ACTION:_default' \
'--fields=[Comma-separated list of fields to include in AXI minimal output]:LIST:_default' \
'--cancel-job=[Cancel a background job by ID]:ID:_default' \
'--manager=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--use=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--json[Emit output as JSON (machine-readable)]' \
'--human[Human-readable output (escape hatch for AXI/TOON mode)]' \
'*-v[Increase verbosity (can be repeated\: -v, -vv)]' \
'*--verbose[Increase verbosity (can be repeated\: -v, -vv)]' \
'-q[Suppress non-error output]' \
'--quiet[Suppress non-error output]' \
'--debug[Enable debug-level diagnostics]' \
'--dry-run[Show what would happen without making changes]' \
'--force[Force the operation, bypassing confirmations]' \
'--interactive[Enable interactive/TUI mode]' \
'--tui[Enable interactive/TUI mode]' \
'--no-pager[Disable pager output]' \
'--no-scan[Skip security scanning]' \
'--scan-only[Only run the security scan, then exit]' \
'--update-security-db[Update the security vulnerability database before scanning]' \
'--full[Show full content (escape hatch for AXI truncation)]' \
'--daemon[Run in daemon mode (long-running background process)]' \
'--no-daemon[Disable daemon mode (force synchronous operation)]' \
'--list-jobs[List background jobs]' \
'--no-telemetry[Disable telemetry collection for this invocation]' \
'--telemetry-preview[Print the telemetry payload that would be sent without actually sending it]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
'::package -- Package name to scan. If omitted, scans the current project:_default' \
&& ret=0
;;
(suggest)
_arguments "${_arguments_options[@]}" : \
'--color=[Color output\: auto, always, never]:WHEN:_default' \
'--on-risk=[Action to take when a security scan detects risk]:ACTION:_default' \
'--fields=[Comma-separated list of fields to include in AXI minimal output]:LIST:_default' \
'--cancel-job=[Cancel a background job by ID]:ID:_default' \
'--manager=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--use=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--json[Emit output as JSON (machine-readable)]' \
'--human[Human-readable output (escape hatch for AXI/TOON mode)]' \
'*-v[Increase verbosity (can be repeated\: -v, -vv)]' \
'*--verbose[Increase verbosity (can be repeated\: -v, -vv)]' \
'-q[Suppress non-error output]' \
'--quiet[Suppress non-error output]' \
'--debug[Enable debug-level diagnostics]' \
'--dry-run[Show what would happen without making changes]' \
'--force[Force the operation, bypassing confirmations]' \
'--interactive[Enable interactive/TUI mode]' \
'--tui[Enable interactive/TUI mode]' \
'--no-pager[Disable pager output]' \
'--no-scan[Skip security scanning]' \
'--scan-only[Only run the security scan, then exit]' \
'--update-security-db[Update the security vulnerability database before scanning]' \
'--full[Show full content (escape hatch for AXI truncation)]' \
'--daemon[Run in daemon mode (long-running background process)]' \
'--no-daemon[Disable daemon mode (force synchronous operation)]' \
'--list-jobs[List background jobs]' \
'--no-telemetry[Disable telemetry collection for this invocation]' \
'--telemetry-preview[Print the telemetry payload that would be sent without actually sending it]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
':package -- Package name to find alternatives for:_default' \
&& ret=0
;;
(info)
_arguments "${_arguments_options[@]}" : \
'--color=[Color output\: auto, always, never]:WHEN:_default' \
'--on-risk=[Action to take when a security scan detects risk]:ACTION:_default' \
'--fields=[Comma-separated list of fields to include in AXI minimal output]:LIST:_default' \
'--cancel-job=[Cancel a background job by ID]:ID:_default' \
'--manager=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--use=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--json[Emit output as JSON (machine-readable)]' \
'--human[Human-readable output (escape hatch for AXI/TOON mode)]' \
'*-v[Increase verbosity (can be repeated\: -v, -vv)]' \
'*--verbose[Increase verbosity (can be repeated\: -v, -vv)]' \
'-q[Suppress non-error output]' \
'--quiet[Suppress non-error output]' \
'--debug[Enable debug-level diagnostics]' \
'--dry-run[Show what would happen without making changes]' \
'--force[Force the operation, bypassing confirmations]' \
'--interactive[Enable interactive/TUI mode]' \
'--tui[Enable interactive/TUI mode]' \
'--no-pager[Disable pager output]' \
'--no-scan[Skip security scanning]' \
'--scan-only[Only run the security scan, then exit]' \
'--update-security-db[Update the security vulnerability database before scanning]' \
'--full[Show full content (escape hatch for AXI truncation)]' \
'--daemon[Run in daemon mode (long-running background process)]' \
'--no-daemon[Disable daemon mode (force synchronous operation)]' \
'--list-jobs[List background jobs]' \
'--no-telemetry[Disable telemetry collection for this invocation]' \
'--telemetry-preview[Print the telemetry payload that would be sent without actually sending it]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
':package -- Package name to inspect:_default' \
&& ret=0
;;
(audit-log)
_arguments "${_arguments_options[@]}" : \
'--color=[Color output\: auto, always, never]:WHEN:_default' \
'--on-risk=[Action to take when a security scan detects risk]:ACTION:_default' \
'--fields=[Comma-separated list of fields to include in AXI minimal output]:LIST:_default' \
'--cancel-job=[Cancel a background job by ID]:ID:_default' \
'--manager=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--use=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--json[Emit output as JSON (machine-readable)]' \
'--human[Human-readable output (escape hatch for AXI/TOON mode)]' \
'*-v[Increase verbosity (can be repeated\: -v, -vv)]' \
'*--verbose[Increase verbosity (can be repeated\: -v, -vv)]' \
'-q[Suppress non-error output]' \
'--quiet[Suppress non-error output]' \
'--debug[Enable debug-level diagnostics]' \
'--dry-run[Show what would happen without making changes]' \
'--force[Force the operation, bypassing confirmations]' \
'--interactive[Enable interactive/TUI mode]' \
'--tui[Enable interactive/TUI mode]' \
'--no-pager[Disable pager output]' \
'--no-scan[Skip security scanning]' \
'--scan-only[Only run the security scan, then exit]' \
'--update-security-db[Update the security vulnerability database before scanning]' \
'--full[Show full content (escape hatch for AXI truncation)]' \
'--daemon[Run in daemon mode (long-running background process)]' \
'--no-daemon[Disable daemon mode (force synchronous operation)]' \
'--list-jobs[List background jobs]' \
'--no-telemetry[Disable telemetry collection for this invocation]' \
'--telemetry-preview[Print the telemetry payload that would be sent without actually sending it]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
&& ret=0
;;
(config)
_arguments "${_arguments_options[@]}" : \
'--color=[Color output\: auto, always, never]:WHEN:_default' \
'--on-risk=[Action to take when a security scan detects risk]:ACTION:_default' \
'--fields=[Comma-separated list of fields to include in AXI minimal output]:LIST:_default' \
'--cancel-job=[Cancel a background job by ID]:ID:_default' \
'--manager=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--use=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--init[Initialize the config file with defaults]' \
'--show[Show the resolved configuration]' \
'--json[Emit output as JSON (machine-readable)]' \
'--human[Human-readable output (escape hatch for AXI/TOON mode)]' \
'*-v[Increase verbosity (can be repeated\: -v, -vv)]' \
'*--verbose[Increase verbosity (can be repeated\: -v, -vv)]' \
'-q[Suppress non-error output]' \
'--quiet[Suppress non-error output]' \
'--debug[Enable debug-level diagnostics]' \
'--dry-run[Show what would happen without making changes]' \
'--force[Force the operation, bypassing confirmations]' \
'--interactive[Enable interactive/TUI mode]' \
'--tui[Enable interactive/TUI mode]' \
'--no-pager[Disable pager output]' \
'--no-scan[Skip security scanning]' \
'--scan-only[Only run the security scan, then exit]' \
'--update-security-db[Update the security vulnerability database before scanning]' \
'--full[Show full content (escape hatch for AXI truncation)]' \
'--daemon[Run in daemon mode (long-running background process)]' \
'--no-daemon[Disable daemon mode (force synchronous operation)]' \
'--list-jobs[List background jobs]' \
'--no-telemetry[Disable telemetry collection for this invocation]' \
'--telemetry-preview[Print the telemetry payload that would be sent without actually sending it]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
&& ret=0
;;
(governance)
_arguments "${_arguments_options[@]}" : \
'--color=[Color output\: auto, always, never]:WHEN:_default' \
'--on-risk=[Action to take when a security scan detects risk]:ACTION:_default' \
'--fields=[Comma-separated list of fields to include in AXI minimal output]:LIST:_default' \
'--cancel-job=[Cancel a background job by ID]:ID:_default' \
'--manager=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--use=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--json[Emit output as JSON (machine-readable)]' \
'--human[Human-readable output (escape hatch for AXI/TOON mode)]' \
'*-v[Increase verbosity (can be repeated\: -v, -vv)]' \
'*--verbose[Increase verbosity (can be repeated\: -v, -vv)]' \
'-q[Suppress non-error output]' \
'--quiet[Suppress non-error output]' \
'--debug[Enable debug-level diagnostics]' \
'--dry-run[Show what would happen without making changes]' \
'--force[Force the operation, bypassing confirmations]' \
'--interactive[Enable interactive/TUI mode]' \
'--tui[Enable interactive/TUI mode]' \
'--no-pager[Disable pager output]' \
'--no-scan[Skip security scanning]' \
'--scan-only[Only run the security scan, then exit]' \
'--update-security-db[Update the security vulnerability database before scanning]' \
'--full[Show full content (escape hatch for AXI truncation)]' \
'--daemon[Run in daemon mode (long-running background process)]' \
'--no-daemon[Disable daemon mode (force synchronous operation)]' \
'--list-jobs[List background jobs]' \
'--no-telemetry[Disable telemetry collection for this invocation]' \
'--telemetry-preview[Print the telemetry payload that would be sent without actually sending it]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
":: :_apmw__subcmd__governance_commands" \
"*::: :->governance" \
&& ret=0

    case $state in
    (governance)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:apmw-governance-command-$line[1]:"
        case $line[1] in
            (refresh)
_arguments "${_arguments_options[@]}" : \
'--color=[Color output\: auto, always, never]:WHEN:_default' \
'--on-risk=[Action to take when a security scan detects risk]:ACTION:_default' \
'--fields=[Comma-separated list of fields to include in AXI minimal output]:LIST:_default' \
'--cancel-job=[Cancel a background job by ID]:ID:_default' \
'--manager=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--use=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--json[Emit output as JSON (machine-readable)]' \
'--human[Human-readable output (escape hatch for AXI/TOON mode)]' \
'*-v[Increase verbosity (can be repeated\: -v, -vv)]' \
'*--verbose[Increase verbosity (can be repeated\: -v, -vv)]' \
'-q[Suppress non-error output]' \
'--quiet[Suppress non-error output]' \
'--debug[Enable debug-level diagnostics]' \
'--dry-run[Show what would happen without making changes]' \
'--force[Force the operation, bypassing confirmations]' \
'--interactive[Enable interactive/TUI mode]' \
'--tui[Enable interactive/TUI mode]' \
'--no-pager[Disable pager output]' \
'--no-scan[Skip security scanning]' \
'--scan-only[Only run the security scan, then exit]' \
'--update-security-db[Update the security vulnerability database before scanning]' \
'--full[Show full content (escape hatch for AXI truncation)]' \
'--daemon[Run in daemon mode (long-running background process)]' \
'--no-daemon[Disable daemon mode (force synchronous operation)]' \
'--list-jobs[List background jobs]' \
'--no-telemetry[Disable telemetry collection for this invocation]' \
'--telemetry-preview[Print the telemetry payload that would be sent without actually sending it]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_apmw__subcmd__governance__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:apmw-governance-help-command-$line[1]:"
        case $line[1] in
            (refresh)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(intercept)
_arguments "${_arguments_options[@]}" : \
'--color=[Color output\: auto, always, never]:WHEN:_default' \
'--on-risk=[Action to take when a security scan detects risk]:ACTION:_default' \
'--fields=[Comma-separated list of fields to include in AXI minimal output]:LIST:_default' \
'--cancel-job=[Cancel a background job by ID]:ID:_default' \
'--manager=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--use=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--json[Emit output as JSON (machine-readable)]' \
'--human[Human-readable output (escape hatch for AXI/TOON mode)]' \
'*-v[Increase verbosity (can be repeated\: -v, -vv)]' \
'*--verbose[Increase verbosity (can be repeated\: -v, -vv)]' \
'-q[Suppress non-error output]' \
'--quiet[Suppress non-error output]' \
'--debug[Enable debug-level diagnostics]' \
'--dry-run[Show what would happen without making changes]' \
'--force[Force the operation, bypassing confirmations]' \
'--interactive[Enable interactive/TUI mode]' \
'--tui[Enable interactive/TUI mode]' \
'--no-pager[Disable pager output]' \
'--no-scan[Skip security scanning]' \
'--scan-only[Only run the security scan, then exit]' \
'--update-security-db[Update the security vulnerability database before scanning]' \
'--full[Show full content (escape hatch for AXI truncation)]' \
'--daemon[Run in daemon mode (long-running background process)]' \
'--no-daemon[Disable daemon mode (force synchronous operation)]' \
'--list-jobs[List background jobs]' \
'--no-telemetry[Disable telemetry collection for this invocation]' \
'--telemetry-preview[Print the telemetry payload that would be sent without actually sending it]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
':tool -- The package manager tool that was intercepted (e.g. `pip`, `npm`):_default' \
'*::args -- Arguments to pass through to the (canonical) package manager:_default' \
&& ret=0
;;
(mcp)
_arguments "${_arguments_options[@]}" : \
'--color=[Color output\: auto, always, never]:WHEN:_default' \
'--on-risk=[Action to take when a security scan detects risk]:ACTION:_default' \
'--fields=[Comma-separated list of fields to include in AXI minimal output]:LIST:_default' \
'--cancel-job=[Cancel a background job by ID]:ID:_default' \
'--manager=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--use=[Override auto-detection and force a specific package manager]:NAME:_default' \
'--json[Emit output as JSON (machine-readable)]' \
'--human[Human-readable output (escape hatch for AXI/TOON mode)]' \
'*-v[Increase verbosity (can be repeated\: -v, -vv)]' \
'*--verbose[Increase verbosity (can be repeated\: -v, -vv)]' \
'-q[Suppress non-error output]' \
'--quiet[Suppress non-error output]' \
'--debug[Enable debug-level diagnostics]' \
'--dry-run[Show what would happen without making changes]' \
'--force[Force the operation, bypassing confirmations]' \
'--interactive[Enable interactive/TUI mode]' \
'--tui[Enable interactive/TUI mode]' \
'--no-pager[Disable pager output]' \
'--no-scan[Skip security scanning]' \
'--scan-only[Only run the security scan, then exit]' \
'--update-security-db[Update the security vulnerability database before scanning]' \
'--full[Show full content (escape hatch for AXI truncation)]' \
'--daemon[Run in daemon mode (long-running background process)]' \
'--no-daemon[Disable daemon mode (force synchronous operation)]' \
'--list-jobs[List background jobs]' \
'--no-telemetry[Disable telemetry collection for this invocation]' \
'--telemetry-preview[Print the telemetry payload that would be sent without actually sending it]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_apmw__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:apmw-help-command-$line[1]:"
        case $line[1] in
            (install)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(detect)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(clone)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(scan)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(suggest)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(info)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(audit-log)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(config)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(governance)
_arguments "${_arguments_options[@]}" : \
":: :_apmw__subcmd__help__subcmd__governance_commands" \
"*::: :->governance" \
&& ret=0

    case $state in
    (governance)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:apmw-help-governance-command-$line[1]:"
        case $line[1] in
            (refresh)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(intercept)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(mcp)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
}

(( $+functions[_apmw_commands] )) ||
_apmw_commands() {
    local commands; commands=(
'install:Install a package or tool' \
'detect:Detect the package manager for the current project' \
'status:Show apmw status and audit log' \
'clone:Create a historyless clone of a repository with AST indexing' \
'scan:Scan a package (or the current project) for security issues' \
'suggest:Suggest within-ecosystem alternatives for a package' \
'info:Show detailed info about a package' \
'audit-log:Show the audit log of past operations' \
'config:View or initialize apmw configuration' \
'governance:Manage governance rules (refresh the spec, show current rules)' \
'intercept:Intercept a package manager call (invoked by PATH shims)' \
'mcp:Start the MCP (Model Context Protocol) server over stdio' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'apmw commands' commands "$@"
}
(( $+functions[_apmw__subcmd__audit-log_commands] )) ||
_apmw__subcmd__audit-log_commands() {
    local commands; commands=()
    _describe -t commands 'apmw audit-log commands' commands "$@"
}
(( $+functions[_apmw__subcmd__clone_commands] )) ||
_apmw__subcmd__clone_commands() {
    local commands; commands=()
    _describe -t commands 'apmw clone commands' commands "$@"
}
(( $+functions[_apmw__subcmd__config_commands] )) ||
_apmw__subcmd__config_commands() {
    local commands; commands=()
    _describe -t commands 'apmw config commands' commands "$@"
}
(( $+functions[_apmw__subcmd__detect_commands] )) ||
_apmw__subcmd__detect_commands() {
    local commands; commands=()
    _describe -t commands 'apmw detect commands' commands "$@"
}
(( $+functions[_apmw__subcmd__governance_commands] )) ||
_apmw__subcmd__governance_commands() {
    local commands; commands=(
'refresh:Force-refresh the cached governance spec from levonk-packages' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'apmw governance commands' commands "$@"
}
(( $+functions[_apmw__subcmd__governance__subcmd__help_commands] )) ||
_apmw__subcmd__governance__subcmd__help_commands() {
    local commands; commands=(
'refresh:Force-refresh the cached governance spec from levonk-packages' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'apmw governance help commands' commands "$@"
}
(( $+functions[_apmw__subcmd__governance__subcmd__help__subcmd__help_commands] )) ||
_apmw__subcmd__governance__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'apmw governance help help commands' commands "$@"
}
(( $+functions[_apmw__subcmd__governance__subcmd__help__subcmd__refresh_commands] )) ||
_apmw__subcmd__governance__subcmd__help__subcmd__refresh_commands() {
    local commands; commands=()
    _describe -t commands 'apmw governance help refresh commands' commands "$@"
}
(( $+functions[_apmw__subcmd__governance__subcmd__refresh_commands] )) ||
_apmw__subcmd__governance__subcmd__refresh_commands() {
    local commands; commands=()
    _describe -t commands 'apmw governance refresh commands' commands "$@"
}
(( $+functions[_apmw__subcmd__help_commands] )) ||
_apmw__subcmd__help_commands() {
    local commands; commands=(
'install:Install a package or tool' \
'detect:Detect the package manager for the current project' \
'status:Show apmw status and audit log' \
'clone:Create a historyless clone of a repository with AST indexing' \
'scan:Scan a package (or the current project) for security issues' \
'suggest:Suggest within-ecosystem alternatives for a package' \
'info:Show detailed info about a package' \
'audit-log:Show the audit log of past operations' \
'config:View or initialize apmw configuration' \
'governance:Manage governance rules (refresh the spec, show current rules)' \
'intercept:Intercept a package manager call (invoked by PATH shims)' \
'mcp:Start the MCP (Model Context Protocol) server over stdio' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'apmw help commands' commands "$@"
}
(( $+functions[_apmw__subcmd__help__subcmd__audit-log_commands] )) ||
_apmw__subcmd__help__subcmd__audit-log_commands() {
    local commands; commands=()
    _describe -t commands 'apmw help audit-log commands' commands "$@"
}
(( $+functions[_apmw__subcmd__help__subcmd__clone_commands] )) ||
_apmw__subcmd__help__subcmd__clone_commands() {
    local commands; commands=()
    _describe -t commands 'apmw help clone commands' commands "$@"
}
(( $+functions[_apmw__subcmd__help__subcmd__config_commands] )) ||
_apmw__subcmd__help__subcmd__config_commands() {
    local commands; commands=()
    _describe -t commands 'apmw help config commands' commands "$@"
}
(( $+functions[_apmw__subcmd__help__subcmd__detect_commands] )) ||
_apmw__subcmd__help__subcmd__detect_commands() {
    local commands; commands=()
    _describe -t commands 'apmw help detect commands' commands "$@"
}
(( $+functions[_apmw__subcmd__help__subcmd__governance_commands] )) ||
_apmw__subcmd__help__subcmd__governance_commands() {
    local commands; commands=(
'refresh:Force-refresh the cached governance spec from levonk-packages' \
    )
    _describe -t commands 'apmw help governance commands' commands "$@"
}
(( $+functions[_apmw__subcmd__help__subcmd__governance__subcmd__refresh_commands] )) ||
_apmw__subcmd__help__subcmd__governance__subcmd__refresh_commands() {
    local commands; commands=()
    _describe -t commands 'apmw help governance refresh commands' commands "$@"
}
(( $+functions[_apmw__subcmd__help__subcmd__help_commands] )) ||
_apmw__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'apmw help help commands' commands "$@"
}
(( $+functions[_apmw__subcmd__help__subcmd__info_commands] )) ||
_apmw__subcmd__help__subcmd__info_commands() {
    local commands; commands=()
    _describe -t commands 'apmw help info commands' commands "$@"
}
(( $+functions[_apmw__subcmd__help__subcmd__install_commands] )) ||
_apmw__subcmd__help__subcmd__install_commands() {
    local commands; commands=()
    _describe -t commands 'apmw help install commands' commands "$@"
}
(( $+functions[_apmw__subcmd__help__subcmd__intercept_commands] )) ||
_apmw__subcmd__help__subcmd__intercept_commands() {
    local commands; commands=()
    _describe -t commands 'apmw help intercept commands' commands "$@"
}
(( $+functions[_apmw__subcmd__help__subcmd__mcp_commands] )) ||
_apmw__subcmd__help__subcmd__mcp_commands() {
    local commands; commands=()
    _describe -t commands 'apmw help mcp commands' commands "$@"
}
(( $+functions[_apmw__subcmd__help__subcmd__scan_commands] )) ||
_apmw__subcmd__help__subcmd__scan_commands() {
    local commands; commands=()
    _describe -t commands 'apmw help scan commands' commands "$@"
}
(( $+functions[_apmw__subcmd__help__subcmd__status_commands] )) ||
_apmw__subcmd__help__subcmd__status_commands() {
    local commands; commands=()
    _describe -t commands 'apmw help status commands' commands "$@"
}
(( $+functions[_apmw__subcmd__help__subcmd__suggest_commands] )) ||
_apmw__subcmd__help__subcmd__suggest_commands() {
    local commands; commands=()
    _describe -t commands 'apmw help suggest commands' commands "$@"
}
(( $+functions[_apmw__subcmd__info_commands] )) ||
_apmw__subcmd__info_commands() {
    local commands; commands=()
    _describe -t commands 'apmw info commands' commands "$@"
}
(( $+functions[_apmw__subcmd__install_commands] )) ||
_apmw__subcmd__install_commands() {
    local commands; commands=()
    _describe -t commands 'apmw install commands' commands "$@"
}
(( $+functions[_apmw__subcmd__intercept_commands] )) ||
_apmw__subcmd__intercept_commands() {
    local commands; commands=()
    _describe -t commands 'apmw intercept commands' commands "$@"
}
(( $+functions[_apmw__subcmd__mcp_commands] )) ||
_apmw__subcmd__mcp_commands() {
    local commands; commands=()
    _describe -t commands 'apmw mcp commands' commands "$@"
}
(( $+functions[_apmw__subcmd__scan_commands] )) ||
_apmw__subcmd__scan_commands() {
    local commands; commands=()
    _describe -t commands 'apmw scan commands' commands "$@"
}
(( $+functions[_apmw__subcmd__status_commands] )) ||
_apmw__subcmd__status_commands() {
    local commands; commands=()
    _describe -t commands 'apmw status commands' commands "$@"
}
(( $+functions[_apmw__subcmd__suggest_commands] )) ||
_apmw__subcmd__suggest_commands() {
    local commands; commands=()
    _describe -t commands 'apmw suggest commands' commands "$@"
}

if [ "$funcstack[1]" = "_apmw" ]; then
    _apmw "$@"
else
    compdef _apmw apmw
fi
