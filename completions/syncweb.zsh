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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(repl)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(start)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(shutdown)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(create)
_arguments "${_arguments_options[@]}" : \
'--prefix=[]:PREFIX:_files' \
'--mode=[]:MODE:_default' \
'--network=[Add the created folder to a named network]:NETWORK:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--relay-fallback[Enable Syncthing relay fallback for this folder]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(join)
_arguments "${_arguments_options[@]}" : \
'--prefix=[]:PREFIX:_files' \
'--mode=[]:MODE:_default' \
'--network=[Add the joined folder to a named network]:NETWORK:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--relay-fallback[Enable Syncthing relay fallback for this folder]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':namespace:_default' \
&& ret=0
;;
(drop)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':namespace:_default' \
&& ret=0
;;
(folders)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(devices)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(config)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':path:_files' \
&& ret=0
;;
(download)
_arguments "${_arguments_options[@]}" : \
'--max-peers=[Fetch only blobs with at most N observed peers]:MAX_PEERS:_default' \
'--min-peers=[Fetch only blobs with at least N observed peers]:MIN_PEERS:_default' \
'--min-count=[]:MIN_COUNT:_default' \
'--max-count=[]:MAX_COUNT:_default' \
'--threads=[Copy threads (1 disables parallelism, 0 uses all available CPUs)]:THREADS:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':source:_files' \
'::destination:_files' \
&& ret=0
;;
(import)
_arguments "${_arguments_options[@]}" : \
'--folder=[Folder namespace; defaults to the only managed folder]:FOLDER:_default' \
'--threads=[Import threads (1 disables parallelism, 0 uses all available CPUs)]:THREADS:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':path:_files' \
&& ret=0
;;
(snapshot)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__snapshot_commands" \
"*::: :->snapshot" \
&& ret=0

    case $state in
    (snapshot)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-snapshot-command-$line[1]:"
        case $line[1] in
            (create)
_arguments "${_arguments_options[@]}" : \
'--description=[]:DESCRIPTION:_default' \
'--threads=[Scanner threads (1 disables parallelism, 0 uses all available CPUs)]:THREADS:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(restore)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':path:_files' \
':snapshot:_default' \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(diff)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':path:_files' \
':first:_default' \
':second:_default' \
&& ret=0
;;
(delete)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':path:_files' \
':snapshot:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__snapshot__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-snapshot-help-command-$line[1]:"
        case $line[1] in
            (create)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(restore)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(diff)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(delete)
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
(health)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(init)
_arguments "${_arguments_options[@]}" : \
'--prefix=[]:PREFIX:_files' \
'--mode=[]:MODE:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(watch)
_arguments "${_arguments_options[@]}" : \
'--debounce-ms=[Debounce changes in milliseconds]:DEBOUNCE_MS:_default' \
'*--exclude=[Ignore a path glob; may be repeated]:GLOB:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--once[Process one event and exit]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(stats)
_arguments "${_arguments_options[@]}" : \
'--folder=[Limit display to a folder or namespace]:FOLDER:_files' \
'--peer=[Limit display to a peer node ID]:PEER:_default' \
'--period=[Retained for compatibility; counters are persisted since period start]:PERIOD:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--reset[Reset persisted counters before displaying them]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(verify)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(schedule)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__schedule_commands" \
"*::: :->schedule" \
&& ret=0

    case $state in
    (schedule)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-schedule-command-$line[1]:"
        case $line[1] in
            (set)
_arguments "${_arguments_options[@]}" : \
'--active=[]:ACTIVE:_default' \
'--bandwidth=[]:BANDWIDTH:_default' \
'--period=[]:PERIOD:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(folder)
_arguments "${_arguments_options[@]}" : \
'--active=[]:ACTIVE:_default' \
'--max-upload=[]:MAX_UPLOAD:_default' \
'--max-download=[]:MAX_DOWNLOAD:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__schedule__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-schedule-help-command-$line[1]:"
        case $line[1] in
            (set)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(folder)
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':namespace:_default' \
&& ret=0
;;
(collection)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(add)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
            (export)
_arguments "${_arguments_options[@]}" : \
'--version=[]:VERSION:_default' \
'*--filter=[]:EXPRESSION:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
'*::paths:_files' \
&& ret=0
;;
(import)
_arguments "${_arguments_options[@]}" : \
'*--filter=[]:EXPRESSION:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
'*::archives:_files' \
&& ret=0
;;
(search)
_arguments "${_arguments_options[@]}" : \
'*--bootstrap=[]:NODE_ID:_default' \
'--timeout-ms=[]:TIMEOUT_MS:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':manifest:_files' \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(versions)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':collection:_default' \
&& ret=0
;;
(switch)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
            (export)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(import)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
&& ret=0
;;
(ls)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
'::name:_default' \
&& ret=0
;;
(join)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':ticket:_default' \
&& ret=0
;;
(leave)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
&& ret=0
;;
(invite)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
(indexing)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__indexing_commands" \
"*::: :->indexing" \
&& ret=0

    case $state in
    (indexing)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-indexing-command-$line[1]:"
        case $line[1] in
            (enable)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':folder:_files' \
