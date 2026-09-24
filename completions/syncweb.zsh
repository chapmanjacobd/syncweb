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
'--network=[Network name for scoped operations (uses data_dir/<network>/). Defaults to '\''default'\'' if absent.]:NETWORK:_default' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
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
            (start)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Override the global persistent data directory]:DATA_DIR:_files' \
'--log-file=[Write daemon logs to this file]:LOG_FILE:_files' \
'--max-threads=[]:MAX_THREADS:_default' \
'--sync-interval=[]:SYNC_INTERVAL:_default' \
'--beacon-port=[Base UDP port the beacon spreads network scopes over]:BEACON_PORT:_default' \
'--discovery-interface=[Restrict the beacon to a single network interface by name]:DISCOVERY_INTERFACE:_default' \
'--media-listen=[Media HTTP server listen address (e.g. 127.0.0.1\:9193)]:MEDIA_LISTEN:_default' \
'--bg[Run in the background (daemon mode)]' \
'--media-only[Run only the media HTTP server (standalone) and exit]' \
'--no-relay[Disable Iroh relay mode (no relay server connections)]' \
'--no-mdns[Disable mDNS local peer discovery]' \
'--no-beacon[Disable the UDP beacon local peer discovery]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(stop)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--force[Skip graceful shutdown]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(reload)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(sync)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::namespace -- Namespace of a live folder to sync now; omit it to sync every enabled folder:_default' \
&& ret=0
;;
(devices)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(folders)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__folders_commands" \
"*::: :->folders" \
&& ret=0

    case $state in
    (folders)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-folders-command-$line[1]:"
        case $line[1] in
            (create)
