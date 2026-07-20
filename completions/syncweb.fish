# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_syncweb_global_optspecs
    string join \n verbose data-dir= h/help
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
complete -c syncweb -n "__fish_syncweb_needs_command" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "version" -d 'Show syncweb version information'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "repl" -d 'Start an interactive command shell'
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
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "download" -d 'Download a local file to a destination'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "init" -d 'Initialize a folder and print a shareable URL'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "automatic" -d 'Run rules-based automatic synchronization'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "subscribe" -d 'Subscribe to a folder with event filters'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "network" -d 'Network connectivity utilities'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "completions" -d 'Generate shell completions'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "manpages" -d 'Generate manpages'
complete -c syncweb -n "__fish_syncweb_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand version" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand version" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand version" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand repl" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand repl" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand repl" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -l mode -r
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -l network -d 'Add the created folder to a named network' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -l relay-fallback -d 'Enable Syncthing relay fallback for this folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand join" -l mode -r
complete -c syncweb -n "__fish_syncweb_using_subcommand join" -l network -d 'Add the joined folder to a named network' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand join" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand join" -l relay-fallback -d 'Enable Syncthing relay fallback for this folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand join" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand join" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand accept" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand accept" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand accept" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand drop" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand drop" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand drop" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand folders" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand folders" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand folders" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand devices" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand devices" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand devices" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and not __fish_seen_subcommand_from set show help" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and not __fish_seen_subcommand_from set show help" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and not __fish_seen_subcommand_from set show help" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and not __fish_seen_subcommand_from set show help" -f -a "set" -d 'Set a configuration value'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and not __fish_seen_subcommand_from set show help" -f -a "show" -d 'Show configuration, optionally limited to a section'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and not __fish_seen_subcommand_from set show help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from set" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from set" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from set" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from show" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from show" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from show" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "set" -d 'Set a configuration value'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "show" -d 'Show configuration, optionally limited to a section'
complete -c syncweb -n "__fish_syncweb_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand ls" -l sort -d 'Collect and sort output instead of streaming it' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand ls" -l threads -d 'Scanner threads (1 disables parallelism, 0 uses all available CPUs)' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand ls" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand ls" -l verbose -d 'Enable verbose structured logging'
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
complete -c syncweb -n "__fish_syncweb_using_subcommand find" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand sort" -l by -r -f -a "niche\t''
frecency\t''
peers\t''
random\t''
folder\t''"
complete -c syncweb -n "__fish_syncweb_using_subcommand sort" -l threads -d 'Scanner threads (1 disables parallelism, 0 uses all available CPUs)' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand sort" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand sort" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand sort" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand stat" -l format -r
complete -c syncweb -n "__fish_syncweb_using_subcommand stat" -l threads -d 'Scanner threads (1 disables parallelism, 0 uses all available CPUs)' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand stat" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand stat" -l terse
complete -c syncweb -n "__fish_syncweb_using_subcommand stat" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand stat" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand download" -l threads -d 'Copy threads (1 disables parallelism, 0 uses all available CPUs)' -r
complete -c syncweb -n "__fish_syncweb_using_subcommand download" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand download" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand download" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand init" -l mode -r
complete -c syncweb -n "__fish_syncweb_using_subcommand init" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand init" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand init" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand automatic" -l paths -d 'Paths evaluated by --dry-run' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand automatic" -l filters -d 'Filter configuration (defaults to DATA_DIR/filters.toml)' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand automatic" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand automatic" -l show-filters -d 'Print the active filter configuration and exit'
complete -c syncweb -n "__fish_syncweb_using_subcommand automatic" -l dry-run -d 'Evaluate paths without starting the daemon'
complete -c syncweb -n "__fish_syncweb_using_subcommand automatic" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand automatic" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l prefix -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l glob -r
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l max-count -r
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l max-size -r
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l ingest-only -d 'Only deliver entries ingested after subscription'
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l ignore-self -d 'Ignore events emitted by this subscription session'
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand subscribe" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from create ls join leave invite kick test-relay help" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from create ls join leave invite kick test-relay help" -l verbose -d 'Enable verbose structured logging'
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
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from create" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from ls" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from ls" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from ls" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from join" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from join" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from join" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from leave" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from leave" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from leave" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from invite" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from invite" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from invite" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from kick" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from kick" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from kick" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from test-relay" -l relay-url -r
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from test-relay" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from test-relay" -l verbose -d 'Enable verbose structured logging'
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
complete -c syncweb -n "__fish_syncweb_using_subcommand completions" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand manpages" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand manpages" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand manpages" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "version" -d 'Show syncweb version information'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "repl" -d 'Start an interactive command shell'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "create" -d 'Create a synchronized folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "join" -d 'Join a folder from an Iroh document ticket'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "accept" -d 'Accept a locally available folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "drop" -d 'Remove a local folder replica'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "folders" -d 'List managed folders'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "devices" -d 'Show this device\'s Iroh and Syncthing identities'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "config" -d 'Show or update local configuration'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "ls" -d 'List files in a local folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "find" -d 'Search local files'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "sort" -d 'Sort local files by discovery criteria'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "stat" -d 'Show detailed metadata for a local file'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "download" -d 'Download a local file to a destination'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "init" -d 'Initialize a folder and print a shareable URL'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "automatic" -d 'Run rules-based automatic synchronization'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "subscribe" -d 'Subscribe to a folder with event filters'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "network" -d 'Network connectivity utilities'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "completions" -d 'Generate shell completions'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "manpages" -d 'Generate manpages'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config ls find sort stat download init automatic subscribe network completions manpages help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "set" -d 'Set a configuration value'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "show" -d 'Show configuration, optionally limited to a section'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "create" -d 'Create a named network'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "ls" -d 'List networks or inspect one'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "join" -d 'Join a network from an invitation'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "leave" -d 'Leave a network'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "invite" -d 'Generate a network invitation'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "kick" -d 'Remove a device from a network'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "test-relay" -d 'Test a Syncthing relay TCP connection'
