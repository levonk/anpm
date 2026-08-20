# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_apmw_global_optspecs
    string join \n install uninstall intercept shell= man usage json color= human v/verbose q/quiet debug dry-run force interactive no-pager no-scan scan-only on-risk= update-security-db fields= full daemon no-daemon list-jobs cancel-job= manager= no-telemetry telemetry-preview h/help V/version
end

function __fish_apmw_needs_command
    # Figure out if the current invocation already has a command.
    set -l cmd (commandline -opc)
    set -e cmd[1]
    argparse -s (__fish_apmw_global_optspecs) -- $cmd 2>/dev/null
    or return
    if set -q argv[1]
        # Also print the command, so this can be used to figure out what it is.
        echo $argv[1]
        return 1
    end
    return 0
end

function __fish_apmw_using_subcommand
    set -l cmd (__fish_apmw_needs_command)
    test -z "$cmd"
    and return 1
    contains -- $cmd[1] $argv
end

complete -c apmw -n "__fish_apmw_needs_command" -l shell -d 'Shell to generate completions for (used with --install)' -r
complete -c apmw -n "__fish_apmw_needs_command" -l color -d 'Color output: auto, always, never' -r
complete -c apmw -n "__fish_apmw_needs_command" -l on-risk -d 'Action to take when a security scan detects risk' -r
complete -c apmw -n "__fish_apmw_needs_command" -l fields -d 'Comma-separated list of fields to include in AXI minimal output' -r
complete -c apmw -n "__fish_apmw_needs_command" -l cancel-job -d 'Cancel a background job by ID' -r
complete -c apmw -n "__fish_apmw_needs_command" -l manager -l use -d 'Override auto-detection and force a specific package manager' -r
complete -c apmw -n "__fish_apmw_needs_command" -l install -d 'Generate shell completions and initialize the config file'
complete -c apmw -n "__fish_apmw_needs_command" -l uninstall -d 'Remove generated completions and config (best-effort cleanup)'
complete -c apmw -n "__fish_apmw_needs_command" -l intercept -d 'Install or remove PATH shims that intercept package manager calls (used with `--install` or `--uninstall`)'
complete -c apmw -n "__fish_apmw_needs_command" -l man -d 'Print the man page to stdout (groff/troff format) and exit'
complete -c apmw -n "__fish_apmw_needs_command" -l usage -d 'Print a brief usage summary and exit'
complete -c apmw -n "__fish_apmw_needs_command" -l json -d 'Emit output as JSON (machine-readable)'
complete -c apmw -n "__fish_apmw_needs_command" -l human -d 'Human-readable output (escape hatch for AXI/TOON mode)'
complete -c apmw -n "__fish_apmw_needs_command" -s v -l verbose -d 'Increase verbosity (can be repeated: -v, -vv)'
complete -c apmw -n "__fish_apmw_needs_command" -s q -l quiet -d 'Suppress non-error output'
complete -c apmw -n "__fish_apmw_needs_command" -l debug -d 'Enable debug-level diagnostics'
complete -c apmw -n "__fish_apmw_needs_command" -l dry-run -d 'Show what would happen without making changes'
complete -c apmw -n "__fish_apmw_needs_command" -l force -d 'Force the operation, bypassing confirmations'
complete -c apmw -n "__fish_apmw_needs_command" -l interactive -l tui -d 'Enable interactive/TUI mode'
complete -c apmw -n "__fish_apmw_needs_command" -l no-pager -d 'Disable pager output'
complete -c apmw -n "__fish_apmw_needs_command" -l no-scan -d 'Skip security scanning'
complete -c apmw -n "__fish_apmw_needs_command" -l scan-only -d 'Only run the security scan, then exit'
complete -c apmw -n "__fish_apmw_needs_command" -l update-security-db -d 'Update the security vulnerability database before scanning'
complete -c apmw -n "__fish_apmw_needs_command" -l full -d 'Show full content (escape hatch for AXI truncation)'
complete -c apmw -n "__fish_apmw_needs_command" -l daemon -d 'Run in daemon mode (long-running background process)'
complete -c apmw -n "__fish_apmw_needs_command" -l no-daemon -d 'Disable daemon mode (force synchronous operation)'
complete -c apmw -n "__fish_apmw_needs_command" -l list-jobs -d 'List background jobs'
complete -c apmw -n "__fish_apmw_needs_command" -l no-telemetry -d 'Disable telemetry collection for this invocation'
complete -c apmw -n "__fish_apmw_needs_command" -l telemetry-preview -d 'Print the telemetry payload that would be sent without actually sending it'
complete -c apmw -n "__fish_apmw_needs_command" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c apmw -n "__fish_apmw_needs_command" -s V -l version -d 'Print version'
complete -c apmw -n "__fish_apmw_needs_command" -f -a "install" -d 'Install a package or tool'
complete -c apmw -n "__fish_apmw_needs_command" -f -a "detect" -d 'Detect the package manager for the current project'
complete -c apmw -n "__fish_apmw_needs_command" -f -a "status" -d 'Show apmw status and audit log'
complete -c apmw -n "__fish_apmw_needs_command" -f -a "clone" -d 'Create a historyless clone of a repository with AST indexing'
complete -c apmw -n "__fish_apmw_needs_command" -f -a "scan" -d 'Scan a package (or the current project) for security issues'
complete -c apmw -n "__fish_apmw_needs_command" -f -a "suggest" -d 'Suggest within-ecosystem alternatives for a package'
complete -c apmw -n "__fish_apmw_needs_command" -f -a "info" -d 'Show detailed info about a package'
complete -c apmw -n "__fish_apmw_needs_command" -f -a "audit-log" -d 'Show the audit log of past operations'
complete -c apmw -n "__fish_apmw_needs_command" -f -a "config" -d 'View or initialize apmw configuration'
complete -c apmw -n "__fish_apmw_needs_command" -f -a "governance" -d 'Manage governance rules (refresh the spec, show current rules)'
complete -c apmw -n "__fish_apmw_needs_command" -f -a "intercept" -d 'Intercept a package manager call (invoked by PATH shims)'
complete -c apmw -n "__fish_apmw_needs_command" -f -a "mcp" -d 'Start the MCP (Model Context Protocol) server over stdio'
complete -c apmw -n "__fish_apmw_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c apmw -n "__fish_apmw_using_subcommand install" -l color -d 'Color output: auto, always, never' -r
complete -c apmw -n "__fish_apmw_using_subcommand install" -l on-risk -d 'Action to take when a security scan detects risk' -r
complete -c apmw -n "__fish_apmw_using_subcommand install" -l fields -d 'Comma-separated list of fields to include in AXI minimal output' -r
complete -c apmw -n "__fish_apmw_using_subcommand install" -l cancel-job -d 'Cancel a background job by ID' -r
complete -c apmw -n "__fish_apmw_using_subcommand install" -l manager -l use -d 'Override auto-detection and force a specific package manager' -r
complete -c apmw -n "__fish_apmw_using_subcommand install" -l dev -d 'Install as a development/build-time dependency'
complete -c apmw -n "__fish_apmw_using_subcommand install" -l json -d 'Emit output as JSON (machine-readable)'
complete -c apmw -n "__fish_apmw_using_subcommand install" -l human -d 'Human-readable output (escape hatch for AXI/TOON mode)'
complete -c apmw -n "__fish_apmw_using_subcommand install" -s v -l verbose -d 'Increase verbosity (can be repeated: -v, -vv)'
complete -c apmw -n "__fish_apmw_using_subcommand install" -s q -l quiet -d 'Suppress non-error output'
complete -c apmw -n "__fish_apmw_using_subcommand install" -l debug -d 'Enable debug-level diagnostics'
complete -c apmw -n "__fish_apmw_using_subcommand install" -l dry-run -d 'Show what would happen without making changes'
complete -c apmw -n "__fish_apmw_using_subcommand install" -l force -d 'Force the operation, bypassing confirmations'
complete -c apmw -n "__fish_apmw_using_subcommand install" -l interactive -l tui -d 'Enable interactive/TUI mode'
complete -c apmw -n "__fish_apmw_using_subcommand install" -l no-pager -d 'Disable pager output'
complete -c apmw -n "__fish_apmw_using_subcommand install" -l no-scan -d 'Skip security scanning'
complete -c apmw -n "__fish_apmw_using_subcommand install" -l scan-only -d 'Only run the security scan, then exit'
complete -c apmw -n "__fish_apmw_using_subcommand install" -l update-security-db -d 'Update the security vulnerability database before scanning'
complete -c apmw -n "__fish_apmw_using_subcommand install" -l full -d 'Show full content (escape hatch for AXI truncation)'
complete -c apmw -n "__fish_apmw_using_subcommand install" -l daemon -d 'Run in daemon mode (long-running background process)'
complete -c apmw -n "__fish_apmw_using_subcommand install" -l no-daemon -d 'Disable daemon mode (force synchronous operation)'
complete -c apmw -n "__fish_apmw_using_subcommand install" -l list-jobs -d 'List background jobs'
complete -c apmw -n "__fish_apmw_using_subcommand install" -l no-telemetry -d 'Disable telemetry collection for this invocation'
complete -c apmw -n "__fish_apmw_using_subcommand install" -l telemetry-preview -d 'Print the telemetry payload that would be sent without actually sending it'
complete -c apmw -n "__fish_apmw_using_subcommand install" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l color -d 'Color output: auto, always, never' -r
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l on-risk -d 'Action to take when a security scan detects risk' -r
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l fields -d 'Comma-separated list of fields to include in AXI minimal output' -r
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l cancel-job -d 'Cancel a background job by ID' -r
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l manager -l use -d 'Override auto-detection and force a specific package manager' -r
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l json -d 'Emit output as JSON (machine-readable)'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l human -d 'Human-readable output (escape hatch for AXI/TOON mode)'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -s v -l verbose -d 'Increase verbosity (can be repeated: -v, -vv)'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -s q -l quiet -d 'Suppress non-error output'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l debug -d 'Enable debug-level diagnostics'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l dry-run -d 'Show what would happen without making changes'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l force -d 'Force the operation, bypassing confirmations'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l interactive -l tui -d 'Enable interactive/TUI mode'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l no-pager -d 'Disable pager output'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l no-scan -d 'Skip security scanning'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l scan-only -d 'Only run the security scan, then exit'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l update-security-db -d 'Update the security vulnerability database before scanning'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l full -d 'Show full content (escape hatch for AXI truncation)'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l daemon -d 'Run in daemon mode (long-running background process)'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l no-daemon -d 'Disable daemon mode (force synchronous operation)'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l list-jobs -d 'List background jobs'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l no-telemetry -d 'Disable telemetry collection for this invocation'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -l telemetry-preview -d 'Print the telemetry payload that would be sent without actually sending it'
complete -c apmw -n "__fish_apmw_using_subcommand detect" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c apmw -n "__fish_apmw_using_subcommand status" -l color -d 'Color output: auto, always, never' -r
complete -c apmw -n "__fish_apmw_using_subcommand status" -l on-risk -d 'Action to take when a security scan detects risk' -r
complete -c apmw -n "__fish_apmw_using_subcommand status" -l fields -d 'Comma-separated list of fields to include in AXI minimal output' -r
complete -c apmw -n "__fish_apmw_using_subcommand status" -l cancel-job -d 'Cancel a background job by ID' -r
complete -c apmw -n "__fish_apmw_using_subcommand status" -l manager -l use -d 'Override auto-detection and force a specific package manager' -r
complete -c apmw -n "__fish_apmw_using_subcommand status" -l json -d 'Emit output as JSON (machine-readable)'
complete -c apmw -n "__fish_apmw_using_subcommand status" -l human -d 'Human-readable output (escape hatch for AXI/TOON mode)'
complete -c apmw -n "__fish_apmw_using_subcommand status" -s v -l verbose -d 'Increase verbosity (can be repeated: -v, -vv)'
complete -c apmw -n "__fish_apmw_using_subcommand status" -s q -l quiet -d 'Suppress non-error output'
complete -c apmw -n "__fish_apmw_using_subcommand status" -l debug -d 'Enable debug-level diagnostics'
complete -c apmw -n "__fish_apmw_using_subcommand status" -l dry-run -d 'Show what would happen without making changes'
complete -c apmw -n "__fish_apmw_using_subcommand status" -l force -d 'Force the operation, bypassing confirmations'
complete -c apmw -n "__fish_apmw_using_subcommand status" -l interactive -l tui -d 'Enable interactive/TUI mode'
complete -c apmw -n "__fish_apmw_using_subcommand status" -l no-pager -d 'Disable pager output'
complete -c apmw -n "__fish_apmw_using_subcommand status" -l no-scan -d 'Skip security scanning'
complete -c apmw -n "__fish_apmw_using_subcommand status" -l scan-only -d 'Only run the security scan, then exit'
complete -c apmw -n "__fish_apmw_using_subcommand status" -l update-security-db -d 'Update the security vulnerability database before scanning'
complete -c apmw -n "__fish_apmw_using_subcommand status" -l full -d 'Show full content (escape hatch for AXI truncation)'
complete -c apmw -n "__fish_apmw_using_subcommand status" -l daemon -d 'Run in daemon mode (long-running background process)'
complete -c apmw -n "__fish_apmw_using_subcommand status" -l no-daemon -d 'Disable daemon mode (force synchronous operation)'
complete -c apmw -n "__fish_apmw_using_subcommand status" -l list-jobs -d 'List background jobs'
complete -c apmw -n "__fish_apmw_using_subcommand status" -l no-telemetry -d 'Disable telemetry collection for this invocation'
complete -c apmw -n "__fish_apmw_using_subcommand status" -l telemetry-preview -d 'Print the telemetry payload that would be sent without actually sending it'
complete -c apmw -n "__fish_apmw_using_subcommand status" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l color -d 'Color output: auto, always, never' -r
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l on-risk -d 'Action to take when a security scan detects risk' -r
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l fields -d 'Comma-separated list of fields to include in AXI minimal output' -r
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l cancel-job -d 'Cancel a background job by ID' -r
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l manager -l use -d 'Override auto-detection and force a specific package manager' -r
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l json -d 'Emit output as JSON (machine-readable)'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l human -d 'Human-readable output (escape hatch for AXI/TOON mode)'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -s v -l verbose -d 'Increase verbosity (can be repeated: -v, -vv)'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -s q -l quiet -d 'Suppress non-error output'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l debug -d 'Enable debug-level diagnostics'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l dry-run -d 'Show what would happen without making changes'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l force -d 'Force the operation, bypassing confirmations'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l interactive -l tui -d 'Enable interactive/TUI mode'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l no-pager -d 'Disable pager output'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l no-scan -d 'Skip security scanning'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l scan-only -d 'Only run the security scan, then exit'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l update-security-db -d 'Update the security vulnerability database before scanning'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l full -d 'Show full content (escape hatch for AXI truncation)'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l daemon -d 'Run in daemon mode (long-running background process)'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l no-daemon -d 'Disable daemon mode (force synchronous operation)'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l list-jobs -d 'List background jobs'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l no-telemetry -d 'Disable telemetry collection for this invocation'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -l telemetry-preview -d 'Print the telemetry payload that would be sent without actually sending it'
complete -c apmw -n "__fish_apmw_using_subcommand clone" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l color -d 'Color output: auto, always, never' -r
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l on-risk -d 'Action to take when a security scan detects risk' -r
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l fields -d 'Comma-separated list of fields to include in AXI minimal output' -r
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l cancel-job -d 'Cancel a background job by ID' -r
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l manager -l use -d 'Override auto-detection and force a specific package manager' -r
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l json -d 'Emit output as JSON (machine-readable)'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l human -d 'Human-readable output (escape hatch for AXI/TOON mode)'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -s v -l verbose -d 'Increase verbosity (can be repeated: -v, -vv)'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -s q -l quiet -d 'Suppress non-error output'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l debug -d 'Enable debug-level diagnostics'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l dry-run -d 'Show what would happen without making changes'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l force -d 'Force the operation, bypassing confirmations'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l interactive -l tui -d 'Enable interactive/TUI mode'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l no-pager -d 'Disable pager output'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l no-scan -d 'Skip security scanning'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l scan-only -d 'Only run the security scan, then exit'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l update-security-db -d 'Update the security vulnerability database before scanning'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l full -d 'Show full content (escape hatch for AXI truncation)'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l daemon -d 'Run in daemon mode (long-running background process)'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l no-daemon -d 'Disable daemon mode (force synchronous operation)'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l list-jobs -d 'List background jobs'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l no-telemetry -d 'Disable telemetry collection for this invocation'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -l telemetry-preview -d 'Print the telemetry payload that would be sent without actually sending it'
complete -c apmw -n "__fish_apmw_using_subcommand scan" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l color -d 'Color output: auto, always, never' -r
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l on-risk -d 'Action to take when a security scan detects risk' -r
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l fields -d 'Comma-separated list of fields to include in AXI minimal output' -r
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l cancel-job -d 'Cancel a background job by ID' -r
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l manager -l use -d 'Override auto-detection and force a specific package manager' -r
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l json -d 'Emit output as JSON (machine-readable)'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l human -d 'Human-readable output (escape hatch for AXI/TOON mode)'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -s v -l verbose -d 'Increase verbosity (can be repeated: -v, -vv)'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -s q -l quiet -d 'Suppress non-error output'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l debug -d 'Enable debug-level diagnostics'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l dry-run -d 'Show what would happen without making changes'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l force -d 'Force the operation, bypassing confirmations'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l interactive -l tui -d 'Enable interactive/TUI mode'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l no-pager -d 'Disable pager output'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l no-scan -d 'Skip security scanning'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l scan-only -d 'Only run the security scan, then exit'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l update-security-db -d 'Update the security vulnerability database before scanning'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l full -d 'Show full content (escape hatch for AXI truncation)'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l daemon -d 'Run in daemon mode (long-running background process)'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l no-daemon -d 'Disable daemon mode (force synchronous operation)'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l list-jobs -d 'List background jobs'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l no-telemetry -d 'Disable telemetry collection for this invocation'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -l telemetry-preview -d 'Print the telemetry payload that would be sent without actually sending it'
complete -c apmw -n "__fish_apmw_using_subcommand suggest" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c apmw -n "__fish_apmw_using_subcommand info" -l color -d 'Color output: auto, always, never' -r
complete -c apmw -n "__fish_apmw_using_subcommand info" -l on-risk -d 'Action to take when a security scan detects risk' -r
complete -c apmw -n "__fish_apmw_using_subcommand info" -l fields -d 'Comma-separated list of fields to include in AXI minimal output' -r
complete -c apmw -n "__fish_apmw_using_subcommand info" -l cancel-job -d 'Cancel a background job by ID' -r
complete -c apmw -n "__fish_apmw_using_subcommand info" -l manager -l use -d 'Override auto-detection and force a specific package manager' -r
complete -c apmw -n "__fish_apmw_using_subcommand info" -l json -d 'Emit output as JSON (machine-readable)'
complete -c apmw -n "__fish_apmw_using_subcommand info" -l human -d 'Human-readable output (escape hatch for AXI/TOON mode)'
complete -c apmw -n "__fish_apmw_using_subcommand info" -s v -l verbose -d 'Increase verbosity (can be repeated: -v, -vv)'
complete -c apmw -n "__fish_apmw_using_subcommand info" -s q -l quiet -d 'Suppress non-error output'
complete -c apmw -n "__fish_apmw_using_subcommand info" -l debug -d 'Enable debug-level diagnostics'
complete -c apmw -n "__fish_apmw_using_subcommand info" -l dry-run -d 'Show what would happen without making changes'
complete -c apmw -n "__fish_apmw_using_subcommand info" -l force -d 'Force the operation, bypassing confirmations'
complete -c apmw -n "__fish_apmw_using_subcommand info" -l interactive -l tui -d 'Enable interactive/TUI mode'
complete -c apmw -n "__fish_apmw_using_subcommand info" -l no-pager -d 'Disable pager output'
complete -c apmw -n "__fish_apmw_using_subcommand info" -l no-scan -d 'Skip security scanning'
complete -c apmw -n "__fish_apmw_using_subcommand info" -l scan-only -d 'Only run the security scan, then exit'
complete -c apmw -n "__fish_apmw_using_subcommand info" -l update-security-db -d 'Update the security vulnerability database before scanning'
complete -c apmw -n "__fish_apmw_using_subcommand info" -l full -d 'Show full content (escape hatch for AXI truncation)'
complete -c apmw -n "__fish_apmw_using_subcommand info" -l daemon -d 'Run in daemon mode (long-running background process)'
complete -c apmw -n "__fish_apmw_using_subcommand info" -l no-daemon -d 'Disable daemon mode (force synchronous operation)'
complete -c apmw -n "__fish_apmw_using_subcommand info" -l list-jobs -d 'List background jobs'
complete -c apmw -n "__fish_apmw_using_subcommand info" -l no-telemetry -d 'Disable telemetry collection for this invocation'
complete -c apmw -n "__fish_apmw_using_subcommand info" -l telemetry-preview -d 'Print the telemetry payload that would be sent without actually sending it'
complete -c apmw -n "__fish_apmw_using_subcommand info" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l color -d 'Color output: auto, always, never' -r
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l on-risk -d 'Action to take when a security scan detects risk' -r
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l fields -d 'Comma-separated list of fields to include in AXI minimal output' -r
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l cancel-job -d 'Cancel a background job by ID' -r
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l manager -l use -d 'Override auto-detection and force a specific package manager' -r
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l json -d 'Emit output as JSON (machine-readable)'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l human -d 'Human-readable output (escape hatch for AXI/TOON mode)'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -s v -l verbose -d 'Increase verbosity (can be repeated: -v, -vv)'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -s q -l quiet -d 'Suppress non-error output'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l debug -d 'Enable debug-level diagnostics'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l dry-run -d 'Show what would happen without making changes'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l force -d 'Force the operation, bypassing confirmations'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l interactive -l tui -d 'Enable interactive/TUI mode'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l no-pager -d 'Disable pager output'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l no-scan -d 'Skip security scanning'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l scan-only -d 'Only run the security scan, then exit'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l update-security-db -d 'Update the security vulnerability database before scanning'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l full -d 'Show full content (escape hatch for AXI truncation)'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l daemon -d 'Run in daemon mode (long-running background process)'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l no-daemon -d 'Disable daemon mode (force synchronous operation)'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l list-jobs -d 'List background jobs'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l no-telemetry -d 'Disable telemetry collection for this invocation'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -l telemetry-preview -d 'Print the telemetry payload that would be sent without actually sending it'
complete -c apmw -n "__fish_apmw_using_subcommand audit-log" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l color -d 'Color output: auto, always, never' -r
complete -c apmw -n "__fish_apmw_using_subcommand config" -l on-risk -d 'Action to take when a security scan detects risk' -r
complete -c apmw -n "__fish_apmw_using_subcommand config" -l fields -d 'Comma-separated list of fields to include in AXI minimal output' -r
complete -c apmw -n "__fish_apmw_using_subcommand config" -l cancel-job -d 'Cancel a background job by ID' -r
complete -c apmw -n "__fish_apmw_using_subcommand config" -l manager -l use -d 'Override auto-detection and force a specific package manager' -r
complete -c apmw -n "__fish_apmw_using_subcommand config" -l init -d 'Initialize the config file with defaults'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l show -d 'Show the resolved configuration'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l json -d 'Emit output as JSON (machine-readable)'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l human -d 'Human-readable output (escape hatch for AXI/TOON mode)'
complete -c apmw -n "__fish_apmw_using_subcommand config" -s v -l verbose -d 'Increase verbosity (can be repeated: -v, -vv)'
complete -c apmw -n "__fish_apmw_using_subcommand config" -s q -l quiet -d 'Suppress non-error output'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l debug -d 'Enable debug-level diagnostics'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l dry-run -d 'Show what would happen without making changes'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l force -d 'Force the operation, bypassing confirmations'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l interactive -l tui -d 'Enable interactive/TUI mode'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l no-pager -d 'Disable pager output'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l no-scan -d 'Skip security scanning'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l scan-only -d 'Only run the security scan, then exit'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l update-security-db -d 'Update the security vulnerability database before scanning'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l full -d 'Show full content (escape hatch for AXI truncation)'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l daemon -d 'Run in daemon mode (long-running background process)'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l no-daemon -d 'Disable daemon mode (force synchronous operation)'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l list-jobs -d 'List background jobs'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l no-telemetry -d 'Disable telemetry collection for this invocation'
complete -c apmw -n "__fish_apmw_using_subcommand config" -l telemetry-preview -d 'Print the telemetry payload that would be sent without actually sending it'
complete -c apmw -n "__fish_apmw_using_subcommand config" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l color -d 'Color output: auto, always, never' -r
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l on-risk -d 'Action to take when a security scan detects risk' -r
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l fields -d 'Comma-separated list of fields to include in AXI minimal output' -r
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l cancel-job -d 'Cancel a background job by ID' -r
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l manager -l use -d 'Override auto-detection and force a specific package manager' -r
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l json -d 'Emit output as JSON (machine-readable)'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l human -d 'Human-readable output (escape hatch for AXI/TOON mode)'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -s v -l verbose -d 'Increase verbosity (can be repeated: -v, -vv)'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -s q -l quiet -d 'Suppress non-error output'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l debug -d 'Enable debug-level diagnostics'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l dry-run -d 'Show what would happen without making changes'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l force -d 'Force the operation, bypassing confirmations'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l interactive -l tui -d 'Enable interactive/TUI mode'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l no-pager -d 'Disable pager output'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l no-scan -d 'Skip security scanning'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l scan-only -d 'Only run the security scan, then exit'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l update-security-db -d 'Update the security vulnerability database before scanning'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l full -d 'Show full content (escape hatch for AXI truncation)'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l daemon -d 'Run in daemon mode (long-running background process)'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l no-daemon -d 'Disable daemon mode (force synchronous operation)'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l list-jobs -d 'List background jobs'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l no-telemetry -d 'Disable telemetry collection for this invocation'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -l telemetry-preview -d 'Print the telemetry payload that would be sent without actually sending it'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -f -a "refresh" -d 'Force-refresh the cached governance spec from levonk-packages'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and not __fish_seen_subcommand_from refresh help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l color -d 'Color output: auto, always, never' -r
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l on-risk -d 'Action to take when a security scan detects risk' -r
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l fields -d 'Comma-separated list of fields to include in AXI minimal output' -r
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l cancel-job -d 'Cancel a background job by ID' -r
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l manager -l use -d 'Override auto-detection and force a specific package manager' -r
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l json -d 'Emit output as JSON (machine-readable)'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l human -d 'Human-readable output (escape hatch for AXI/TOON mode)'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -s v -l verbose -d 'Increase verbosity (can be repeated: -v, -vv)'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -s q -l quiet -d 'Suppress non-error output'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l debug -d 'Enable debug-level diagnostics'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l dry-run -d 'Show what would happen without making changes'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l force -d 'Force the operation, bypassing confirmations'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l interactive -l tui -d 'Enable interactive/TUI mode'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l no-pager -d 'Disable pager output'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l no-scan -d 'Skip security scanning'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l scan-only -d 'Only run the security scan, then exit'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l update-security-db -d 'Update the security vulnerability database before scanning'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l full -d 'Show full content (escape hatch for AXI truncation)'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l daemon -d 'Run in daemon mode (long-running background process)'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l no-daemon -d 'Disable daemon mode (force synchronous operation)'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l list-jobs -d 'List background jobs'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l no-telemetry -d 'Disable telemetry collection for this invocation'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -l telemetry-preview -d 'Print the telemetry payload that would be sent without actually sending it'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from refresh" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from help" -f -a "refresh" -d 'Force-refresh the cached governance spec from levonk-packages'
complete -c apmw -n "__fish_apmw_using_subcommand governance; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l color -d 'Color output: auto, always, never' -r
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l on-risk -d 'Action to take when a security scan detects risk' -r
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l fields -d 'Comma-separated list of fields to include in AXI minimal output' -r
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l cancel-job -d 'Cancel a background job by ID' -r
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l manager -l use -d 'Override auto-detection and force a specific package manager' -r
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l json -d 'Emit output as JSON (machine-readable)'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l human -d 'Human-readable output (escape hatch for AXI/TOON mode)'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -s v -l verbose -d 'Increase verbosity (can be repeated: -v, -vv)'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -s q -l quiet -d 'Suppress non-error output'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l debug -d 'Enable debug-level diagnostics'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l dry-run -d 'Show what would happen without making changes'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l force -d 'Force the operation, bypassing confirmations'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l interactive -l tui -d 'Enable interactive/TUI mode'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l no-pager -d 'Disable pager output'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l no-scan -d 'Skip security scanning'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l scan-only -d 'Only run the security scan, then exit'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l update-security-db -d 'Update the security vulnerability database before scanning'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l full -d 'Show full content (escape hatch for AXI truncation)'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l daemon -d 'Run in daemon mode (long-running background process)'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l no-daemon -d 'Disable daemon mode (force synchronous operation)'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l list-jobs -d 'List background jobs'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l no-telemetry -d 'Disable telemetry collection for this invocation'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -l telemetry-preview -d 'Print the telemetry payload that would be sent without actually sending it'
complete -c apmw -n "__fish_apmw_using_subcommand intercept" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l color -d 'Color output: auto, always, never' -r
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l on-risk -d 'Action to take when a security scan detects risk' -r
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l fields -d 'Comma-separated list of fields to include in AXI minimal output' -r
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l cancel-job -d 'Cancel a background job by ID' -r
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l manager -l use -d 'Override auto-detection and force a specific package manager' -r
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l json -d 'Emit output as JSON (machine-readable)'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l human -d 'Human-readable output (escape hatch for AXI/TOON mode)'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -s v -l verbose -d 'Increase verbosity (can be repeated: -v, -vv)'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -s q -l quiet -d 'Suppress non-error output'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l debug -d 'Enable debug-level diagnostics'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l dry-run -d 'Show what would happen without making changes'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l force -d 'Force the operation, bypassing confirmations'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l interactive -l tui -d 'Enable interactive/TUI mode'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l no-pager -d 'Disable pager output'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l no-scan -d 'Skip security scanning'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l scan-only -d 'Only run the security scan, then exit'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l update-security-db -d 'Update the security vulnerability database before scanning'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l full -d 'Show full content (escape hatch for AXI truncation)'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l daemon -d 'Run in daemon mode (long-running background process)'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l no-daemon -d 'Disable daemon mode (force synchronous operation)'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l list-jobs -d 'List background jobs'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l no-telemetry -d 'Disable telemetry collection for this invocation'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -l telemetry-preview -d 'Print the telemetry payload that would be sent without actually sending it'
complete -c apmw -n "__fish_apmw_using_subcommand mcp" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c apmw -n "__fish_apmw_using_subcommand help; and not __fish_seen_subcommand_from install detect status clone scan suggest info audit-log config governance intercept mcp help" -f -a "install" -d 'Install a package or tool'
complete -c apmw -n "__fish_apmw_using_subcommand help; and not __fish_seen_subcommand_from install detect status clone scan suggest info audit-log config governance intercept mcp help" -f -a "detect" -d 'Detect the package manager for the current project'
complete -c apmw -n "__fish_apmw_using_subcommand help; and not __fish_seen_subcommand_from install detect status clone scan suggest info audit-log config governance intercept mcp help" -f -a "status" -d 'Show apmw status and audit log'
complete -c apmw -n "__fish_apmw_using_subcommand help; and not __fish_seen_subcommand_from install detect status clone scan suggest info audit-log config governance intercept mcp help" -f -a "clone" -d 'Create a historyless clone of a repository with AST indexing'
complete -c apmw -n "__fish_apmw_using_subcommand help; and not __fish_seen_subcommand_from install detect status clone scan suggest info audit-log config governance intercept mcp help" -f -a "scan" -d 'Scan a package (or the current project) for security issues'
complete -c apmw -n "__fish_apmw_using_subcommand help; and not __fish_seen_subcommand_from install detect status clone scan suggest info audit-log config governance intercept mcp help" -f -a "suggest" -d 'Suggest within-ecosystem alternatives for a package'
complete -c apmw -n "__fish_apmw_using_subcommand help; and not __fish_seen_subcommand_from install detect status clone scan suggest info audit-log config governance intercept mcp help" -f -a "info" -d 'Show detailed info about a package'
complete -c apmw -n "__fish_apmw_using_subcommand help; and not __fish_seen_subcommand_from install detect status clone scan suggest info audit-log config governance intercept mcp help" -f -a "audit-log" -d 'Show the audit log of past operations'
complete -c apmw -n "__fish_apmw_using_subcommand help; and not __fish_seen_subcommand_from install detect status clone scan suggest info audit-log config governance intercept mcp help" -f -a "config" -d 'View or initialize apmw configuration'
complete -c apmw -n "__fish_apmw_using_subcommand help; and not __fish_seen_subcommand_from install detect status clone scan suggest info audit-log config governance intercept mcp help" -f -a "governance" -d 'Manage governance rules (refresh the spec, show current rules)'
complete -c apmw -n "__fish_apmw_using_subcommand help; and not __fish_seen_subcommand_from install detect status clone scan suggest info audit-log config governance intercept mcp help" -f -a "intercept" -d 'Intercept a package manager call (invoked by PATH shims)'
complete -c apmw -n "__fish_apmw_using_subcommand help; and not __fish_seen_subcommand_from install detect status clone scan suggest info audit-log config governance intercept mcp help" -f -a "mcp" -d 'Start the MCP (Model Context Protocol) server over stdio'
complete -c apmw -n "__fish_apmw_using_subcommand help; and not __fish_seen_subcommand_from install detect status clone scan suggest info audit-log config governance intercept mcp help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c apmw -n "__fish_apmw_using_subcommand help; and __fish_seen_subcommand_from governance" -f -a "refresh" -d 'Force-refresh the cached governance spec from levonk-packages'