&& ret=0
;;
(disable)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':folder:_files' \
&& ret=0
;;
(publish)
_arguments "${_arguments_options[@]}" : \
'--catalog=[]:CATALOG:_default' \
'*--tag=[]:TAGS:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':folder:_files' \
&& ret=0
;;
(search)
_arguments "${_arguments_options[@]}" : \
'--limit=[]:LIMIT:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':query:_default' \
&& ret=0
;;
(health)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':hash:_default' \
&& ret=0
;;
(meta)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__indexing__subcmd__meta_commands" \
"*::: :->meta" \
&& ret=0

    case $state in
    (meta)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-indexing-meta-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
'--sequence=[]:SEQUENCE:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':hash:_default' \
':key:_default' \
':value:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__indexing__subcmd__meta__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-indexing-meta-help-command-$line[1]:"
        case $line[1] in
            (add)
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
(filter)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__indexing__subcmd__filter_commands" \
"*::: :->filter" \
&& ret=0

    case $state in
    (filter)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-indexing-filter-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':rule_type:(device file hash)' \
':value:_default' \
&& ret=0
;;
(subscribe)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':source:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__indexing__subcmd__filter__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-indexing-filter-help-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(subscribe)
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
(help)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__indexing__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-indexing-help-command-$line[1]:"
        case $line[1] in
            (enable)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(disable)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(publish)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(search)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(health)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(meta)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__indexing__subcmd__help__subcmd__meta_commands" \
"*::: :->meta" \
&& ret=0

    case $state in
    (meta)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-indexing-help-meta-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(filter)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__indexing__subcmd__help__subcmd__filter_commands" \
"*::: :->filter" \
&& ret=0

    case $state in
    (filter)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-indexing-help-filter-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(subscribe)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
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
(link)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__link_commands" \
"*::: :->link" \
&& ret=0

    case $state in
    (link)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-link-command-$line[1]:"
        case $line[1] in
            (create)
_arguments "${_arguments_options[@]}" : \
'(--private)--name=[]:NAME:_default' \
'--version=[]:VERSION:_default' \
'--sequence=[]:SEQUENCE:_default' \
'--expires=[Private-link expiration as a Unix timestamp]:EXPIRES:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'(--name)--private[]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':source:_files' \
&& ret=0
;;
(resolve)
_arguments "${_arguments_options[@]}" : \
'--version=[]:VERSION:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':link:_default' \
&& ret=0
;;
(revoke)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':link:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__link__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-link-help-command-$line[1]:"
        case $line[1] in
            (create)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(resolve)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(revoke)
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
(mirror)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__mirror_commands" \
"*::: :->mirror" \
&& ret=0

    case $state in
    (mirror)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-mirror-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':collection:_default' \
':provider:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__mirror__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-mirror-help-command-$line[1]:"
        case $line[1] in
            (add)
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
(trust)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__trust_commands" \
"*::: :->trust" \
&& ret=0

    case $state in
    (trust)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-trust-command-$line[1]:"
        case $line[1] in
            (show)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':subject:_default' \
&& ret=0
;;
(delegate)
_arguments "${_arguments_options[@]}" : \
'--expires=[]:EXPIRES:_default' \
'--scope=[]:SCOPE:_default' \
'--sequence=[]:SEQUENCE:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':publisher:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__trust__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-trust-help-command-$line[1]:"
        case $line[1] in
            (show)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(delegate)
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
(attest)
_arguments "${_arguments_options[@]}" : \
'(--provenance --derivative)--license=[]:LICENSE:_default' \
'(--license --derivative)--provenance=[]:PROVENANCE:_default' \
'(--license --provenance)--derivative=[]:DERIVATIVE:_default' \
'--sequence=[]:SEQUENCE:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':content:_default' \
&& ret=0
;;
(report)
_arguments "${_arguments_options[@]}" : \
'--reason=[]:REASON:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':record:_default' \
&& ret=0
;;
(moderation)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__moderation_commands" \
"*::: :->moderation" \
&& ret=0

    case $state in
    (moderation)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-moderation-command-$line[1]:"
        case $line[1] in
            (ls)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