_arguments "${_arguments_options[@]}" : \
'--mode=[Sync mode\: sendreceive, receiveonly, or sendonly]:MODE:_default' \
'--network=[Add the created folder to a named network]:NETWORK:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--relay-fallback[Enable Syncthing relay fallback for this folder]' \
'--import[Scan and import existing files in the directory]' \
'(--import)--no-import[Skip scanning existing files in the directory]' \
'--write[Grant write access on the share ticket (default\: read-only)]' \
'--no-share[Create the folder without sharing it (no ticket/URL printed)]' \
'--no-indexing[Do not opt the folder into local indexing (indexing is enabled by default)]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(join)
_arguments "${_arguments_options[@]}" : \
'--mode=[]:MODE:_default' \
'--network=[Add the joined folder to a named network]:NETWORK:_default' \
'--prefix=[Parent directory prepended to the path argument]:PREFIX:_files' \
'(--glob)--sync-prefix=[Area prefix filter for subscription entries]:SYNC_PREFIX:_files' \
'(--sync-prefix)--glob=[]:GLOB:_default' \
'--max-count=[]:MAX_COUNT:_default' \
'--max-size=[]:MAX_SIZE:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--relay-fallback[Enable Syncthing relay fallback for this folder]' \
'--subscribe[Track + enable live syncing (persisted subscribe-changes); off by default, idempotent on an existing folder]' \
'--ingest-only[Only deliver entries ingested after live syncing is enabled]' \
'--ignore-self[Ignore events emitted by this device'\''s own writes]' \
'--download-existing[Download matching existing content to the local folder after joining (one-shot; uses the same prefix/glob/max filters). Off by default so a big folder can'\''t fill your disk by accident]' \
'--download[Download matching existing content to the local folder after joining (one-shot; uses the same prefix/glob/max filters). Off by default so a big folder can'\''t fill your disk by accident]' \
'--no-indexing[Do not opt the folder into local indexing (indexing is enabled by default)]' \
'--metadata-only[Track folder metadata without downloading content into the store; fetch content later with \`download\`]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':ticket -- Iroh document ticket for a new folder, or a folder selector for an already-tracked folder:_default' \
'::path:_files' \
&& ret=0
;;
(leave)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--delete-files[Also delete the folder'\''s local files]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':folder -- Namespace ID or path to a managed folder:_default' \
&& ret=0
;;
(import)
_arguments "${_arguments_options[@]}" : \
'--folder=[Folder namespace or managed folder path; defaults to the only managed folder]:FOLDER:_default' \
'--namespace=[Folder namespace or managed folder path; defaults to the only managed folder]:FOLDER:_default' \
'--threads=[Scanner threads (1 disables parallelism, 0 uses all available CPUs)]:THREADS:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--enrich[Query daemon for peer counts and frequency data to enrich niche/frecency/peers sorting]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':path:_files' \
&& ret=0
;;
        esac
    ;;
esac
;;
(ls)
_arguments "${_arguments_options[@]}" : \
'--sort=[Collect and sort output instead of streaming it]:SORT:_default' \
'--threads=[Scanner threads (1 disables parallelism, 0 uses all available CPUs)]:THREADS:_default' \
'--path-prefix=[Only entries whose path starts with this prefix]:PATH_PREFIX:_default' \
'--path-glob=[Only entries whose path matches this glob pattern]:PATH_GLOB:_default' \
'*-e+[File extensions to include (can repeat)]:EXT:_default' \
'*--ext=[File extensions to include (can repeat)]:EXT:_default' \
'*--size=[Size constraints\: N, -N, +N, N%10, +5GB, etc. (can repeat)]:SIZE:_default' \
'*--depth=[Depth constraints\: N, +N (min), -N (max) (can repeat)]:DEPTH:_default' \
'--min-depth=[Alternative min depth notation]:MIN_DEPTH:_default' \
'--max-depth=[Alternative max depth notation]:MAX_DEPTH:_default' \
'--type=[Filter by type\: f=file, d=dir, l=symlink]:FILE_TYPE:(f d l)' \
'*--modified-within=[Newer than\: '\''3 days'\'', '\''2 weeks'\'' (can repeat)]:MODIFIED_WITHIN:_default' \
'*--modified-before=[Older than\: '\''3 years'\'', '\''1 month'\'' (can repeat)]:MODIFIED_BEFORE:_default' \
'*--time-modified=[Time modified\: '\''-3 days'\'' (newer), '\''+3 days'\'' (older) (can repeat)]:TIME_MODIFIED:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'(--remote-only)--local-only[Scan the local disk instead of the metadata index (works on any path, even outside a Syncweb folder)]' \
'--no-enrich[Skip per-file disk metadata lookup (pure metadata listing)]' \
'--remote-only[Show only entries not yet downloaded (State == remote)]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
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
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':path:_files' \
&& ret=0
;;
(find)
_arguments "${_arguments_options[@]}" : \
'--kind=[]:KIND:(exact glob regex)' \
'--threads=[Scanner threads (1 disables parallelism, 0 uses all available CPUs)]:THREADS:_default' \
'--path-prefix=[Only entries whose path starts with this prefix]:PATH_PREFIX:_default' \
'--path-glob=[Only entries whose path matches this glob pattern]:PATH_GLOB:_default' \
'*-e+[File extensions to include (can repeat)]:EXT:_default' \
'*--ext=[File extensions to include (can repeat)]:EXT:_default' \
'*--size=[Size constraints\: N, -N, +N, N%10, +5GB, etc. (can repeat)]:SIZE:_default' \
'*--depth=[Depth constraints\: N, +N (min), -N (max) (can repeat)]:DEPTH:_default' \
'--min-depth=[Alternative min depth notation]:MIN_DEPTH:_default' \
'--max-depth=[Alternative max depth notation]:MAX_DEPTH:_default' \
'--type=[Filter by type\: f=file, d=dir, l=symlink]:FILE_TYPE:(f d l)' \
'*--modified-within=[Newer than\: '\''3 days'\'', '\''2 weeks'\'' (can repeat)]:MODIFIED_WITHIN:_default' \
'*--modified-before=[Older than\: '\''3 years'\'', '\''1 month'\'' (can repeat)]:MODIFIED_BEFORE:_default' \
'*--time-modified=[Time modified\: '\''-3 days'\'' (newer), '\''+3 days'\'' (older) (can repeat)]:TIME_MODIFIED:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'(-s --case-sensitive)-i[Case insensitive search]' \
'(-s --case-sensitive)--ignore-case[Case insensitive search]' \
'(-i --ignore-case)-s[Case sensitive search]' \
'(-i --ignore-case)--case-sensitive[Case sensitive search]' \
'-F[Treat patterns as literal strings]' \
'--fixed-strings[Treat patterns as literal strings]' \
'-p[Search full path (default\: filename only)]' \
'--full-path[Search full path (default\: filename only)]' \
'-H[Search hidden files and directories]' \
'--hidden[Search hidden files and directories]' \
'-L[Follow symbolic links]' \
'--follow-links[Follow symbolic links]' \
'-a[Print absolute paths]' \
'--absolute-path[Print absolute paths]' \
'-d[Exclude sendonly folders from search]' \
'--download[Exclude sendonly folders from search]' \
'(--remote-only)--local-only[Scan the local disk instead of the metadata index (works on any path, even outside a Syncweb folder)]' \
'--no-enrich[Skip per-file disk metadata lookup (pure metadata listing)]' \
'--remote-only[Show only entries not yet downloaded (State == remote)]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':pattern:_default' \
'::path:_files' \
&& ret=0
;;
(search)
_arguments "${_arguments_options[@]}" : \
'--kind=[Which backend to search\: all, catalog, package, or channel]:KIND:((all\:"Search the local catalog index (FTS) and, when requested, editorial channels and gossip"
catalog\:"Search only the local catalog index (FTS over subscribed catalog docs)"
package\:"Search installed packages and, when requested, the gossip package catalog"
channel\:"Search only an editorial channel"))' \
'--channel=[Restrict results to an editorial channel]:CHANNEL:_default' \
'--limit=[Maximum number of results]:LIMIT:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
'::query -- Search query; omit to list everything:_default' \
&& ret=0
;;
(sort)
_arguments "${_arguments_options[@]}" : \
'--by=[]:BY:(niche frecency peers random folder time date week month year size folder-size folder-avg-size folder-date folder-time count)' \
'--min-seeders=[Filter files with fewer than N seeders]:MIN_SEEDERS:_default' \
'--max-seeders=[Filter files with more than N seeders]:MAX_SEEDERS:_default' \
'--niche=[Ideal popularity (peer count) for niche scoring]:NICHE:_default' \
'--frecency-weight=[Divisor for recency weighting in frecency calculation]:FRECENCY_WEIGHT:_default' \
'--limit-size=[Quit after printing N bytes of files]:LIMIT_SIZE:_default' \
'--threads=[Scanner threads (1 disables parallelism, 0 uses all available CPUs)]:THREADS:_default' \
'--path-prefix=[Only entries whose path starts with this prefix]:PATH_PREFIX:_default' \
'--path-glob=[Only entries whose path matches this glob pattern]:PATH_GLOB:_default' \
'*-e+[File extensions to include (can repeat)]:EXT:_default' \
'*--ext=[File extensions to include (can repeat)]:EXT:_default' \
'*--size=[Size constraints\: N, -N, +N, N%10, +5GB, etc. (can repeat)]:SIZE:_default' \
'*--depth=[Depth constraints\: N, +N (min), -N (max) (can repeat)]:DEPTH:_default' \
'--min-depth=[Alternative min depth notation]:MIN_DEPTH:_default' \
'--max-depth=[Alternative max depth notation]:MAX_DEPTH:_default' \
'--type=[Filter by type\: f=file, d=dir, l=symlink]:FILE_TYPE:(f d l)' \
'*--modified-within=[Newer than\: '\''3 days'\'', '\''2 weeks'\'' (can repeat)]:MODIFIED_WITHIN:_default' \
'*--modified-before=[Older than\: '\''3 years'\'', '\''1 month'\'' (can repeat)]:MODIFIED_BEFORE:_default' \
'*--time-modified=[Time modified\: '\''-3 days'\'' (newer), '\''+3 days'\'' (older) (can repeat)]:TIME_MODIFIED:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--enrich[Query daemon for peer counts and frequency data to enrich niche/frecency/peers sorting]' \
'(--remote-only)--local-only[Scan the local disk instead of the metadata index (works on any path, even outside a Syncweb folder)]' \
'--no-enrich[Skip per-file disk metadata lookup (pure metadata listing)]' \
'--remote-only[Show only entries not yet downloaded (State == remote)]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(download)
_arguments "${_arguments_options[@]}" : \
'*--hash=[Content hash(es) to select (can repeat)]:HASH:_default' \
'--path-prefix=[Only entries whose path starts with this prefix]:PATH_PREFIX:_default' \
'--path-glob=[Only entries whose path matches this glob pattern]:PATH_GLOB:_default' \
'*-e+[File extensions to include (can repeat)]:EXT:_default' \
'*--ext=[File extensions to include (can repeat)]:EXT:_default' \
'*--size=[Size constraints\: N, -N, +N, N%10, +5GB, etc. (can repeat)]:SIZE:_default' \
'*--depth=[Depth constraints\: N, +N (min), -N (max) (can repeat)]:DEPTH:_default' \
'--min-depth=[Alternative min depth notation]:MIN_DEPTH:_default' \
'--max-depth=[Alternative max depth notation]:MAX_DEPTH:_default' \
'--type=[Filter by type\: f=file, d=dir, l=symlink]:FILE_TYPE:(f d l)' \
'*--modified-within=[Newer than\: '\''3 days'\'', '\''2 weeks'\'' (can repeat)]:MODIFIED_WITHIN:_default' \
'*--modified-before=[Older than\: '\''3 years'\'', '\''1 month'\'' (can repeat)]:MODIFIED_BEFORE:_default' \
'*--time-modified=[Time modified\: '\''-3 days'\'' (newer), '\''+3 days'\'' (older) (can repeat)]:TIME_MODIFIED:_default' \
'*--from=[Blob ticket(s) for providers (can repeat)]:FROM:_default' \
'*--provider=[Blob ticket(s) for providers (can repeat)]:FROM:_default' \
'--min-providers=[Minimum providers for healthy replication]:MIN_PROVIDERS:_default' \
'--max-peers=[Fetch only blobs with at most N observed peers]:MAX_PEERS:_default' \
'--min-peers=[Fetch only blobs with at least N observed peers]:MIN_PEERS:_default' \
'--min-count=[Minimum number of blobs to fetch]:MIN_COUNT:_default' \
'--max-count=[Maximum number of blobs to fetch]:MAX_COUNT:_default' \
'--threads=[Copy threads (1 disables parallelism, 0 uses all available CPUs)]:THREADS:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--remote-only[Show only entries not yet downloaded (State == remote)]' \
'--no-sharing[Do not share or seed downloaded content]' \
'--no-seeding[Do not share or seed downloaded content]' \
'--dry-run[Preview which folder entries would be fetched (paths, sizes, counts) without downloading anything]' \
'--preview[Preview which folder entries would be fetched (paths, sizes, counts) without downloading anything]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':source:_files' \
'::destination:_files' \
&& ret=0
;;
(verify)
_arguments "${_arguments_options[@]}" : \
'*--hash=[Content hash(es) to select (can repeat)]:HASH:_default' \
'--path-prefix=[Only entries whose path starts with this prefix]:PATH_PREFIX:_default' \
'--path-glob=[Only entries whose path matches this glob pattern]:PATH_GLOB:_default' \
'*-e+[File extensions to include (can repeat)]:EXT:_default' \
'*--ext=[File extensions to include (can repeat)]:EXT:_default' \
'*--size=[Size constraints\: N, -N, +N, N%10, +5GB, etc. (can repeat)]:SIZE:_default' \
'*--depth=[Depth constraints\: N, +N (min), -N (max) (can repeat)]:DEPTH:_default' \
'--min-depth=[Alternative min depth notation]:MIN_DEPTH:_default' \
'--max-depth=[Alternative max depth notation]:MAX_DEPTH:_default' \
'--type=[Filter by type\: f=file, d=dir, l=symlink]:FILE_TYPE:(f d l)' \
'*--modified-within=[Newer than\: '\''3 days'\'', '\''2 weeks'\'' (can repeat)]:MODIFIED_WITHIN:_default' \
'*--modified-before=[Older than\: '\''3 years'\'', '\''1 month'\'' (can repeat)]:MODIFIED_BEFORE:_default' \
'*--time-modified=[Time modified\: '\''-3 days'\'' (newer), '\''+3 days'\'' (older) (can repeat)]:TIME_MODIFIED:_default' \
'*--from=[Blob ticket(s) for providers (can repeat)]:FROM:_default' \
'*--provider=[Blob ticket(s) for providers (can repeat)]:FROM:_default' \
'--min-providers=[Minimum providers for healthy replication]:MIN_PROVIDERS:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--remote-only[Show only entries not yet downloaded (State == remote)]' \
'--fix[Attempt to repair corrupted blobs by re-downloading from peers]' \
'--no-sharing[Do not share or seed downloaded content]' \
'--no-seeding[Do not share or seed downloaded content]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(transfer)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__transfer_commands" \
"*::: :->transfer" \
&& ret=0

    case $state in
    (transfer)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-transfer-command-$line[1]:"
        case $line[1] in
            (info)
_arguments "${_arguments_options[@]}" : \
'--namespace=[Limit display to a namespace]:NAMESPACE:_default' \
'--state=[Limit display to a lifecycle state]:STATE:_default' \
'--sort=[]:SORT:(created updated size peers path)' \
'--group-by=[]:GROUP_BY:(namespace root state)' \
'--limit=[]:LIMIT:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(remaining)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(root)
_arguments "${_arguments_options[@]}" : \
'--min-free=[Free bytes to preserve on this root]:MIN_FREE:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--disabled[Disable this root for allocation]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':id:_default' \
':path:_files' \
&& ret=0
;;
(enqueue)
_arguments "${_arguments_options[@]}" : \
'--namespace=[]:NAMESPACE:_default' \
'--path=[Relative materialization path]:PATH:_files' \
'--hash=[32-byte blob hash in hexadecimal]:HASH:_default' \
'--source=[Local file to read content from; computes the blob hash and size automatically]:SOURCE:_files' \
'--size=[Blob size in bytes (ignored when --source is given)]:SIZE:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--now[Allocate and materialize the job immediately instead of leaving it queued]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(allocate)
_arguments "${_arguments_options[@]}" : \
'--namespace=[Limit allocation to a namespace]:NAMESPACE:_default' \
'--path-prefix=[Only allocate paths below this relative prefix]:PATH_PREFIX:_files' \
'--min-size=[]:MIN_SIZE:_default' \
'--max-size=[]:MAX_SIZE:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--dry-run[Report allocations without persisting them]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(materialize)
_arguments "${_arguments_options[@]}" : \
'--namespace=[Limit processing to a namespace]:NAMESPACE:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(pause)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':id:_default' \
&& ret=0
;;
(resume)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':id:_default' \
&& ret=0
;;
(cancel)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':id:_default' \
&& ret=0
;;
(retry)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':id:_default' \
&& ret=0
;;
        esac
    ;;
esac
;;
(share)
_arguments "${_arguments_options[@]}" : \
'--blob=[Share a single content hash as an unauthenticated blob ticket (blobs are immutable; always pinned, never persisted)]:BLOB:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--write[Grant write access (default\: read-only)]' \
'--no-pin[Skip pinning the shared folder'\''s blobs]' \
'--no-persist[Skip persisting the share record]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::path -- Folder path or namespace:_files' \
":: :_syncweb__subcmd__share_commands" \
"*::: :->share" \
&& ret=0

    case $state in
    (share)
        words=($line[2] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-share-command-$line[2]:"
        case $line[2] in
            (list)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::path -- Folder path or namespace to filter persisted shares:_files' \
&& ret=0
;;
(provider)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__share__subcmd__provider_commands" \
"*::: :->provider" \
&& ret=0

    case $state in
    (provider)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-share-provider-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':collection:_default' \
':provider:_default' \
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
(access)
_arguments "${_arguments_options[@]}" : \
'--blob=[Revoke a blob share by content hash (unpins and unannounces the blob) instead of a folder share; requires --revoke]:BLOB:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--revoke[Revoke a share instead of listing access (requires the positional path or --blob)]' \
'(--read)--write[Revoke the write share (default\: the read share)]' \
'(--write)--read[Revoke the read share (the default, prompt-free path)]' \
'--full[Show every shared-with row instead of capping the list]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::path -- Folder path or namespace (omit to show every folder):_files' \
&& ret=0
;;
(link)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
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
'--publish=[Namespace (folder) to publish the link into]:PUBLISH:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'(--name)--private[]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':source:_files' \
&& ret=0
;;
(resolve)
_arguments "${_arguments_options[@]}" : \
'--version=[]:VERSION:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--no-fetch[Print the resolution without fetching or pinning the resolved content]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':link:_default' \
&& ret=0
;;
(revoke)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':link:_default' \
&& ret=0
;;
        esac
    ;;
esac
;;
(package)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
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
            (add)
_arguments "${_arguments_options[@]}" : \
'--version=[]:VERSION:_default' \
'--name=[]:NAME:_default' \
'--root=[Override the common root for logical path rebasing]:PATH:_files' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'*::paths:_files' \
&& ret=0
;;
(bump)
_arguments "${_arguments_options[@]}" : \
'--version=[]:VERSION:_default' \
'--changelog=[]:CHANGELOG:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(publish)
_arguments "${_arguments_options[@]}" : \
'--namespace=[Folder namespace, managed folder path, or omitted to default to the only managed folder]:NAMESPACE:_default' \
'--sequence=[]:SEQUENCE:_default' \
'*--bootstrap=[]:NODE_ID:_default' \
'--root=[Override the common root for logical path rebasing]:PATH:_files' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'*::paths:_files' \
&& ret=0
;;
(export)
_arguments "${_arguments_options[@]}" : \
'--version=[]:VERSION:_default' \
'*--filter=[]:EXPRESSION:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
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
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'*::archives:_files' \
&& ret=0
;;
(info)
_arguments "${_arguments_options[@]}" : \
'--hash=[Blob hash of the manifest (requires --node-id)]:HASH:_default' \
'--node-id=[Node ID hosting the manifest blob]:NODE_ID:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::ticket:_default' \
&& ret=0
;;
(install)
_arguments "${_arguments_options[@]}" : \
'--path=[]:PATH:_files' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':ticket:_default' \
&& ret=0
;;
(upgrade)
_arguments "${_arguments_options[@]}" : \
'--path=[]:PATH:_files' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':ticket:_default' \
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':collection:_default' \
':version:_default' \
&& ret=0
;;
(verify)
_arguments "${_arguments_options[@]}" : \
'--version=[]:VERSION:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':collection:_default' \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(versions)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':collection:_default' \
&& ret=0
;;
(switch)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':collection:_default' \
':version:_default' \
&& ret=0
;;
        esac
    ;;
esac
;;
(network)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
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
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
&& ret=0
;;
(join)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':ticket:_default' \
&& ret=0
;;
(leave)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::name -- Optional network name to inspect:_default' \
&& ret=0
;;
(invite)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
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
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
':device:_default' \
&& ret=0
;;
(events)
_arguments "${_arguments_options[@]}" : \
'--limit=[]:LIMIT:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':network_id:_default' \
&& ret=0
;;
(test-relay)
_arguments "${_arguments_options[@]}" : \
'--relay-url=[]:RELAY_URL:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::name -- Optional network name or ID to inspect:_default' \
&& ret=0
;;
(peers)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::folder -- Optional folder path or namespace to inspect:_default' \
&& ret=0
;;
        esac
    ;;
esac
;;
(watch)
_arguments "${_arguments_options[@]}" : \
'--debounce-ms=[Debounce changes in milliseconds]:DEBOUNCE_MS:_default' \
'*--exclude=[Ignore a path glob; may be repeated]:GLOB:_default' \
'*--paths=[Paths evaluated by --dry-run]:PATHS:_files' \
'--filters=[Filter configuration (defaults to DATA_DIR/filters.toml)]:FILTERS:_files' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--once[Process one event and exit]' \
'--show-filters[Print the active filter configuration and exit]' \
'--dry-run[Evaluate paths against the filter rules without importing]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(snapshot)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
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
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(restore)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
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
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(diff)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
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
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':path:_files' \
':snapshot:_default' \
&& ret=0
;;
        esac
    ;;
esac
;;
(indexing)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
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
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':folder:_files' \
&& ret=0
;;
(disable)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':folder:_files' \
&& ret=0
;;
(filter)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
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
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
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
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':source:_default' \
&& ret=0
;;
        esac
    ;;
esac
;;
(publish)
_arguments "${_arguments_options[@]}" : \
'--catalog=[]:CATALOG:_default' \
'*--tag=[]:TAGS:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':folder:_files' \
&& ret=0
;;
        esac
    ;;
esac
;;
(stats)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__stats_commands" \
"*::: :->stats" \
&& ret=0

    case $state in
    (stats)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-stats-command-$line[1]:"
        case $line[1] in
            (network)
_arguments "${_arguments_options[@]}" : \
'--folder=[Limit display to a folder or namespace]:FOLDER:_files' \
'--peer=[Limit display to a peer node ID]:PEER:_default' \
'--period=[Only include transfer events recorded in the last N (e.g. 24h, 7d)]:PERIOD:_default' \
'--since=[Only include transfer events recorded since a unix timestamp or period (e.g. 24h, 7d)]:SINCE:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--reset[Reset persisted counters before displaying them]' \
'--follow[Stream sync progress and network events live (one JSON object per line under --json)]' \
'--watch[Stream sync progress and network events live (one JSON object per line under --json)]' \
'--once[With --follow\: print the current snapshot and exit instead of streaming]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(files)
_arguments "${_arguments_options[@]}" : \
'--by=[]:BY:(extension size all time)' \
'--top-largest=[Top N largest files by size]:TOP_LARGEST:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::path -- Namespace ID or path to a managed folder:_files' \
&& ret=0
;;
        esac
    ;;
esac
;;
(db)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__db_commands" \
"*::: :->db" \
&& ret=0

    case $state in
    (db)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-db-command-$line[1]:"
        case $line[1] in
            (check)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(vacuum)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(stats)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(backup)
_arguments "${_arguments_options[@]}" : \
'--output=[]:OUTPUT:_files' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--include-blobs[Also copy the blob store (can be very large); off by default]' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
        esac
    ;;
esac
;;
(config)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
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
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
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
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::section:_default' \
&& ret=0
;;
(schedule)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_syncweb__subcmd__config__subcmd__schedule_commands" \
"*::: :->schedule" \
&& ret=0

    case $state in
    (schedule)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:syncweb-config-schedule-command-$line[1]:"
        case $line[1] in
            (set)
_arguments "${_arguments_options[@]}" : \
'--active=[]:ACTIVE:_default' \
'--bandwidth=[Bandwidth rate (e.g. '\''500K'\'', '\''2M'\'')]:BANDWIDTH:_default' \
'--period=[Time window for the bandwidth limit (e.g. '\''08\:00-18\:00'\'')]:PERIOD:_default' \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
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
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
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
(version)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(completions)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
':shell:(bash elvish fish powershell zsh)' \
&& ret=0
;;
(manpages)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::dir:_files' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
'--data-dir=[Directory used for persistent node identity and data]:DATA_DIR:_files' \
'--verbose[Enable verbose structured logging]' \
'--json[Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)]' \
'--yes[Assume yes to every destructive-operation prompt]' \
'--no-daemon[Bypass the daemon and use an embedded node for supported commands]' \
'--embedded[Bypass the daemon and use an embedded node for supported commands]' \
'-h[Print help]' \
'--help[Print help]' \
'::command:_default' \
&& ret=0
;;
        esac
    ;;
esac
}

(( $+functions[_syncweb_commands] )) ||
_syncweb_commands() {
    local commands; commands=(
'start:Start the local syncweb daemon' \
'stop:Stop the local syncweb daemon' \
'status:Show local daemon status' \
'reload:Ask the local daemon to reload configuration' \
'sync:Ask the local daemon to trigger synchronization' \
'devices:Show this device'\''s Iroh and Syncthing identities' \
'folders:Manage synchronized folders (bare\: list managed folders)' \
'ls:List files in a local folder' \
'stat:Show detailed metadata for a local file' \
'find:Search local files' \
'search:Search catalog content, packages, and editorial channels' \
'sort:Sort local files by discovery criteria' \
'download:Download folder content or copy a local file' \
'verify:Re-check local folder blob integrity' \
'transfer:Inspect and control durable transfer jobs' \
'share:Share a folder, printing a ticket (read-only by default, --write for write access)' \
'access:Show who can read/write each folder in one table, and revoke access in place (--revoke)' \
'link:Create and resolve stable syncweb links' \
'package:Create, version, publish, and manage collection packages' \
'network:Manage networks and membership' \
'watch:Watch a folder and import filesystem changes' \
'snapshot:Manage content-addressed snapshots' \
'indexing:Manage opt-in indexing, catalogs, and metadata' \
'stats:Show statistics for folders and files' \
'db:Database maintenance\: check, vacuum, stats, backup' \
'config:Show or update local configuration' \
'version:Show syncweb version information' \
'completions:Generate shell completions' \
'manpages:Generate manpages' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__access_commands] )) ||
_syncweb__subcmd__access_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb access commands' commands "$@"
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
'schedule:Show or update synchronization schedules' \
    )
    _describe -t commands 'syncweb config commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__config__subcmd__schedule_commands] )) ||
_syncweb__subcmd__config__subcmd__schedule_commands() {
    local commands; commands=(
'set:Update the global schedule' \
'folder:Set schedule overrides for a named folder' \
    )
    _describe -t commands 'syncweb config schedule commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__config__subcmd__schedule__subcmd__folder_commands] )) ||
_syncweb__subcmd__config__subcmd__schedule__subcmd__folder_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb config schedule folder commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__config__subcmd__schedule__subcmd__set_commands] )) ||
_syncweb__subcmd__config__subcmd__schedule__subcmd__set_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb config schedule set commands' commands "$@"
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
(( $+functions[_syncweb__subcmd__db_commands] )) ||
_syncweb__subcmd__db_commands() {
    local commands; commands=(
'check:Run integrity check on all databases' \
'vacuum:Run VACUUM to reclaim space in all databases' \
'stats:Show database sizes and table statistics' \
'backup:Back up databases, config, and identity to a directory (blobs opt-in with --include-blobs)' \
    )
    _describe -t commands 'syncweb db commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__db__subcmd__backup_commands] )) ||
_syncweb__subcmd__db__subcmd__backup_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb db backup commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__db__subcmd__check_commands] )) ||
_syncweb__subcmd__db__subcmd__check_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb db check commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__db__subcmd__stats_commands] )) ||
_syncweb__subcmd__db__subcmd__stats_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb db stats commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__db__subcmd__vacuum_commands] )) ||
_syncweb__subcmd__db__subcmd__vacuum_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb db vacuum commands' commands "$@"
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
(( $+functions[_syncweb__subcmd__find_commands] )) ||
_syncweb__subcmd__find_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb find commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__folders_commands] )) ||
_syncweb__subcmd__folders_commands() {
    local commands; commands=(
'create:Create a synchronized folder and print a read-only join ticket/URL (--write for write access, --no-share to skip)' \
'join:Join a folder from an Iroh document ticket' \
'leave:Leave a synchronized folder, optionally deleting its local files' \
'import:Import local files into a synchronized folder' \
    )
    _describe -t commands 'syncweb folders commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__folders__subcmd__create_commands] )) ||
_syncweb__subcmd__folders__subcmd__create_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb folders create commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__folders__subcmd__import_commands] )) ||
_syncweb__subcmd__folders__subcmd__import_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb folders import commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__folders__subcmd__join_commands] )) ||
_syncweb__subcmd__folders__subcmd__join_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb folders join commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__folders__subcmd__leave_commands] )) ||
_syncweb__subcmd__folders__subcmd__leave_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb folders leave commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help_commands] )) ||
_syncweb__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing_commands] )) ||
_syncweb__subcmd__indexing_commands() {
    local commands; commands=(
'enable:Opt a synchronized folder into indexing' \
'disable:Remove a folder from the local index' \
'filter:Manage local and federated denylists' \
'publish:Publish folder metadata to a catalog' \
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
    )
    _describe -t commands 'syncweb indexing filter commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__filter__subcmd__add_commands] )) ||
_syncweb__subcmd__indexing__subcmd__filter__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing filter add commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__filter__subcmd__subscribe_commands] )) ||
_syncweb__subcmd__indexing__subcmd__filter__subcmd__subscribe_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing filter subscribe commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__indexing__subcmd__publish_commands] )) ||
_syncweb__subcmd__indexing__subcmd__publish_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb indexing publish commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__link_commands] )) ||
_syncweb__subcmd__link_commands() {
    local commands; commands=(
'create:Create an immutable, private, or mutable link' \
'resolve:Resolve a stable link' \
'revoke:Revoke a private capability link' \
    )
    _describe -t commands 'syncweb link commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__link__subcmd__create_commands] )) ||
_syncweb__subcmd__link__subcmd__create_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb link create commands' commands "$@"
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
(( $+functions[_syncweb__subcmd__network_commands] )) ||
_syncweb__subcmd__network_commands() {
    local commands; commands=(
'create:Create a named network' \
'join:Join a network from an invitation' \
'leave:Leave a network' \
'list:List networks, optionally limited to a single network by name' \
'invite:Generate a network invitation' \
'kick:Remove a device from a network' \
'events:Show recent network events' \
'test-relay:Test a Syncthing relay TCP connection' \
'status:Show network membership and health, optionally limited to a single network by name' \
'peers:Show peer availability for a folder\: which peers joined and how seeded its blobs are' \
    )
    _describe -t commands 'syncweb network commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__create_commands] )) ||
_syncweb__subcmd__network__subcmd__create_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network create commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__events_commands] )) ||
_syncweb__subcmd__network__subcmd__events_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network events commands' commands "$@"
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
(( $+functions[_syncweb__subcmd__network__subcmd__list_commands] )) ||
_syncweb__subcmd__network__subcmd__list_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network list commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__peers_commands] )) ||
_syncweb__subcmd__network__subcmd__peers_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network peers commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__status_commands] )) ||
_syncweb__subcmd__network__subcmd__status_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network status commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__test-relay_commands] )) ||
_syncweb__subcmd__network__subcmd__test-relay_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network test-relay commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package_commands] )) ||
_syncweb__subcmd__package_commands() {
    local commands; commands=(
'add:Scan one or more paths into a package manifest (creates it if missing)' \
'bump:Create a new package manifest version' \
'publish:Publish a package manifest ticket and announce it to the catalog' \
'export:Export one or more package directories as compressed CAR archive files' \
'import:Import and install a compressed CAR archive file' \
'info:Show a collection manifest from a ticket or blob hash' \
'install:Verify, stage, and atomically install a collection version' \
'upgrade:Install a newer collection manifest version via ticket' \
'remove:Remove a non-current installed collection version' \
'verify:Verify an installed collection version' \
'list:List locally installed collections' \
'versions:List installed versions for a collection' \
'switch:Switch the active installed collection version' \
    )
    _describe -t commands 'syncweb package commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__add_commands] )) ||
_syncweb__subcmd__package__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package add commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__bump_commands] )) ||
_syncweb__subcmd__package__subcmd__bump_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package bump commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__export_commands] )) ||
_syncweb__subcmd__package__subcmd__export_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package export commands' commands "$@"
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
(( $+functions[_syncweb__subcmd__package__subcmd__publish_commands] )) ||
_syncweb__subcmd__package__subcmd__publish_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package publish commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__package__subcmd__remove_commands] )) ||
_syncweb__subcmd__package__subcmd__remove_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb package remove commands' commands "$@"
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
(( $+functions[_syncweb__subcmd__reload_commands] )) ||
_syncweb__subcmd__reload_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb reload commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__search_commands] )) ||
_syncweb__subcmd__search_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb search commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__share_commands] )) ||
_syncweb__subcmd__share_commands() {
    local commands; commands=(
'list:List persisted shares, optionally filtered by path' \
'provider:Manage blob provider registrations' \
    )
    _describe -t commands 'syncweb share commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__share__subcmd__list_commands] )) ||
_syncweb__subcmd__share__subcmd__list_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb share list commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__share__subcmd__provider_commands] )) ||
_syncweb__subcmd__share__subcmd__provider_commands() {
    local commands; commands=(
'add:Register a blob ticket as an alternate provider' \
    )
    _describe -t commands 'syncweb share provider commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__share__subcmd__provider__subcmd__add_commands] )) ||
_syncweb__subcmd__share__subcmd__provider__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb share provider add commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__snapshot_commands] )) ||
_syncweb__subcmd__snapshot_commands() {
    local commands; commands=(
'create:Create a content-addressed snapshot' \
'restore:Restore a snapshot to a folder or directory' \
'list:List local snapshots' \
'diff:Compare two snapshots' \
'delete:Delete a snapshot and release its pins' \
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
    local commands; commands=(
'network:Show persisted bandwidth accounting' \
'files:Show file-level statistics for synced folder content' \
    )
    _describe -t commands 'syncweb stats commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__stats__subcmd__files_commands] )) ||
_syncweb__subcmd__stats__subcmd__files_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb stats files commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__stats__subcmd__network_commands] )) ||
_syncweb__subcmd__stats__subcmd__network_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb stats network commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__status_commands] )) ||
_syncweb__subcmd__status_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb status commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__stop_commands] )) ||
_syncweb__subcmd__stop_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb stop commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__sync_commands] )) ||
_syncweb__subcmd__sync_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb sync commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__transfer_commands] )) ||
_syncweb__subcmd__transfer_commands() {
    local commands; commands=(
'info:List durable transfer jobs' \
'remaining:Show configured roots and remaining capacity' \
'root:Add or update a materialization root' \
'enqueue:Enqueue an individually addressable file job' \
'allocate:Allocate queued jobs to configured roots' \
'materialize:Fetch and materialize assigned jobs through the daemon' \
'pause:Pause a transfer job' \
'resume:Resume a paused transfer job' \
'cancel:Cancel a transfer job' \
'retry:Retry a failed transfer job' \
    )
    _describe -t commands 'syncweb transfer commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__transfer__subcmd__allocate_commands] )) ||
_syncweb__subcmd__transfer__subcmd__allocate_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb transfer allocate commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__transfer__subcmd__cancel_commands] )) ||
_syncweb__subcmd__transfer__subcmd__cancel_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb transfer cancel commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__transfer__subcmd__enqueue_commands] )) ||
_syncweb__subcmd__transfer__subcmd__enqueue_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb transfer enqueue commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__transfer__subcmd__info_commands] )) ||
_syncweb__subcmd__transfer__subcmd__info_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb transfer info commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__transfer__subcmd__materialize_commands] )) ||
_syncweb__subcmd__transfer__subcmd__materialize_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb transfer materialize commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__transfer__subcmd__pause_commands] )) ||
_syncweb__subcmd__transfer__subcmd__pause_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb transfer pause commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__transfer__subcmd__remaining_commands] )) ||
_syncweb__subcmd__transfer__subcmd__remaining_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb transfer remaining commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__transfer__subcmd__resume_commands] )) ||
_syncweb__subcmd__transfer__subcmd__resume_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb transfer resume commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__transfer__subcmd__retry_commands] )) ||
_syncweb__subcmd__transfer__subcmd__retry_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb transfer retry commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__transfer__subcmd__root_commands] )) ||
_syncweb__subcmd__transfer__subcmd__root_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb transfer root commands' commands "$@"
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
