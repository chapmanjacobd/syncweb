#compdef syncweb

autoload -U is-at-least

_syncweb() {
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
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb_commands" \
"*::: :->syncweb" \
&& ret=0
    case $state in
    (syncweb)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-command-$line[1]:"
        case $line[1] in
            (version)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(repl)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(create)
_arguments "${_arguments_options[@]}" : \
'--mode=[]:MODE:_default' \
'--network=[Add the created folder to a named network]:NETWORK:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--relay-fallback[Enable Syncthing relay fallback for this folder]' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(join)
_arguments "${_arguments_options[@]}" : \
'--mode=[]:MODE:_default' \
'--network=[Add the joined folder to a named network]:NETWORK:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--relay-fallback[Enable Syncthing relay fallback for this folder]' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':ticket:_default' \
'::path:_files' \
&& ret=0
;;
(accept)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':namespace:_default' \
&& ret=0
;;
(drop)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':namespace:_default' \
&& ret=0
;;
(folders)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(devices)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(config)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__config_commands" \
"*::: :->config" \
&& ret=0

    case $state in
    (config)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-config-command-$line[1]:"
        case $line[1] in
            (set)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':key:_default' \
':value:_default' \
&& ret=0
;;
(show)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
'::section:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__config__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-config-help-command-$line[1]:"
        case $line[1] in
            (set)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(show)
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
(ls)
_arguments "${_arguments_options[@]}" : \
'--sort=[Collect and sort output instead of streaming it]:SORT:_default' \
'--threads=[Scanner threads (1 disables parallelism, 0 uses all available CPUs)]:THREADS:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(find)
_arguments "${_arguments_options[@]}" : \
'--kind=[]:KIND:(exact glob regex)' \
'--max-depth=[]:MAX_DEPTH:_default' \
'--min-size=[]:MIN_SIZE:_default' \
'--max-size=[]:MAX_SIZE:_default' \
'--extension=[]:EXTENSION:_default' \
'--type=[]:FILE_TYPE:(f d l)' \
'--threads=[Scanner threads (1 disables parallelism, 0 uses all available CPUs)]:THREADS:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':pattern:_default' \
'::path:_files' \
&& ret=0
;;
(sort)
_arguments "${_arguments_options[@]}" : \
'--by=[]:BY:(niche frecency peers random folder)' \
'--threads=[Scanner threads (1 disables parallelism, 0 uses all available CPUs)]:THREADS:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(stat)
_arguments "${_arguments_options[@]}" : \
'(--terse)--format=[]:FORMAT:_default' \
'--threads=[Scanner threads (1 disables parallelism, 0 uses all available CPUs)]:THREADS:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'(--format)--terse[]' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':path:_files' \
&& ret=0
;;
(download)
_arguments "${_arguments_options[@]}" : \
'--threads=[Copy threads (1 disables parallelism, 0 uses all available CPUs)]:THREADS:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':source:_files' \
':destination:_files' \
&& ret=0
;;
(init)
_arguments "${_arguments_options[@]}" : \
'--mode=[]:MODE:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(automatic)
_arguments "${_arguments_options[@]}" : \
'*--paths=[Paths evaluated by --dry-run]:PATHS:_files' \
'--filters=[Filter configuration (defaults to DATA_DIR/filters.toml)]:FILTERS:_files' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--show-filters[Print the active filter configuration and exit]' \
'--dry-run[Evaluate paths without starting the daemon]' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(subscribe)
_arguments "${_arguments_options[@]}" : \
'(--glob)--prefix=[]:PREFIX:_files' \
'(--prefix)--glob=[]:GLOB:_default' \
'--max-count=[]:MAX_COUNT:_default' \
'--max-size=[]:MAX_SIZE:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--ingest-only[Only deliver entries ingested after subscription]' \
'--ignore-self[Ignore events emitted by this subscription session]' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':ticket:_default' \
'::path:_files' \
&& ret=0
;;
(publish)
_arguments "${_arguments_options[@]}" : \
'--blob=[Publish this content hash as an unauthenticated blob ticket]:BLOB:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':namespace:_default' \
&& ret=0
;;
(unpublish)
_arguments "${_arguments_options[@]}" : \
'--blob=[]:BLOB:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':namespace:_default' \
&& ret=0
;;
(collection)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__collection_commands" \
"*::: :->collection" \
&& ret=0

    case $state in
    (collection)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-collection-command-$line[1]:"
        case $line[1] in
            (init)
_arguments "${_arguments_options[@]}" : \
'--version=[]:VERSION:_default' \
'--name=[]:NAME:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(add)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(versions)
_arguments "${_arguments_options[@]}" : \
'--version=[]:VERSION:_default' \
'--changelog=[]:CHANGELOG:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(publish)
_arguments "${_arguments_options[@]}" : \
'--namespace=[]:NAMESPACE:_default' \
'--sequence=[]:SEQUENCE:_default' \
'*--bootstrap=[]:NODE_ID:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__collection__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-collection-help-command-$line[1]:"
        case $line[1] in
            (init)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(versions)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(publish)
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
(package)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__package_commands" \
"*::: :->package" \
&& ret=0

    case $state in
    (package)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-package-command-$line[1]:"
        case $line[1] in
            (search)
_arguments "${_arguments_options[@]}" : \
'*--bootstrap=[]:NODE_ID:_default' \
'--timeout-ms=[]:TIMEOUT_MS:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
'::query:_default' \
&& ret=0
;;
(info)
_arguments "${_arguments_options[@]}" : \
'()--ticket=[]:TICKET:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
'::manifest:_files' \
&& ret=0
;;
(install)
_arguments "${_arguments_options[@]}" : \
'()--ticket=[]:TICKET:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
'::manifest:_files' \
'::source:_files' \
&& ret=0
;;
(upgrade)
_arguments "${_arguments_options[@]}" : \
'()--ticket=[]:TICKET:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
'::manifest:_files' \
'::source:_files' \
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':collection:_default' \
':version:_default' \
&& ret=0
;;
(verify)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':manifest:_files' \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(versions)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':collection:_default' \
&& ret=0
;;
(switch)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':collection:_default' \
':version:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__package__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-package-help-command-$line[1]:"
        case $line[1] in
            (search)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(info)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(install)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(upgrade)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(verify)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(versions)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(switch)
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
(network)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__network_commands" \
"*::: :->network" \
&& ret=0

    case $state in
    (network)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-network-command-$line[1]:"
        case $line[1] in
            (create)
_arguments "${_arguments_options[@]}" : \
'--label=[]:LABEL:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--invite-only[]' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
&& ret=0
;;
(ls)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
'::name:_default' \
&& ret=0
;;
(join)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':ticket:_default' \
&& ret=0
;;
(leave)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
&& ret=0
;;
(invite)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
'::device -- Optional Iroh node ID to bind the invitation to:_default' \
&& ret=0
;;
(kick)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
':device:_default' \
&& ret=0
;;
(test-relay)
_arguments "${_arguments_options[@]}" : \
'--relay-url=[]:RELAY_URL:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__network__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-network-help-command-$line[1]:"
        case $line[1] in
            (create)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(ls)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(join)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(leave)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(invite)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(kick)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(test-relay)
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
(completions)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
':shell:(bash elvish fish powershell zsh)' \
&& ret=0
;;
(manpages)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'-h[Print help]' \
'--help[Print help]' \
'::dir:_files' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-help-command-$line[1]:"
        case $line[1] in
            (version)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(repl)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(create)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(join)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(accept)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(drop)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(folders)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(devices)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(config)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__help__subcmd__config_commands" \
"*::: :->config" \
&& ret=0

    case $state in
    (config)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-help-config-command-$line[1]:"
        case $line[1] in
            (set)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(show)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(ls)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(find)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(sort)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(stat)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(download)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(init)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(automatic)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(subscribe)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(publish)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(unpublish)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(collection)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__help__subcmd__collection_commands" \
"*::: :->collection" \
&& ret=0

    case $state in
    (collection)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-help-collection-command-$line[1]:"
        case $line[1] in
            (init)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(versions)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(publish)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(package)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__help__subcmd__package_commands" \
"*::: :->package" \
&& ret=0

    case $state in
    (package)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-help-package-command-$line[1]:"
        case $line[1] in
            (search)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(info)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(install)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(upgrade)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(verify)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(versions)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(switch)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(network)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__help__subcmd__network_commands" \
"*::: :->network" \
&& ret=0

    case $state in
    (network)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-help-network-command-$line[1]:"
        case $line[1] in
            (create)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(ls)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(join)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(leave)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(invite)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(kick)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(test-relay)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(completions)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(manpages)
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

(( $+functions[_syncweb_commands] )) ||
_syncweb_commands() {
    local commands; commands=(
'version:Show syncweb version information' \
'repl:Start an interactive command shell' \
'create:Create a synchronized folder' \
'join:Join a folder from an Iroh document ticket' \
'accept:Accept a locally available folder' \
'drop:Remove a local folder replica' \
'folders:List managed folders' \
'devices:Show this device'\''s Iroh and Syncthing identities' \
'config:Show or update local configuration' \
'ls:List files in a local folder' \
'find:Search local files' \
'sort:Sort local files by discovery criteria' \
'stat:Show detailed metadata for a local file' \
'download:Download a local file to a destination' \
'init:Initialize a folder and print a shareable URL' \
'automatic:Run rules-based automatic synchronization' \
'subscribe:Subscribe to a folder with event filters' \
'publish:Publish a folder or blob for public read access' \
'unpublish:Remove a public blob pin' \
'collection:Create and publish versioned content collections' \
'package:Manage locally installed collection packages' \
'network:Network connectivity utilities' \
'completions:Generate shell completions' \
'manpages:Generate manpages' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__accept_commands] )) ||
_syncweb__subcmd__accept_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb accept commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__automatic_commands] )) ||
_syncweb__subcmd__automatic_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb automatic commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__collection_commands] )) ||
_syncweb__subcmd__collection_commands() {
    local commands; commands=(
'init:Initialize a directory as a versioned collection' \
'add:Scan files and update the local collection manifest' \
'versions:Create a new collection manifest version' \
'publish:Store a collection manifest and mutable head in a folder' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb collection commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__collection__subcmd__add_commands] )) ||
_syncweb__subcmd__collection__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb collection add commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__collection__subcmd__help_commands] )) ||
_syncweb__subcmd__collection__subcmd__help_commands() {
    local commands; commands=(
'init:Initialize a directory as a versioned collection' \
'add:Scan files and update the local collection manifest' \
'versions:Create a new collection manifest version' \
'publish:Store a collection manifest and mutable head in a folder' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb collection help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__collection__subcmd__help__subcmd__add_commands] )) ||
_syncweb__subcmd__collection__subcmd__help__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb collection help add commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__collection__subcmd__help__subcmd__help_commands] )) ||
_syncweb__subcmd__collection__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb collection help help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__collection__subcmd__help__subcmd__init_commands] )) ||
_syncweb__subcmd__collection__subcmd__help__subcmd__init_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb collection help init commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__collection__subcmd__help__subcmd__publish_commands] )) ||
_syncweb__subcmd__collection__subcmd__help__subcmd__publish_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb collection help publish commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__collection__subcmd__help__subcmd__versions_commands] )) ||
_syncweb__subcmd__collection__subcmd__help__subcmd__versions_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb collection help versions commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__collection__subcmd__init_commands] )) ||
_syncweb__subcmd__collection__subcmd__init_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb collection init commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__collection__subcmd__publish_commands] )) ||
_syncweb__subcmd__collection__subcmd__publish_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb collection publish commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__collection__subcmd__versions_commands] )) ||
_syncweb__subcmd__collection__subcmd__versions_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb collection versions commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__completions_commands] )) ||
_syncweb__subcmd__completions_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb completions commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__config_commands] )) ||
_syncweb__subcmd__config_commands() {
    local commands; commands=(
'set:Set a configuration value' \
'show:Show configuration, optionally limited to a section' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb config commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__config__subcmd__help_commands] )) ||
_syncweb__subcmd__config__subcmd__help_commands() {
    local commands; commands=(
'set:Set a configuration value' \
'show:Show configuration, optionally limited to a section' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb config help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__config__subcmd__help__subcmd__help_commands] )) ||
_syncweb__subcmd__config__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb config help help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__config__subcmd__help__subcmd__set_commands] )) ||
_syncweb__subcmd__config__subcmd__help__subcmd__set_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb config help set commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__config__subcmd__help__subcmd__show_commands] )) ||
_syncweb__subcmd__config__subcmd__help__subcmd__show_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb config help show commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__config__subcmd__set_commands] )) ||
_syncweb__subcmd__config__subcmd__set_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb config set commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__config__subcmd__show_commands] )) ||
_syncweb__subcmd__config__subcmd__show_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb config show commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__create_commands] )) ||
_syncweb__subcmd__create_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb create commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__devices_commands] )) ||
_syncweb__subcmd__devices_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb devices commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__download_commands] )) ||
_syncweb__subcmd__download_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb download commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__drop_commands] )) ||
_syncweb__subcmd__drop_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb drop commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__find_commands] )) ||
_syncweb__subcmd__find_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb find commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__folders_commands] )) ||
_syncweb__subcmd__folders_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb folders commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help_commands] )) ||
_syncweb__subcmd__help_commands() {
    local commands; commands=(
'version:Show syncweb version information' \
'repl:Start an interactive command shell' \
'create:Create a synchronized folder' \
'join:Join a folder from an Iroh document ticket' \
'accept:Accept a locally available folder' \
'drop:Remove a local folder replica' \
'folders:List managed folders' \
'devices:Show this device'\''s Iroh and Syncthing identities' \
'config:Show or update local configuration' \
'ls:List files in a local folder' \
'find:Search local files' \
'sort:Sort local files by discovery criteria' \
'stat:Show detailed metadata for a local file' \
'download:Download a local file to a destination' \
'init:Initialize a folder and print a shareable URL' \
'automatic:Run rules-based automatic synchronization' \
'subscribe:Subscribe to a folder with event filters' \
'publish:Publish a folder or blob for public read access' \
'unpublish:Remove a public blob pin' \
'collection:Create and publish versioned content collections' \
'package:Manage locally installed collection packages' \
'network:Network connectivity utilities' \
'completions:Generate shell completions' \
'manpages:Generate manpages' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__accept_commands] )) ||
_syncweb__subcmd__help__subcmd__accept_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help accept commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__automatic_commands] )) ||
_syncweb__subcmd__help__subcmd__automatic_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help automatic commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__collection_commands] )) ||
_syncweb__subcmd__help__subcmd__collection_commands() {
    local commands; commands=(
'init:Initialize a directory as a versioned collection' \
'add:Scan files and update the local collection manifest' \
'versions:Create a new collection manifest version' \
'publish:Store a collection manifest and mutable head in a folder' \
    )
    _describe -t commands 'syncweb help collection commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__collection__subcmd__add_commands] )) ||
_syncweb__subcmd__help__subcmd__collection__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help collection add commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__collection__subcmd__init_commands] )) ||
_syncweb__subcmd__help__subcmd__collection__subcmd__init_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help collection init commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__collection__subcmd__publish_commands] )) ||
_syncweb__subcmd__help__subcmd__collection__subcmd__publish_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help collection publish commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__collection__subcmd__versions_commands] )) ||
_syncweb__subcmd__help__subcmd__collection__subcmd__versions_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help collection versions commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__completions_commands] )) ||
_syncweb__subcmd__help__subcmd__completions_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help completions commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__config_commands] )) ||
_syncweb__subcmd__help__subcmd__config_commands() {
    local commands; commands=(
'set:Set a configuration value' \
'show:Show configuration, optionally limited to a section' \
    )
    _describe -t commands 'syncweb help config commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__config__subcmd__set_commands] )) ||
_syncweb__subcmd__help__subcmd__config__subcmd__set_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help config set commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__config__subcmd__show_commands] )) ||
_syncweb__subcmd__help__subcmd__config__subcmd__show_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help config show commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__create_commands] )) ||
_syncweb__subcmd__help__subcmd__create_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help create commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__devices_commands] )) ||
_syncweb__subcmd__help__subcmd__devices_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help devices commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__download_commands] )) ||
_syncweb__subcmd__help__subcmd__download_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help download commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__drop_commands] )) ||
_syncweb__subcmd__help__subcmd__drop_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help drop commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__find_commands] )) ||
_syncweb__subcmd__help__subcmd__find_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help find commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__folders_commands] )) ||
_syncweb__subcmd__help__subcmd__folders_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help folders commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__help_commands] )) ||
_syncweb__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__init_commands] )) ||
_syncweb__subcmd__help__subcmd__init_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help init commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__join_commands] )) ||
_syncweb__subcmd__help__subcmd__join_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help join commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__ls_commands] )) ||
_syncweb__subcmd__help__subcmd__ls_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help ls commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__manpages_commands] )) ||
_syncweb__subcmd__help__subcmd__manpages_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help manpages commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__network_commands] )) ||
_syncweb__subcmd__help__subcmd__network_commands() {
    local commands; commands=(
'create:Create a named network' \
'ls:List networks or inspect one' \
'join:Join a network from an invitation' \
'leave:Leave a network' \
'invite:Generate a network invitation' \
'kick:Remove a device from a network' \
'test-relay:Test a Syncthing relay TCP connection' \
    )
    _describe -t commands 'syncweb help network commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__network__subcmd__create_commands] )) ||
_syncweb__subcmd__help__subcmd__network__subcmd__create_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help network create commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__network__subcmd__invite_commands] )) ||
_syncweb__subcmd__help__subcmd__network__subcmd__invite_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help network invite commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__network__subcmd__join_commands] )) ||
_syncweb__subcmd__help__subcmd__network__subcmd__join_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help network join commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__network__subcmd__kick_commands] )) ||
_syncweb__subcmd__help__subcmd__network__subcmd__kick_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help network kick commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__network__subcmd__leave_commands] )) ||
_syncweb__subcmd__help__subcmd__network__subcmd__leave_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help network leave commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__network__subcmd__ls_commands] )) ||
_syncweb__subcmd__help__subcmd__network__subcmd__ls_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help network ls commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__network__subcmd__test-relay_commands] )) ||
_syncweb__subcmd__help__subcmd__network__subcmd__test-relay_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help network test-relay commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__package_commands] )) ||
_syncweb__subcmd__help__subcmd__package_commands() {
    local commands; commands=(
'search:List locally installed packages, optionally filtering by text' \
'info:Show a collection manifest' \
'install:Verify, stage, and atomically install a collection version' \
'upgrade:Install a newer collection manifest version' \
'remove:Remove a non-current installed collection version' \
'verify:Verify an installed collection version against its manifest' \
'list:List locally installed collections' \
'versions:List installed versions for a collection' \
'switch:Switch the active installed collection version' \
    )
    _describe -t commands 'syncweb help package commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__package__subcmd__info_commands] )) ||
_syncweb__subcmd__help__subcmd__package__subcmd__info_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help package info commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__package__subcmd__install_commands] )) ||
_syncweb__subcmd__help__subcmd__package__subcmd__install_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help package install commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__package__subcmd__list_commands] )) ||
_syncweb__subcmd__help__subcmd__package__subcmd__list_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help package list commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__package__subcmd__remove_commands] )) ||
_syncweb__subcmd__help__subcmd__package__subcmd__remove_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help package remove commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__package__subcmd__search_commands] )) ||
_syncweb__subcmd__help__subcmd__package__subcmd__search_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help package search commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__package__subcmd__switch_commands] )) ||
_syncweb__subcmd__help__subcmd__package__subcmd__switch_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help package switch commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__package__subcmd__upgrade_commands] )) ||
_syncweb__subcmd__help__subcmd__package__subcmd__upgrade_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help package upgrade commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__package__subcmd__verify_commands] )) ||
_syncweb__subcmd__help__subcmd__package__subcmd__verify_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help package verify commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__package__subcmd__versions_commands] )) ||
_syncweb__subcmd__help__subcmd__package__subcmd__versions_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help package versions commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__publish_commands] )) ||
_syncweb__subcmd__help__subcmd__publish_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help publish commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__repl_commands] )) ||
_syncweb__subcmd__help__subcmd__repl_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help repl commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__sort_commands] )) ||
_syncweb__subcmd__help__subcmd__sort_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help sort commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__stat_commands] )) ||
_syncweb__subcmd__help__subcmd__stat_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help stat commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__subscribe_commands] )) ||
_syncweb__subcmd__help__subcmd__subscribe_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help subscribe commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__unpublish_commands] )) ||
_syncweb__subcmd__help__subcmd__unpublish_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help unpublish commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__version_commands] )) ||
_syncweb__subcmd__help__subcmd__version_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help version commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__init_commands] )) ||
_syncweb__subcmd__init_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb init commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__join_commands] )) ||
_syncweb__subcmd__join_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb join commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__ls_commands] )) ||
_syncweb__subcmd__ls_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb ls commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__manpages_commands] )) ||
_syncweb__subcmd__manpages_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb manpages commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network_commands] )) ||
_syncweb__subcmd__network_commands() {
    local commands; commands=(
'create:Create a named network' \
'ls:List networks or inspect one' \
'join:Join a network from an invitation' \
'leave:Leave a network' \
'invite:Generate a network invitation' \
'kick:Remove a device from a network' \
'test-relay:Test a Syncthing relay TCP connection' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb network commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__create_commands] )) ||
_syncweb__subcmd__network__subcmd__create_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network create commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__help_commands] )) ||
_syncweb__subcmd__network__subcmd__help_commands() {
    local commands; commands=(
'create:Create a named network' \
'ls:List networks or inspect one' \
'join:Join a network from an invitation' \
'leave:Leave a network' \
'invite:Generate a network invitation' \
'kick:Remove a device from a network' \
'test-relay:Test a Syncthing relay TCP connection' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb network help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__help__subcmd__create_commands] )) ||
_syncweb__subcmd__network__subcmd__help__subcmd__create_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network help create commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__help__subcmd__help_commands] )) ||
_syncweb__subcmd__network__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network help help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__help__subcmd__invite_commands] )) ||
_syncweb__subcmd__network__subcmd__help__subcmd__invite_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network help invite commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__help__subcmd__join_commands] )) ||
_syncweb__subcmd__network__subcmd__help__subcmd__join_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network help join commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__help__subcmd__kick_commands] )) ||
_syncweb__subcmd__network__subcmd__help__subcmd__kick_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network help kick commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__help__subcmd__leave_commands] )) ||
_syncweb__subcmd__network__subcmd__help__subcmd__leave_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network help leave commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__help__subcmd__ls_commands] )) ||
_syncweb__subcmd__network__subcmd__help__subcmd__ls_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network help ls commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__help__subcmd__test-relay_commands] )) ||
_syncweb__subcmd__network__subcmd__help__subcmd__test-relay_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network help test-relay commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__invite_commands] )) ||
_syncweb__subcmd__network__subcmd__invite_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network invite commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__join_commands] )) ||
_syncweb__subcmd__network__subcmd__join_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network join commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__kick_commands] )) ||
_syncweb__subcmd__network__subcmd__kick_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network kick commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__leave_commands] )) ||
_syncweb__subcmd__network__subcmd__leave_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network leave commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__ls_commands] )) ||
_syncweb__subcmd__network__subcmd__ls_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network ls commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__test-relay_commands] )) ||
_syncweb__subcmd__network__subcmd__test-relay_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network test-relay commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package_commands] )) ||
_syncweb__subcmd__package_commands() {
    local commands; commands=(
'search:List locally installed packages, optionally filtering by text' \
'info:Show a collection manifest' \
'install:Verify, stage, and atomically install a collection version' \
'upgrade:Install a newer collection manifest version' \
'remove:Remove a non-current installed collection version' \
'verify:Verify an installed collection version against its manifest' \
'list:List locally installed collections' \
'versions:List installed versions for a collection' \
'switch:Switch the active installed collection version' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb package commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__help_commands] )) ||
_syncweb__subcmd__package__subcmd__help_commands() {
    local commands; commands=(
'search:List locally installed packages, optionally filtering by text' \
'info:Show a collection manifest' \
'install:Verify, stage, and atomically install a collection version' \
'upgrade:Install a newer collection manifest version' \
'remove:Remove a non-current installed collection version' \
'verify:Verify an installed collection version against its manifest' \
'list:List locally installed collections' \
'versions:List installed versions for a collection' \
'switch:Switch the active installed collection version' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb package help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__help__subcmd__help_commands] )) ||
_syncweb__subcmd__package__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package help help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__help__subcmd__info_commands] )) ||
_syncweb__subcmd__package__subcmd__help__subcmd__info_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package help info commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__help__subcmd__install_commands] )) ||
_syncweb__subcmd__package__subcmd__help__subcmd__install_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package help install commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__help__subcmd__list_commands] )) ||
_syncweb__subcmd__package__subcmd__help__subcmd__list_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package help list commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__help__subcmd__remove_commands] )) ||
_syncweb__subcmd__package__subcmd__help__subcmd__remove_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package help remove commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__help__subcmd__search_commands] )) ||
_syncweb__subcmd__package__subcmd__help__subcmd__search_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package help search commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__help__subcmd__switch_commands] )) ||
_syncweb__subcmd__package__subcmd__help__subcmd__switch_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package help switch commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__help__subcmd__upgrade_commands] )) ||
_syncweb__subcmd__package__subcmd__help__subcmd__upgrade_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package help upgrade commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__help__subcmd__verify_commands] )) ||
_syncweb__subcmd__package__subcmd__help__subcmd__verify_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package help verify commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__help__subcmd__versions_commands] )) ||
_syncweb__subcmd__package__subcmd__help__subcmd__versions_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package help versions commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__info_commands] )) ||
_syncweb__subcmd__package__subcmd__info_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package info commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__install_commands] )) ||
_syncweb__subcmd__package__subcmd__install_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package install commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__list_commands] )) ||
_syncweb__subcmd__package__subcmd__list_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package list commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__remove_commands] )) ||
_syncweb__subcmd__package__subcmd__remove_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package remove commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__search_commands] )) ||
_syncweb__subcmd__package__subcmd__search_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package search commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__switch_commands] )) ||
_syncweb__subcmd__package__subcmd__switch_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package switch commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__upgrade_commands] )) ||
_syncweb__subcmd__package__subcmd__upgrade_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package upgrade commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__verify_commands] )) ||
_syncweb__subcmd__package__subcmd__verify_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package verify commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__versions_commands] )) ||
_syncweb__subcmd__package__subcmd__versions_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package versions commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__publish_commands] )) ||
_syncweb__subcmd__publish_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb publish commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__repl_commands] )) ||
_syncweb__subcmd__repl_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb repl commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__sort_commands] )) ||
_syncweb__subcmd__sort_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb sort commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__stat_commands] )) ||
_syncweb__subcmd__stat_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb stat commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__subscribe_commands] )) ||
_syncweb__subcmd__subscribe_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb subscribe commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__unpublish_commands] )) ||
_syncweb__subcmd__unpublish_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb unpublish commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__version_commands] )) ||
_syncweb__subcmd__version_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb version commands' commands "$@"
}

if [ "$funcstack[1]" = "_syncweb" ]; then
    _syncweb "$@"
else
    compdef _syncweb syncweb
fi