'::content:_default' \
&& ret=0
;;
(hide)
_arguments "${_arguments_options[@]}" : \
'--reason=[]:REASON:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':record:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__moderation__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-moderation-help-command-$line[1]:"
        case $line[1] in
            (ls)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(hide)
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
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
'-h[Print help]' \
'--help[Print help]' \
':shell:(bash elvish fish powershell zsh)' \
&& ret=0
;;
(manpages)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON where supported]' \
'--no-color[Disable colored output]' \
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
(start)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(shutdown)
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
(import)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(snapshot)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__help__subcmd__snapshot_commands" \
"*::: :->snapshot" \
&& ret=0

    case $state in
    (snapshot)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-help-snapshot-command-$line[1]:"
        case $line[1] in
            (create)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(restore)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(diff)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(delete)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(health)
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
(watch)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(stats)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(verify)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(schedule)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__help__subcmd__schedule_commands" \
"*::: :->schedule" \
&& ret=0

    case $state in
    (schedule)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-help-schedule-command-$line[1]:"
        case $line[1] in
            (set)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(folder)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
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
            (export)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(import)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
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
(indexing)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__help__subcmd__indexing_commands" \
"*::: :->indexing" \
&& ret=0

    case $state in
    (indexing)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-help-indexing-command-$line[1]:"
        case $line[1] in
            (enable)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(disable)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(publish)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(search)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(health)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(meta)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__help__subcmd__indexing__subcmd__meta_commands" \
"*::: :->meta" \
&& ret=0

    case $state in
    (meta)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-help-indexing-meta-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(filter)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__help__subcmd__indexing__subcmd__filter_commands" \
"*::: :->filter" \
&& ret=0

    case $state in
    (filter)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-help-indexing-filter-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(subscribe)
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
(link)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__help__subcmd__link_commands" \
"*::: :->link" \
&& ret=0

    case $state in
    (link)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-help-link-command-$line[1]:"
        case $line[1] in
            (create)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(resolve)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(revoke)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(mirror)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__help__subcmd__mirror_commands" \
"*::: :->mirror" \
&& ret=0

    case $state in
    (mirror)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-help-mirror-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(trust)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__help__subcmd__trust_commands" \
"*::: :->trust" \
&& ret=0

    case $state in
    (trust)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-help-trust-command-$line[1]:"
        case $line[1] in
            (show)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(delegate)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(attest)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(report)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(moderation)
