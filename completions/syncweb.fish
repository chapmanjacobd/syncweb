# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_syncweb_global_optspecs
    string join \n verbose json no-color data-dir= h/help
end

function __fish_syncweb_needs_command
    # Figure out if the current invocation already has a command.
    set -l cmd (commandline -opc)
    set -e cmd[1]
    argparse -s (__fish_syncweb_global_optspecs) -- $cmd 2>/dev/null
    or return
    if set -q argv[1]
        # Also print the command, so this can be used to figure out what it is.
        echo $argv[1]
        return 1
    end
    return 0
end

function __fish_syncweb_using_subcommand
    set -l cmd (__fish_syncweb_needs_command)
    test -z "$cmd"
    and return 1
    contains -- $cmd[1] $argv
end

complete -c syncweb -n "__fish_syncweb_needs_command" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_needs_command" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_needs_command" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_needs_command" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_needs_command" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "version" -d 'Show syncweb version information'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "repl" -d 'Start an interactive command shell'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "start" -d 'Start the local syncweb node for one command invocation'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "shutdown" -d 'Stop the local syncweb node'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "create" -d 'Create a synchronized folder'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "join" -d 'Join a folder from an Iroh document ticket'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "accept" -d 'Accept a locally available folder'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "drop" -d 'Remove a local folder replica'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "folders" -d 'List managed folders'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "devices" -d 'Show this device\'s Iroh and Syncthing identities'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "config" -d 'Show or update local configuration'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "ls" -d 'List files in a local folder'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "find" -d 'Search local files'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "sort" -d 'Sort local files by discovery criteria'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "stat" -d 'Show detailed metadata for a local file'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "download" -d 'Download folder content or copy a local file'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "import" -d 'Import local files into a synchronized folder'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "backup" -d 'Create a content-addressed snapshot'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "restore" -d 'Restore a snapshot to a folder or directory'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "snapshots" -d 'List, diff, or delete snapshots'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "health" -d 'Show seeding status per folder blob'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "init" -d 'Initialize a folder and print a shareable URL'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "automatic" -d 'Run rules-based automatic synchronization'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "watch" -d 'Watch a folder and import filesystem changes'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "stats" -d 'Show persisted bandwidth accounting'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "verify" -d 'Re-check local folder blob integrity'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "schedule" -d 'Show or update synchronization schedules'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "subscribe" -d 'Subscribe to a folder with event filters'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "publish" -d 'Publish a folder or blob for public read access'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "unpublish" -d 'Remove a public blob pin'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "collection" -d 'Create and publish versioned content collections'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "package" -d 'Manage locally installed collection packages'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "network" -d 'Network connectivity utilities'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "completions" -d 'Generate shell completions'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "manpages" -d 'Generate manpages'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand version" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand version" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand version" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand version" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand version" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand repl" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand repl" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand repl" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand repl" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand repl" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand start" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand start" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand start" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand start" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand start" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand shutdown" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand shutdown" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand shutdown" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand shutdown" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand shutdown" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -l mode -r
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -l network -d 'Add the created folder to a named network' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -l relay-fallback -d 'Enable Syncthing relay fallback for this folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand join" -l mode -r
complete -c syncweb -n "__fish_syncweb_using_subcommand join" -l network -d 'Add the joined folder to a named network' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand join" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand join" -l relay-fallback -d 'Enable Syncthing relay fallback for this folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand join" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand join" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand join" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand join" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand accept" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand accept" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand accept" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand accept" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand accept" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand drop" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand drop" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand drop" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand drop" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand drop" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand folders" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand folders" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand folders" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand folders" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand folders" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand devices" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand devices" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand devices" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand devices" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand devices" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and not __fish_seen_subcommand_from set show help" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and not __fish_seen_subcommand_from set show help" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and not __fish_seen_subcommand_from set show help" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and not __fish_seen_subcommand_from set show help" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and not __fish_seen_subcommand_from set show help" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and not __fish_seen_subcommand_from set show help" -f -a "set" -d 'Set a configuration value'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and not __fish_seen_subcommand_from set show help" -f -a "show" -d 'Show configuration, optionally limited to a section'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and not __fish_seen_subcommand_from set show help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from set" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from set" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from set" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from set" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from set" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from show" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from show" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from show" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from show" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from show" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "set" -d 'Set a configuration value'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "show" -d 'Show configuration, optionally limited to a section'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand ls" -l sort -d 'Collect and sort output instead of streaming it' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand ls" -l threads -d 'Scanner threads (1 disables parallelism, 0 uses all available CPUs)' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand ls" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand ls" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand ls" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand ls" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand ls" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand find" -l kind -r -f -a "exact\t''
glob\t''
regex\t''"
complete -c syncweb -n "__fish_syncweb_using_subcommand find" -l max-depth -r
complete -c syncweb -n "__fish_syncweb_using_subcommand find" -l min-size -r
complete -c syncweb -n "__fish_syncweb_using_subcommand find" -l max-size -r
complete -c syncweb -n "__fish_syncweb_using_subcommand find" -l extension -r
complete -c syncweb -n "__fish_syncweb_using_subcommand find" -l type -r -f -a "f\t''
d\t''
l\t''"
complete -c syncweb -n "__fish_syncweb_using_subcommand find" -l threads -d 'Scanner threads (1 disables parallelism, 0 uses all available CPUs)' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand find" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand find" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand find" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand find" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand find" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand sort" -l by -r -f -a "niche\t''
frecency\t''
peers\t''
random\t''
folder\t''"
complete -c syncweb -n "__fish_syncweb_using_subcommand sort" -l threads -d 'Scanner threads (1 disables parallelism, 0 uses all available CPUs)' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand sort" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand sort" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand sort" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand sort" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand sort" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand stat" -l format -r
complete -c syncweb -n "__fish_syncweb_using_subcommand stat" -l threads -d 'Scanner threads (1 disables parallelism, 0 uses all available CPUs)' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand stat" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand stat" -l terse
complete -c syncweb -n "__fish_syncweb_using_subcommand stat" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand stat" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand stat" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand stat" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand download" -l max-peers -d 'Fetch only blobs with at most N observed peers' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand download" -l min-peers -d 'Fetch only blobs with at least N observed peers' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand download" -l min-count -r
complete -c syncweb -n "__fish_syncweb_using_subcommand download" -l max-count -r
complete -c syncweb -n "__fish_syncweb_using_subcommand download" -l threads -d 'Copy threads (1 disables parallelism, 0 uses all available CPUs)' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand download" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand download" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand download" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand download" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand download" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand import" -l folder -d 'Folder namespace; defaults to the only managed folder' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand import" -l threads -d 'Import threads (1 disables parallelism, 0 uses all available CPUs)' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand import" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand import" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand import" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand import" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand import" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand backup" -l description -r
complete -c syncweb -n "__fish_syncweb_using_subcommand backup" -l threads -d 'Scanner threads (1 disables parallelism, 0 uses all available CPUs)' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand backup" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand backup" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand backup" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand backup" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand backup" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand restore" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand restore" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand restore" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand restore" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand restore" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and not __fish_seen_subcommand_from diff delete help" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and not __fish_seen_subcommand_from diff delete help" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and not __fish_seen_subcommand_from diff delete help" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and not __fish_seen_subcommand_from diff delete help" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and not __fish_seen_subcommand_from diff delete help" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and not __fish_seen_subcommand_from diff delete help" -a "diff" -d 'Compare two snapshots'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and not __fish_seen_subcommand_from diff delete help" -a "delete" -d 'Delete a snapshot and release its pins'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and not __fish_seen_subcommand_from diff delete help" -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and __fish_seen_subcommand_from diff" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and __fish_seen_subcommand_from diff" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and __fish_seen_subcommand_from diff" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and __fish_seen_subcommand_from diff" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and __fish_seen_subcommand_from diff" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and __fish_seen_subcommand_from delete" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and __fish_seen_subcommand_from delete" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and __fish_seen_subcommand_from delete" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and __fish_seen_subcommand_from delete" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and __fish_seen_subcommand_from delete" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and __fish_seen_subcommand_from help" -f -a "diff" -d 'Compare two snapshots'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and __fish_seen_subcommand_from help" -f -a "delete" -d 'Delete a snapshot and release its pins'
complete -c syncweb -n "__fish_syncweb_using_subcommand snapshots; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand health" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand health" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand health" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand health" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand health" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand init" -l mode -r
complete -c syncweb -n "__fish_syncweb_using_subcommand init" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand init" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand init" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand init" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand init" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand automatic" -l paths -d 'Paths evaluated by --dry-run' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand automatic" -l filters -d 'Filter configuration (defaults to DATA_DIR/filters.toml)' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand automatic" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand automatic" -l show-filters -d 'Print the active filter configuration and exit'
complete -c syncweb -n "__fish_syncweb_using_subcommand automatic" -l dry-run -d 'Evaluate paths without starting the daemon'
complete -c syncweb -n "__fish_syncweb_using_subcommand automatic" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand automatic" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand automatic" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand automatic" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand watch" -l debounce-ms -d 'Debounce changes in milliseconds' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand watch" -l exclude -d 'Ignore a path glob; may be repeated' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand watch" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand watch" -l once -d 'Process one event and exit'
complete -c syncweb -n "__fish_syncweb_using_subcommand watch" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand watch" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand watch" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand watch" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand stats" -l folder -d 'Limit display to a folder or namespace' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand stats" -l peer -d 'Limit display to a peer node ID' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand stats" -l period -d 'Retained for compatibility; counters are persisted since period start' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand stats" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand stats" -l reset -d 'Reset persisted counters before displaying them'
complete -c syncweb -n "__fish_syncweb_using_subcommand stats" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand stats" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand stats" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand stats" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand verify" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand verify" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand verify" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand verify" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand verify" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and not __fish_seen_subcommand_from set folder help" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and not __fish_seen_subcommand_from set folder help" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and not __fish_seen_subcommand_from set folder help" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and not __fish_seen_subcommand_from set folder help" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and not __fish_seen_subcommand_from set folder help" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and not __fish_seen_subcommand_from set folder help" -f -a "set" -d 'Update the global schedule'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and not __fish_seen_subcommand_from set folder help" -f -a "folder" -d 'Set schedule overrides for a named folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and not __fish_seen_subcommand_from set folder help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from set" -l active -r
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from set" -l bandwidth -r
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from set" -l period -r
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from set" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from set" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from set" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from set" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from set" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from folder" -l active -r
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from folder" -l max-upload -r
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from folder" -l max-download -r
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from folder" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from folder" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from folder" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from folder" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from folder" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from help" -f -a "set" -d 'Update the global schedule'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from help" -f -a "folder" -d 'Set schedule overrides for a named folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand schedule; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l prefix -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l glob -r
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l max-count -r
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l max-size -r
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l ingest-only -d 'Only deliver entries ingested after subscription'
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l ignore-self -d 'Ignore events emitted by this subscription session'
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand publish" -l blob -d 'Publish this content hash as an unauthenticated blob ticket' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand publish" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand publish" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand publish" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand publish" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand publish" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand unpublish" -l blob -r
complete -c syncweb -n "__fish_syncweb_using_subcommand unpublish" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand unpublish" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand unpublish" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand unpublish" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand unpublish" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and not __fish_seen_subcommand_from init add versions publish help" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and not __fish_seen_subcommand_from init add versions publish help" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and not __fish_seen_subcommand_from init add versions publish help" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and not __fish_seen_subcommand_from init add versions publish help" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and not __fish_seen_subcommand_from init add versions publish help" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and not __fish_seen_subcommand_from init add versions publish help" -f -a "init" -d 'Initialize a directory as a versioned collection'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and not __fish_seen_subcommand_from init add versions publish help" -f -a "add" -d 'Scan files and update the local collection manifest'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and not __fish_seen_subcommand_from init add versions publish help" -f -a "versions" -d 'Create a new collection manifest version'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and not __fish_seen_subcommand_from init add versions publish help" -f -a "publish" -d 'Store a collection manifest and mutable head in a folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and not __fish_seen_subcommand_from init add versions publish help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from init" -l version -r
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from init" -l name -r
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from init" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from init" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from init" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from init" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from init" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from add" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from add" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from add" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from add" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from add" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from versions" -l version -r
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from versions" -l changelog -r
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from versions" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from versions" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from versions" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from versions" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from versions" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from publish" -l namespace -r
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from publish" -l sequence -r
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from publish" -l bootstrap -r
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from publish" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from publish" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from publish" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from publish" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from publish" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from help" -f -a "init" -d 'Initialize a directory as a versioned collection'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from help" -f -a "add" -d 'Scan files and update the local collection manifest'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from help" -f -a "versions" -d 'Create a new collection manifest version'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from help" -f -a "publish" -d 'Store a collection manifest and mutable head in a folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand collection; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and not __fish_seen_subcommand_from drop search info install upgrade remove verify list versions switch help" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and not __fish_seen_subcommand_from drop search info install upgrade remove verify list versions switch help" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and not __fish_seen_subcommand_from drop search info install upgrade remove verify list versions switch help" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and not __fish_seen_subcommand_from drop search info install upgrade remove verify list versions switch help" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and not __fish_seen_subcommand_from drop search info install upgrade remove verify list versions switch help" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and not __fish_seen_subcommand_from drop search info install upgrade remove verify list versions switch help" -f -a "drop" -d 'Export package versions as compressed CAR drop files'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and not __fish_seen_subcommand_from drop search info install upgrade remove verify list versions switch help" -f -a "search" -d 'List locally installed packages, optionally filtering by text'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and not __fish_seen_subcommand_from drop search info install upgrade remove verify list versions switch help" -f -a "info" -d 'Show a collection manifest'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and not __fish_seen_subcommand_from drop search info install upgrade remove verify list versions switch help" -f -a "install" -d 'Verify, stage, and atomically install a collection version'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and not __fish_seen_subcommand_from drop search info install upgrade remove verify list versions switch help" -f -a "upgrade" -d 'Install a newer collection manifest version'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and not __fish_seen_subcommand_from drop search info install upgrade remove verify list versions switch help" -f -a "remove" -d 'Remove a non-current installed collection version'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and not __fish_seen_subcommand_from drop search info install upgrade remove verify list versions switch help" -f -a "verify" -d 'Verify an installed collection version against its manifest'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and not __fish_seen_subcommand_from drop search info install upgrade remove verify list versions switch help" -f -a "list" -d 'List locally installed collections'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and not __fish_seen_subcommand_from drop search info install upgrade remove verify list versions switch help" -f -a "versions" -d 'List installed versions for a collection'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and not __fish_seen_subcommand_from drop search info install upgrade remove verify list versions switch help" -f -a "switch" -d 'Switch the active installed collection version'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and not __fish_seen_subcommand_from drop search info install upgrade remove verify list versions switch help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from drop" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from drop" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from drop" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from drop" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from drop" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from drop" -f -a "export" -d 'Export one or more package directories'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from drop" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from search" -l bootstrap -r
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from search" -l timeout-ms -r
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from search" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from search" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from search" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from search" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from search" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from info" -l ticket -r
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from info" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from info" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from info" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from info" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from info" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from install" -l ticket -r
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from install" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from install" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from install" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from install" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from install" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from upgrade" -l ticket -r
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from upgrade" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from upgrade" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from upgrade" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from upgrade" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from upgrade" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from remove" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from remove" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from remove" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from remove" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from remove" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from verify" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from verify" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from verify" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from verify" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from verify" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from list" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from list" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from list" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from list" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from versions" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from versions" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from versions" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from versions" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from versions" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from switch" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from switch" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from switch" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from switch" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from switch" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from help" -f -a "drop" -d 'Export package versions as compressed CAR drop files'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from help" -f -a "search" -d 'List locally installed packages, optionally filtering by text'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from help" -f -a "info" -d 'Show a collection manifest'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from help" -f -a "install" -d 'Verify, stage, and atomically install a collection version'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from help" -f -a "upgrade" -d 'Install a newer collection manifest version'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from help" -f -a "remove" -d 'Remove a non-current installed collection version'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from help" -f -a "verify" -d 'Verify an installed collection version against its manifest'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from help" -f -a "list" -d 'List locally installed collections'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from help" -f -a "versions" -d 'List installed versions for a collection'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from help" -f -a "switch" -d 'Switch the active installed collection version'
complete -c syncweb -n "__fish_syncweb_using_subcommand package; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from create ls join leave invite kick test-relay help" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from create ls join leave invite kick test-relay help" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from create ls join leave invite kick test-relay help" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from create ls join leave invite kick test-relay help" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from create ls join leave invite kick test-relay help" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from create ls join leave invite kick test-relay help" -f -a "create" -d 'Create a named network'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from create ls join leave invite kick test-relay help" -f -a "ls" -d 'List networks or inspect one'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from create ls join leave invite kick test-relay help" -f -a "join" -d 'Join a network from an invitation'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from create ls join leave invite kick test-relay help" -f -a "leave" -d 'Leave a network'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from create ls join leave invite kick test-relay help" -f -a "invite" -d 'Generate a network invitation'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from create ls join leave invite kick test-relay help" -f -a "kick" -d 'Remove a device from a network'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from create ls join leave invite kick test-relay help" -f -a "test-relay" -d 'Test a Syncthing relay TCP connection'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from create ls join leave invite kick test-relay help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from create" -l label -r
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from create" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from create" -l invite-only
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from create" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from create" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from create" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from create" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from ls" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from ls" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from ls" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from ls" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from ls" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from join" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from join" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from join" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from join" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from join" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from leave" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from leave" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from leave" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from leave" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from leave" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from invite" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from invite" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from invite" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from invite" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from invite" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from kick" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from kick" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from kick" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from kick" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from kick" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from test-relay" -l relay-url -r
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from test-relay" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from test-relay" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from test-relay" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from test-relay" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from test-relay" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from help" -f -a "create" -d 'Create a named network'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from help" -f -a "ls" -d 'List networks or inspect one'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from help" -f -a "join" -d 'Join a network from an invitation'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from help" -f -a "leave" -d 'Leave a network'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from help" -f -a "invite" -d 'Generate a network invitation'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from help" -f -a "kick" -d 'Remove a device from a network'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from help" -f -a "test-relay" -d 'Test a Syncthing relay TCP connection'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand completions" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand completions" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand completions" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand completions" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand completions" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand manpages" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand manpages" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand manpages" -l json -d 'Emit machine-readable JSON where supported'
complete -c syncweb -n "__fish_syncweb_using_subcommand manpages" -l no-color -d 'Disable colored output'
complete -c syncweb -n "__fish_syncweb_using_subcommand manpages" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "version" -d 'Show syncweb version information'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "repl" -d 'Start an interactive command shell'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "start" -d 'Start the local syncweb node for one command invocation'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "shutdown" -d 'Stop the local syncweb node'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "create" -d 'Create a synchronized folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "join" -d 'Join a folder from an Iroh document ticket'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "accept" -d 'Accept a locally available folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "drop" -d 'Remove a local folder replica'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "folders" -d 'List managed folders'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "devices" -d 'Show this device\'s Iroh and Syncthing identities'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "config" -d 'Show or update local configuration'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "ls" -d 'List files in a local folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "find" -d 'Search local files'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "sort" -d 'Sort local files by discovery criteria'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "stat" -d 'Show detailed metadata for a local file'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "download" -d 'Download folder content or copy a local file'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "import" -d 'Import local files into a synchronized folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "backup" -d 'Create a content-addressed snapshot'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "restore" -d 'Restore a snapshot to a folder or directory'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "snapshots" -d 'List, diff, or delete snapshots'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "health" -d 'Show seeding status per folder blob'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "init" -d 'Initialize a folder and print a shareable URL'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "automatic" -d 'Run rules-based automatic synchronization'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "watch" -d 'Watch a folder and import filesystem changes'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "stats" -d 'Show persisted bandwidth accounting'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "verify" -d 'Re-check local folder blob integrity'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "schedule" -d 'Show or update synchronization schedules'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "subscribe" -d 'Subscribe to a folder with event filters'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "publish" -d 'Publish a folder or blob for public read access'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "unpublish" -d 'Remove a public blob pin'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "collection" -d 'Create and publish versioned content collections'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "package" -d 'Manage locally installed collection packages'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "network" -d 'Network connectivity utilities'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "completions" -d 'Generate shell completions'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "manpages" -d 'Generate manpages'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl start shutdown create join accept drop folders devices config ls find sort stat download import backup restore snapshots health init automatic watch stats verify schedule subscribe publish unpublish collection package network completions manpages help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "set" -d 'Set a configuration value'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "show" -d 'Show configuration, optionally limited to a section'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from snapshots" -f -a "diff" -d 'Compare two snapshots'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from snapshots" -f -a "delete" -d 'Delete a snapshot and release its pins'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from schedule" -f -a "set" -d 'Update the global schedule'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from schedule" -f -a "folder" -d 'Set schedule overrides for a named folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from collection" -f -a "init" -d 'Initialize a directory as a versioned collection'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from collection" -f -a "add" -d 'Scan files and update the local collection manifest'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from collection" -f -a "versions" -d 'Create a new collection manifest version'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from collection" -f -a "publish" -d 'Store a collection manifest and mutable head in a folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from package" -f -a "drop" -d 'Export package versions as compressed CAR drop files'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from package" -f -a "search" -d 'List locally installed packages, optionally filtering by text'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from package" -f -a "info" -d 'Show a collection manifest'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from package" -f -a "install" -d 'Verify, stage, and atomically install a collection version'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from package" -f -a "upgrade" -d 'Install a newer collection manifest version'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from package" -f -a "remove" -d 'Remove a non-current installed collection version'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from package" -f -a "verify" -d 'Verify an installed collection version against its manifest'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from package" -f -a "list" -d 'List locally installed collections'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from package" -f -a "versions" -d 'List installed versions for a collection'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from package" -f -a "switch" -d 'Switch the active installed collection version'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "create" -d 'Create a named network'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "ls" -d 'List networks or inspect one'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "join" -d 'Join a network from an invitation'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "leave" -d 'Leave a network'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "invite" -d 'Generate a network invitation'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "kick" -d 'Remove a device from a network'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "test-relay" -d 'Test a Syncthing relay TCP connection'
