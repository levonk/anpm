_apmw() {
    local i cur prev opts cmd
    COMPREPLY=()
    if [[ "${BASH_VERSINFO[0]}" -ge 4 ]]; then
        cur="$2"
    else
        cur="${COMP_WORDS[COMP_CWORD]}"
    fi
    prev="$3"
    cmd=""
    opts=""

    for i in "${COMP_WORDS[@]:0:COMP_CWORD}"
    do
        case "${cmd},${i}" in
            ",$1")
                cmd="apmw"
                ;;
            apmw,audit-log)
                cmd="apmw__subcmd__audit__subcmd__log"
                ;;
            apmw,clone)
                cmd="apmw__subcmd__clone"
                ;;
            apmw,config)
                cmd="apmw__subcmd__config"
                ;;
            apmw,detect)
                cmd="apmw__subcmd__detect"
                ;;
            apmw,governance)
                cmd="apmw__subcmd__governance"
                ;;
            apmw,help)
                cmd="apmw__subcmd__help"
                ;;
            apmw,info)
                cmd="apmw__subcmd__info"
                ;;
            apmw,install)
                cmd="apmw__subcmd__install"
                ;;
            apmw,intercept)
                cmd="apmw__subcmd__intercept"
                ;;
            apmw,mcp)
                cmd="apmw__subcmd__mcp"
                ;;
            apmw,scan)
                cmd="apmw__subcmd__scan"
                ;;
            apmw,status)
                cmd="apmw__subcmd__status"
                ;;
            apmw,suggest)
                cmd="apmw__subcmd__suggest"
                ;;
            apmw__subcmd__governance,help)
                cmd="apmw__subcmd__governance__subcmd__help"
                ;;
            apmw__subcmd__governance,refresh)
                cmd="apmw__subcmd__governance__subcmd__refresh"
                ;;
            apmw__subcmd__governance__subcmd__help,help)
                cmd="apmw__subcmd__governance__subcmd__help__subcmd__help"
                ;;
            apmw__subcmd__governance__subcmd__help,refresh)
                cmd="apmw__subcmd__governance__subcmd__help__subcmd__refresh"
                ;;
            apmw__subcmd__help,audit-log)
                cmd="apmw__subcmd__help__subcmd__audit__subcmd__log"
                ;;
            apmw__subcmd__help,clone)
                cmd="apmw__subcmd__help__subcmd__clone"
                ;;
            apmw__subcmd__help,config)
                cmd="apmw__subcmd__help__subcmd__config"
                ;;
            apmw__subcmd__help,detect)
                cmd="apmw__subcmd__help__subcmd__detect"
                ;;
            apmw__subcmd__help,governance)
                cmd="apmw__subcmd__help__subcmd__governance"
                ;;
            apmw__subcmd__help,help)
                cmd="apmw__subcmd__help__subcmd__help"
                ;;
            apmw__subcmd__help,info)
                cmd="apmw__subcmd__help__subcmd__info"
                ;;
            apmw__subcmd__help,install)
                cmd="apmw__subcmd__help__subcmd__install"
                ;;
            apmw__subcmd__help,intercept)
                cmd="apmw__subcmd__help__subcmd__intercept"
                ;;
            apmw__subcmd__help,mcp)
                cmd="apmw__subcmd__help__subcmd__mcp"
                ;;
            apmw__subcmd__help,scan)
                cmd="apmw__subcmd__help__subcmd__scan"
                ;;
            apmw__subcmd__help,status)
                cmd="apmw__subcmd__help__subcmd__status"
                ;;
            apmw__subcmd__help,suggest)
                cmd="apmw__subcmd__help__subcmd__suggest"
                ;;
            apmw__subcmd__help__subcmd__governance,refresh)
                cmd="apmw__subcmd__help__subcmd__governance__subcmd__refresh"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        apmw)
            opts="-v -q -h -V --install --uninstall --intercept --shell --man --usage --json --color --human --verbose --quiet --debug --dry-run --force --tui --interactive --no-pager --no-scan --scan-only --on-risk --update-security-db --fields --full --daemon --no-daemon --list-jobs --cancel-job --use --manager --no-telemetry --telemetry-preview --help --version install detect status clone scan suggest info audit-log config governance intercept mcp help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --shell)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --on-risk)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fields)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cancel-job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --manager)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --use)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__audit__subcmd__log)
            opts="-v -q -h --json --color --human --verbose --quiet --debug --dry-run --force --tui --interactive --no-pager --no-scan --scan-only --on-risk --update-security-db --fields --full --daemon --no-daemon --list-jobs --cancel-job --use --manager --no-telemetry --telemetry-preview --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --on-risk)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fields)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cancel-job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --manager)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --use)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__clone)
            opts="-v -q -h --json --color --human --verbose --quiet --debug --dry-run --force --tui --interactive --no-pager --no-scan --scan-only --on-risk --update-security-db --fields --full --daemon --no-daemon --list-jobs --cancel-job --use --manager --no-telemetry --telemetry-preview --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --on-risk)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fields)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cancel-job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --manager)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --use)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__config)
            opts="-v -q -h --init --show --json --color --human --verbose --quiet --debug --dry-run --force --tui --interactive --no-pager --no-scan --scan-only --on-risk --update-security-db --fields --full --daemon --no-daemon --list-jobs --cancel-job --use --manager --no-telemetry --telemetry-preview --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --on-risk)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fields)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cancel-job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --manager)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --use)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__detect)
            opts="-v -q -h --json --color --human --verbose --quiet --debug --dry-run --force --tui --interactive --no-pager --no-scan --scan-only --on-risk --update-security-db --fields --full --daemon --no-daemon --list-jobs --cancel-job --use --manager --no-telemetry --telemetry-preview --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --on-risk)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fields)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cancel-job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --manager)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --use)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__governance)
            opts="-v -q -h --json --color --human --verbose --quiet --debug --dry-run --force --tui --interactive --no-pager --no-scan --scan-only --on-risk --update-security-db --fields --full --daemon --no-daemon --list-jobs --cancel-job --use --manager --no-telemetry --telemetry-preview --help refresh help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --on-risk)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fields)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cancel-job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --manager)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --use)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__governance__subcmd__help)
            opts="refresh help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__governance__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__governance__subcmd__help__subcmd__refresh)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__governance__subcmd__refresh)
            opts="-v -q -h --json --color --human --verbose --quiet --debug --dry-run --force --tui --interactive --no-pager --no-scan --scan-only --on-risk --update-security-db --fields --full --daemon --no-daemon --list-jobs --cancel-job --use --manager --no-telemetry --telemetry-preview --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --on-risk)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fields)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cancel-job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --manager)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --use)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__help)
            opts="install detect status clone scan suggest info audit-log config governance intercept mcp help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__help__subcmd__audit__subcmd__log)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__help__subcmd__clone)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__help__subcmd__config)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__help__subcmd__detect)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__help__subcmd__governance)
            opts="refresh"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__help__subcmd__governance__subcmd__refresh)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__help__subcmd__info)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__help__subcmd__install)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__help__subcmd__intercept)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__help__subcmd__mcp)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__help__subcmd__scan)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__help__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__help__subcmd__suggest)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__info)
            opts="-v -q -h --json --color --human --verbose --quiet --debug --dry-run --force --tui --interactive --no-pager --no-scan --scan-only --on-risk --update-security-db --fields --full --daemon --no-daemon --list-jobs --cancel-job --use --manager --no-telemetry --telemetry-preview --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --on-risk)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fields)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cancel-job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --manager)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --use)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__install)
            opts="-v -q -h --dev --json --color --human --verbose --quiet --debug --dry-run --force --tui --interactive --no-pager --no-scan --scan-only --on-risk --update-security-db --fields --full --daemon --no-daemon --list-jobs --cancel-job --use --manager --no-telemetry --telemetry-preview --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --on-risk)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fields)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cancel-job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --manager)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --use)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__intercept)
            opts="-v -q -h --json --color --human --verbose --quiet --debug --dry-run --force --tui --interactive --no-pager --no-scan --scan-only --on-risk --update-security-db --fields --full --daemon --no-daemon --list-jobs --cancel-job --use --manager --no-telemetry --telemetry-preview --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --on-risk)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fields)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cancel-job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --manager)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --use)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__mcp)
            opts="-v -q -h --json --color --human --verbose --quiet --debug --dry-run --force --tui --interactive --no-pager --no-scan --scan-only --on-risk --update-security-db --fields --full --daemon --no-daemon --list-jobs --cancel-job --use --manager --no-telemetry --telemetry-preview --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --on-risk)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fields)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cancel-job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --manager)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --use)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__scan)
            opts="-v -q -h --json --color --human --verbose --quiet --debug --dry-run --force --tui --interactive --no-pager --no-scan --scan-only --on-risk --update-security-db --fields --full --daemon --no-daemon --list-jobs --cancel-job --use --manager --no-telemetry --telemetry-preview --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --on-risk)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fields)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cancel-job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --manager)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --use)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__status)
            opts="-v -q -h --json --color --human --verbose --quiet --debug --dry-run --force --tui --interactive --no-pager --no-scan --scan-only --on-risk --update-security-db --fields --full --daemon --no-daemon --list-jobs --cancel-job --use --manager --no-telemetry --telemetry-preview --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --on-risk)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fields)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cancel-job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --manager)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --use)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        apmw__subcmd__suggest)
            opts="-v -q -h --json --color --human --verbose --quiet --debug --dry-run --force --tui --interactive --no-pager --no-scan --scan-only --on-risk --update-security-db --fields --full --daemon --no-daemon --list-jobs --cancel-job --use --manager --no-telemetry --telemetry-preview --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --on-risk)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fields)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cancel-job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --manager)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --use)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F _apmw -o nosort -o bashdefault -o default apmw
else
    complete -F _apmw -o bashdefault -o default apmw
fi