_arguments "${_arguments_options[@]}" : \
":: :_syncweb__subcmd__help__subcmd__moderation_commands" \
"*::: :->moderation" \
&& ret=0

    case $state in
    (moderation)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-help-moderation-command-$line[1]:"
        case $line[1] in
            (ls)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(hide)
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
'start:Start the local syncweb node for one command invocation' \
'shutdown:Stop the local syncweb node' \
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
'download:Download folder content or copy a local file' \
'import:Import local files into a synchronized folder' \
'snapshot:Manage content-addressed snapshots' \
'health:Show seeding status per folder blob' \
'init:Initialize a folder and print a shareable URL' \
'automatic:Run rules-based automatic synchronization' \
'watch:Watch a folder and import filesystem changes' \
'stats:Show persisted bandwidth accounting' \
'verify:Re-check local folder blob integrity' \
'schedule:Show or update synchronization schedules' \
'subscribe:Subscribe to a folder with event filters' \
'publish:Publish a folder or blob for public read access' \
'unpublish:Remove a public blob pin' \
'collection:Create and publish versioned content collections' \
'package:Manage locally installed collection packages' \
'network:Network connectivity utilities' \
'indexing:Manage opt-in indexing, catalogs, and metadata' \
'link:Create and resolve stable syncweb links' \
'mirror:Register alternate content providers' \
'trust:Inspect and delegate local trust' \
'attest:Sign content provenance attestations' \
'report:Submit a local moderation report' \
'moderation:Manage local moderation decisions' \
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
(( $+functions[_syncweb__subcmd__attest_commands] )) ||
_syncweb__subcmd__attest_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb attest commands' commands "$@"
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
(( $+functions[_syncweb__subcmd__health_commands] )) ||
_syncweb__subcmd__health_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb health commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help_commands] )) ||
_syncweb__subcmd__help_commands() {
    local commands; commands=(
'version:Show syncweb version information' \
'repl:Start an interactive command shell' \
'start:Start the local syncweb node for one command invocation' \
'shutdown:Stop the local syncweb node' \
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
'download:Download folder content or copy a local file' \
'import:Import local files into a synchronized folder' \
'snapshot:Manage content-addressed snapshots' \
'health:Show seeding status per folder blob' \
'init:Initialize a folder and print a shareable URL' \
'automatic:Run rules-based automatic synchronization' \
'watch:Watch a folder and import filesystem changes' \
'stats:Show persisted bandwidth accounting' \
'verify:Re-check local folder blob integrity' \
'schedule:Show or update synchronization schedules' \
'subscribe:Subscribe to a folder with event filters' \
'publish:Publish a folder or blob for public read access' \
'unpublish:Remove a public blob pin' \
'collection:Create and publish versioned content collections' \
'package:Manage locally installed collection packages' \
'network:Network connectivity utilities' \
'indexing:Manage opt-in indexing, catalogs, and metadata' \
'link:Create and resolve stable syncweb links' \
'mirror:Register alternate content providers' \
'trust:Inspect and delegate local trust' \
'attest:Sign content provenance attestations' \
'report:Submit a local moderation report' \
'moderation:Manage local moderation decisions' \
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
(( $+functions[_syncweb__subcmd__help__subcmd__attest_commands] )) ||
_syncweb__subcmd__help__subcmd__attest_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help attest commands' commands "$@"
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
(( $+functions[_syncweb__subcmd__help__subcmd__health_commands] )) ||
_syncweb__subcmd__help__subcmd__health_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help health commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__help_commands] )) ||
_syncweb__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__import_commands] )) ||
_syncweb__subcmd__help__subcmd__import_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help import commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__indexing_commands] )) ||
_syncweb__subcmd__help__subcmd__indexing_commands() {
    local commands; commands=(
'enable:Opt a synchronized folder into indexing' \
'disable:Remove a folder from the local index' \
'publish:Publish folder metadata to a catalog' \
'search:Search subscribed catalogs' \
'health:Show verified provider health for a content hash' \
'meta:Manage signed metadata' \
'filter:Manage local and federated denylists' \
    )
    _describe -t commands 'syncweb help indexing commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__indexing__subcmd__disable_commands] )) ||
_syncweb__subcmd__help__subcmd__indexing__subcmd__disable_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help indexing disable commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__indexing__subcmd__enable_commands] )) ||
_syncweb__subcmd__help__subcmd__indexing__subcmd__enable_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help indexing enable commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__indexing__subcmd__filter_commands] )) ||
_syncweb__subcmd__help__subcmd__indexing__subcmd__filter_commands() {
    local commands; commands=(
'add:Add a device, file, or hash denylist rule' \
'subscribe:Import a signed federated filter list' \
    )
    _describe -t commands 'syncweb help indexing filter commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__indexing__subcmd__filter__subcmd__add_commands] )) ||
_syncweb__subcmd__help__subcmd__indexing__subcmd__filter__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help indexing filter add commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__indexing__subcmd__filter__subcmd__subscribe_commands] )) ||
_syncweb__subcmd__help__subcmd__indexing__subcmd__filter__subcmd__subscribe_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help indexing filter subscribe commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__indexing__subcmd__health_commands] )) ||
_syncweb__subcmd__help__subcmd__indexing__subcmd__health_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help indexing health commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__indexing__subcmd__meta_commands] )) ||
_syncweb__subcmd__help__subcmd__indexing__subcmd__meta_commands() {
    local commands; commands=(
'add:Append signed metadata to a content hash' \
    )
    _describe -t commands 'syncweb help indexing meta commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__indexing__subcmd__meta__subcmd__add_commands] )) ||
_syncweb__subcmd__help__subcmd__indexing__subcmd__meta__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help indexing meta add commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__indexing__subcmd__publish_commands] )) ||
_syncweb__subcmd__help__subcmd__indexing__subcmd__publish_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help indexing publish commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__indexing__subcmd__search_commands] )) ||
_syncweb__subcmd__help__subcmd__indexing__subcmd__search_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help indexing search commands' commands "$@"
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
(( $+functions[_syncweb__subcmd__help__subcmd__link_commands] )) ||
_syncweb__subcmd__help__subcmd__link_commands() {
    local commands; commands=(
'create:Create an immutable, private, or mutable link' \
'resolve:Resolve a stable link' \
'revoke:Revoke a private capability link' \
    )
    _describe -t commands 'syncweb help link commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__link__subcmd__create_commands] )) ||
_syncweb__subcmd__help__subcmd__link__subcmd__create_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help link create commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__link__subcmd__resolve_commands] )) ||
_syncweb__subcmd__help__subcmd__link__subcmd__resolve_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help link resolve commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__link__subcmd__revoke_commands] )) ||
_syncweb__subcmd__help__subcmd__link__subcmd__revoke_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help link revoke commands' commands "$@"
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
(( $+functions[_syncweb__subcmd__help__subcmd__mirror_commands] )) ||
_syncweb__subcmd__help__subcmd__mirror_commands() {
    local commands; commands=(
'add:Register a blob ticket as an alternate provider' \
    )
    _describe -t commands 'syncweb help mirror commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__mirror__subcmd__add_commands] )) ||
_syncweb__subcmd__help__subcmd__mirror__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help mirror add commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__moderation_commands] )) ||
_syncweb__subcmd__help__subcmd__moderation_commands() {
    local commands; commands=(
'ls:List local moderation records' \
'hide:Hide a content record locally' \
    )
    _describe -t commands 'syncweb help moderation commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__moderation__subcmd__hide_commands] )) ||
_syncweb__subcmd__help__subcmd__moderation__subcmd__hide_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help moderation hide commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__moderation__subcmd__ls_commands] )) ||
_syncweb__subcmd__help__subcmd__moderation__subcmd__ls_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help moderation ls commands' commands "$@"
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
'export:Export one or more package directories as compressed CAR archive files' \
'import:Import and install a compressed CAR archive file' \
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
(( $+functions[_syncweb__subcmd__help__subcmd__package__subcmd__export_commands] )) ||
_syncweb__subcmd__help__subcmd__package__subcmd__export_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help package export commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__package__subcmd__import_commands] )) ||
_syncweb__subcmd__help__subcmd__package__subcmd__import_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help package import commands' commands "$@"
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
(( $+functions[_syncweb__subcmd__help__subcmd__report_commands] )) ||
_syncweb__subcmd__help__subcmd__report_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help report commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__schedule_commands] )) ||
_syncweb__subcmd__help__subcmd__schedule_commands() {
    local commands; commands=(
'set:Update the global schedule' \
'folder:Set schedule overrides for a named folder' \
    )
    _describe -t commands 'syncweb help schedule commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__schedule__subcmd__folder_commands] )) ||
_syncweb__subcmd__help__subcmd__schedule__subcmd__folder_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help schedule folder commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__schedule__subcmd__set_commands] )) ||
_syncweb__subcmd__help__subcmd__schedule__subcmd__set_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help schedule set commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__shutdown_commands] )) ||
_syncweb__subcmd__help__subcmd__shutdown_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help shutdown commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__snapshot_commands] )) ||
_syncweb__subcmd__help__subcmd__snapshot_commands() {
    local commands; commands=(
'create:Create a content-addressed snapshot' \
'restore:Restore a snapshot to a folder or directory' \
'list:List local snapshots' \
'diff:Compare two snapshots' \
'delete:Delete a snapshot and release its pins' \
    )
    _describe -t commands 'syncweb help snapshot commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__snapshot__subcmd__create_commands] )) ||
_syncweb__subcmd__help__subcmd__snapshot__subcmd__create_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help snapshot create commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__snapshot__subcmd__delete_commands] )) ||
_syncweb__subcmd__help__subcmd__snapshot__subcmd__delete_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help snapshot delete commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__snapshot__subcmd__diff_commands] )) ||
_syncweb__subcmd__help__subcmd__snapshot__subcmd__diff_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help snapshot diff commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__snapshot__subcmd__list_commands] )) ||
_syncweb__subcmd__help__subcmd__snapshot__subcmd__list_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help snapshot list commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__snapshot__subcmd__restore_commands] )) ||
_syncweb__subcmd__help__subcmd__snapshot__subcmd__restore_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help snapshot restore commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__sort_commands] )) ||
_syncweb__subcmd__help__subcmd__sort_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help sort commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__start_commands] )) ||
_syncweb__subcmd__help__subcmd__start_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help start commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__stat_commands] )) ||
_syncweb__subcmd__help__subcmd__stat_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help stat commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__stats_commands] )) ||
_syncweb__subcmd__help__subcmd__stats_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help stats commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__subscribe_commands] )) ||
_syncweb__subcmd__help__subcmd__subscribe_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help subscribe commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__trust_commands] )) ||
_syncweb__subcmd__help__subcmd__trust_commands() {
    local commands; commands=(
'show:Show trust and moderation state' \
'delegate:Delegate trust to a publisher identity' \
    )
    _describe -t commands 'syncweb help trust commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__trust__subcmd__delegate_commands] )) ||
_syncweb__subcmd__help__subcmd__trust__subcmd__delegate_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help trust delegate commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__trust__subcmd__show_commands] )) ||
_syncweb__subcmd__help__subcmd__trust__subcmd__show_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help trust show commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__unpublish_commands] )) ||
_syncweb__subcmd__help__subcmd__unpublish_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help unpublish commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__verify_commands] )) ||
_syncweb__subcmd__help__subcmd__verify_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help verify commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__version_commands] )) ||
_syncweb__subcmd__help__subcmd__version_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help version commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__watch_commands] )) ||
_syncweb__subcmd__help__subcmd__watch_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help watch commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__import_commands] )) ||
_syncweb__subcmd__import_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb import commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing_commands] )) ||
_syncweb__subcmd__indexing_commands() {
    local commands; commands=(
'enable:Opt a synchronized folder into indexing' \
'disable:Remove a folder from the local index' \
'publish:Publish folder metadata to a catalog' \
'search:Search subscribed catalogs' \
'health:Show verified provider health for a content hash' \
'meta:Manage signed metadata' \
'filter:Manage local and federated denylists' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb indexing commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__disable_commands] )) ||
_syncweb__subcmd__indexing__subcmd__disable_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing disable commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__enable_commands] )) ||
_syncweb__subcmd__indexing__subcmd__enable_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing enable commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__filter_commands] )) ||
_syncweb__subcmd__indexing__subcmd__filter_commands() {
    local commands; commands=(
'add:Add a device, file, or hash denylist rule' \
'subscribe:Import a signed federated filter list' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb indexing filter commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__filter__subcmd__add_commands] )) ||
_syncweb__subcmd__indexing__subcmd__filter__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing filter add commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__filter__subcmd__help_commands] )) ||
_syncweb__subcmd__indexing__subcmd__filter__subcmd__help_commands() {
    local commands; commands=(
'add:Add a device, file, or hash denylist rule' \
'subscribe:Import a signed federated filter list' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb indexing filter help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__filter__subcmd__help__subcmd__add_commands] )) ||
_syncweb__subcmd__indexing__subcmd__filter__subcmd__help__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing filter help add commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__filter__subcmd__help__subcmd__help_commands] )) ||
_syncweb__subcmd__indexing__subcmd__filter__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing filter help help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__filter__subcmd__help__subcmd__subscribe_commands] )) ||
_syncweb__subcmd__indexing__subcmd__filter__subcmd__help__subcmd__subscribe_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing filter help subscribe commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__filter__subcmd__subscribe_commands] )) ||
_syncweb__subcmd__indexing__subcmd__filter__subcmd__subscribe_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing filter subscribe commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__health_commands] )) ||
_syncweb__subcmd__indexing__subcmd__health_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing health commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__help_commands] )) ||
_syncweb__subcmd__indexing__subcmd__help_commands() {
    local commands; commands=(
'enable:Opt a synchronized folder into indexing' \
'disable:Remove a folder from the local index' \
'publish:Publish folder metadata to a catalog' \
'search:Search subscribed catalogs' \
'health:Show verified provider health for a content hash' \
'meta:Manage signed metadata' \
'filter:Manage local and federated denylists' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb indexing help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__help__subcmd__disable_commands] )) ||
_syncweb__subcmd__indexing__subcmd__help__subcmd__disable_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing help disable commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__help__subcmd__enable_commands] )) ||
_syncweb__subcmd__indexing__subcmd__help__subcmd__enable_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing help enable commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__help__subcmd__filter_commands] )) ||
_syncweb__subcmd__indexing__subcmd__help__subcmd__filter_commands() {
    local commands; commands=(
'add:Add a device, file, or hash denylist rule' \
'subscribe:Import a signed federated filter list' \
    )
    _describe -t commands 'syncweb indexing help filter commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__help__subcmd__filter__subcmd__add_commands] )) ||
_syncweb__subcmd__indexing__subcmd__help__subcmd__filter__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing help filter add commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__help__subcmd__filter__subcmd__subscribe_commands] )) ||
_syncweb__subcmd__indexing__subcmd__help__subcmd__filter__subcmd__subscribe_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing help filter subscribe commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__help__subcmd__health_commands] )) ||
_syncweb__subcmd__indexing__subcmd__help__subcmd__health_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing help health commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__help__subcmd__help_commands] )) ||
_syncweb__subcmd__indexing__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing help help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__help__subcmd__meta_commands] )) ||
_syncweb__subcmd__indexing__subcmd__help__subcmd__meta_commands() {
    local commands; commands=(
'add:Append signed metadata to a content hash' \
    )
    _describe -t commands 'syncweb indexing help meta commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__help__subcmd__meta__subcmd__add_commands] )) ||
_syncweb__subcmd__indexing__subcmd__help__subcmd__meta__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing help meta add commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__help__subcmd__publish_commands] )) ||
_syncweb__subcmd__indexing__subcmd__help__subcmd__publish_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing help publish commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__help__subcmd__search_commands] )) ||
_syncweb__subcmd__indexing__subcmd__help__subcmd__search_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing help search commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__meta_commands] )) ||
_syncweb__subcmd__indexing__subcmd__meta_commands() {
    local commands; commands=(
'add:Append signed metadata to a content hash' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb indexing meta commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__meta__subcmd__add_commands] )) ||
_syncweb__subcmd__indexing__subcmd__meta__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing meta add commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__meta__subcmd__help_commands] )) ||
_syncweb__subcmd__indexing__subcmd__meta__subcmd__help_commands() {
    local commands; commands=(
'add:Append signed metadata to a content hash' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb indexing meta help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__meta__subcmd__help__subcmd__add_commands] )) ||
_syncweb__subcmd__indexing__subcmd__meta__subcmd__help__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing meta help add commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__meta__subcmd__help__subcmd__help_commands] )) ||
_syncweb__subcmd__indexing__subcmd__meta__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing meta help help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__publish_commands] )) ||
_syncweb__subcmd__indexing__subcmd__publish_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing publish commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__search_commands] )) ||
_syncweb__subcmd__indexing__subcmd__search_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing search commands' commands "$@"
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
(( $+functions[_syncweb__subcmd__link_commands] )) ||
_syncweb__subcmd__link_commands() {
    local commands; commands=(
'create:Create an immutable, private, or mutable link' \
'resolve:Resolve a stable link' \
'revoke:Revoke a private capability link' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb link commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__link__subcmd__create_commands] )) ||
_syncweb__subcmd__link__subcmd__create_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb link create commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__link__subcmd__help_commands] )) ||
_syncweb__subcmd__link__subcmd__help_commands() {
    local commands; commands=(
'create:Create an immutable, private, or mutable link' \
'resolve:Resolve a stable link' \
'revoke:Revoke a private capability link' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb link help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__link__subcmd__help__subcmd__create_commands] )) ||
_syncweb__subcmd__link__subcmd__help__subcmd__create_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb link help create commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__link__subcmd__help__subcmd__help_commands] )) ||
_syncweb__subcmd__link__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb link help help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__link__subcmd__help__subcmd__resolve_commands] )) ||
_syncweb__subcmd__link__subcmd__help__subcmd__resolve_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb link help resolve commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__link__subcmd__help__subcmd__revoke_commands] )) ||
_syncweb__subcmd__link__subcmd__help__subcmd__revoke_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb link help revoke commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__link__subcmd__resolve_commands] )) ||
_syncweb__subcmd__link__subcmd__resolve_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb link resolve commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__link__subcmd__revoke_commands] )) ||
_syncweb__subcmd__link__subcmd__revoke_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb link revoke commands' commands "$@"
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
(( $+functions[_syncweb__subcmd__mirror_commands] )) ||
_syncweb__subcmd__mirror_commands() {
    local commands; commands=(
'add:Register a blob ticket as an alternate provider' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb mirror commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__mirror__subcmd__add_commands] )) ||
_syncweb__subcmd__mirror__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb mirror add commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__mirror__subcmd__help_commands] )) ||
_syncweb__subcmd__mirror__subcmd__help_commands() {
    local commands; commands=(
'add:Register a blob ticket as an alternate provider' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb mirror help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__mirror__subcmd__help__subcmd__add_commands] )) ||
_syncweb__subcmd__mirror__subcmd__help__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb mirror help add commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__mirror__subcmd__help__subcmd__help_commands] )) ||
_syncweb__subcmd__mirror__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb mirror help help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__moderation_commands] )) ||
_syncweb__subcmd__moderation_commands() {
    local commands; commands=(
'ls:List local moderation records' \
'hide:Hide a content record locally' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb moderation commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__moderation__subcmd__help_commands] )) ||
_syncweb__subcmd__moderation__subcmd__help_commands() {
    local commands; commands=(
'ls:List local moderation records' \
'hide:Hide a content record locally' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb moderation help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__moderation__subcmd__help__subcmd__help_commands] )) ||
_syncweb__subcmd__moderation__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb moderation help help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__moderation__subcmd__help__subcmd__hide_commands] )) ||
_syncweb__subcmd__moderation__subcmd__help__subcmd__hide_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb moderation help hide commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__moderation__subcmd__help__subcmd__ls_commands] )) ||
_syncweb__subcmd__moderation__subcmd__help__subcmd__ls_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb moderation help ls commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__moderation__subcmd__hide_commands] )) ||
_syncweb__subcmd__moderation__subcmd__hide_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb moderation hide commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__moderation__subcmd__ls_commands] )) ||
_syncweb__subcmd__moderation__subcmd__ls_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb moderation ls commands' commands "$@"
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
'export:Export one or more package directories as compressed CAR archive files' \
'import:Import and install a compressed CAR archive file' \
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
(( $+functions[_syncweb__subcmd__package__subcmd__export_commands] )) ||
_syncweb__subcmd__package__subcmd__export_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package export commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__help_commands] )) ||
_syncweb__subcmd__package__subcmd__help_commands() {
    local commands; commands=(
'export:Export one or more package directories as compressed CAR archive files' \
'import:Import and install a compressed CAR archive file' \
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
(( $+functions[_syncweb__subcmd__package__subcmd__help__subcmd__export_commands] )) ||
_syncweb__subcmd__package__subcmd__help__subcmd__export_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package help export commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__help__subcmd__help_commands] )) ||
_syncweb__subcmd__package__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package help help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__help__subcmd__import_commands] )) ||
_syncweb__subcmd__package__subcmd__help__subcmd__import_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package help import commands' commands "$@"
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
(( $+functions[_syncweb__subcmd__package__subcmd__import_commands] )) ||
_syncweb__subcmd__package__subcmd__import_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package import commands' commands "$@"
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
(( $+functions[_syncweb__subcmd__report_commands] )) ||
_syncweb__subcmd__report_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb report commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__schedule_commands] )) ||
_syncweb__subcmd__schedule_commands() {
    local commands; commands=(
'set:Update the global schedule' \
'folder:Set schedule overrides for a named folder' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb schedule commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__schedule__subcmd__folder_commands] )) ||
_syncweb__subcmd__schedule__subcmd__folder_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb schedule folder commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__schedule__subcmd__help_commands] )) ||
_syncweb__subcmd__schedule__subcmd__help_commands() {
    local commands; commands=(
'set:Update the global schedule' \
'folder:Set schedule overrides for a named folder' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb schedule help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__schedule__subcmd__help__subcmd__folder_commands] )) ||
_syncweb__subcmd__schedule__subcmd__help__subcmd__folder_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb schedule help folder commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__schedule__subcmd__help__subcmd__help_commands] )) ||
_syncweb__subcmd__schedule__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb schedule help help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__schedule__subcmd__help__subcmd__set_commands] )) ||
_syncweb__subcmd__schedule__subcmd__help__subcmd__set_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb schedule help set commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__schedule__subcmd__set_commands] )) ||
_syncweb__subcmd__schedule__subcmd__set_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb schedule set commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__shutdown_commands] )) ||
_syncweb__subcmd__shutdown_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb shutdown commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__snapshot_commands] )) ||
_syncweb__subcmd__snapshot_commands() {
    local commands; commands=(
'create:Create a content-addressed snapshot' \
'restore:Restore a snapshot to a folder or directory' \
'list:List local snapshots' \
'diff:Compare two snapshots' \
'delete:Delete a snapshot and release its pins' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb snapshot commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__snapshot__subcmd__create_commands] )) ||
_syncweb__subcmd__snapshot__subcmd__create_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb snapshot create commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__snapshot__subcmd__delete_commands] )) ||
_syncweb__subcmd__snapshot__subcmd__delete_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb snapshot delete commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__snapshot__subcmd__diff_commands] )) ||
_syncweb__subcmd__snapshot__subcmd__diff_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb snapshot diff commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__snapshot__subcmd__help_commands] )) ||
_syncweb__subcmd__snapshot__subcmd__help_commands() {
    local commands; commands=(
'create:Create a content-addressed snapshot' \
'restore:Restore a snapshot to a folder or directory' \
'list:List local snapshots' \
'diff:Compare two snapshots' \
'delete:Delete a snapshot and release its pins' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb snapshot help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__snapshot__subcmd__help__subcmd__create_commands] )) ||
_syncweb__subcmd__snapshot__subcmd__help__subcmd__create_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb snapshot help create commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__snapshot__subcmd__help__subcmd__delete_commands] )) ||
_syncweb__subcmd__snapshot__subcmd__help__subcmd__delete_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb snapshot help delete commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__snapshot__subcmd__help__subcmd__diff_commands] )) ||
_syncweb__subcmd__snapshot__subcmd__help__subcmd__diff_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb snapshot help diff commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__snapshot__subcmd__help__subcmd__help_commands] )) ||
_syncweb__subcmd__snapshot__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb snapshot help help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__snapshot__subcmd__help__subcmd__list_commands] )) ||
_syncweb__subcmd__snapshot__subcmd__help__subcmd__list_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb snapshot help list commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__snapshot__subcmd__help__subcmd__restore_commands] )) ||
_syncweb__subcmd__snapshot__subcmd__help__subcmd__restore_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb snapshot help restore commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__snapshot__subcmd__list_commands] )) ||
_syncweb__subcmd__snapshot__subcmd__list_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb snapshot list commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__snapshot__subcmd__restore_commands] )) ||
_syncweb__subcmd__snapshot__subcmd__restore_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb snapshot restore commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__sort_commands] )) ||
_syncweb__subcmd__sort_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb sort commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__start_commands] )) ||
_syncweb__subcmd__start_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb start commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__stat_commands] )) ||
_syncweb__subcmd__stat_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb stat commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__stats_commands] )) ||
_syncweb__subcmd__stats_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb stats commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__subscribe_commands] )) ||
_syncweb__subcmd__subscribe_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb subscribe commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__trust_commands] )) ||
_syncweb__subcmd__trust_commands() {
    local commands; commands=(
'show:Show trust and moderation state' \
'delegate:Delegate trust to a publisher identity' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb trust commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__trust__subcmd__delegate_commands] )) ||
_syncweb__subcmd__trust__subcmd__delegate_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb trust delegate commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__trust__subcmd__help_commands] )) ||
_syncweb__subcmd__trust__subcmd__help_commands() {
    local commands; commands=(
'show:Show trust and moderation state' \
'delegate:Delegate trust to a publisher identity' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb trust help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__trust__subcmd__help__subcmd__delegate_commands] )) ||
_syncweb__subcmd__trust__subcmd__help__subcmd__delegate_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb trust help delegate commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__trust__subcmd__help__subcmd__help_commands] )) ||
_syncweb__subcmd__trust__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb trust help help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__trust__subcmd__help__subcmd__show_commands] )) ||
_syncweb__subcmd__trust__subcmd__help__subcmd__show_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb trust help show commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__trust__subcmd__show_commands] )) ||
_syncweb__subcmd__trust__subcmd__show_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb trust show commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__unpublish_commands] )) ||
_syncweb__subcmd__unpublish_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb unpublish commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__verify_commands] )) ||
_syncweb__subcmd__verify_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb verify commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__version_commands] )) ||
_syncweb__subcmd__version_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb version commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__watch_commands] )) ||
_syncweb__subcmd__watch_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb watch commands' commands "$@"
}

if [ "$funcstack[1]" = "_syncweb" ]; then
    _syncweb "$@"
else
    compdef _syncweb syncweb
fi
