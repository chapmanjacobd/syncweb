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
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -l relay-fallback -d 'Enable Syncthing relay fallback for this folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand create" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand join" -l mode -r
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
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from test-relay help" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from test-relay help" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from test-relay help" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from test-relay help" -f -a "test-relay" -d 'Test a Syncthing relay TCP connection'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and not __fish_seen_subcommand_from test-relay help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from test-relay" -l relay-url -r
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from test-relay" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from test-relay" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from test-relay" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from help" -f -a "test-relay" -d 'Test a Syncthing relay TCP connection'
complete -c syncweb -n "__fish_syncweb_using_subcommand network; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand completions" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand completions" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand completions" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand manpages" -l data-dir -d 'Directory used for persistent node identity and data' -r -F
complete -c syncweb -n "__fish_syncweb_using_subcommand manpages" -l verbose -d 'Enable verbose structured logging'
complete -c syncweb -n "__fish_syncweb_using_subcommand manpages" -s h -l help -d 'Print help'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config network completions manpages help" -f -a "version" -d 'Show syncweb version information'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config network completions manpages help" -f -a "repl" -d 'Start an interactive command shell'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config network completions manpages help" -f -a "create" -d 'Create a synchronized folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config network completions manpages help" -f -a "join" -d 'Join a folder from an Iroh document ticket'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config network completions manpages help" -f -a "accept" -d 'Accept a locally available folder'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config network completions manpages help" -f -a "drop" -d 'Remove a local folder replica'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config network completions manpages help" -f -a "folders" -d 'List managed folders'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config network completions manpages help" -f -a "devices" -d 'Show this device\'s Iroh and Syncthing identities'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config network completions manpages help" -f -a "config" -d 'Show or update local configuration'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config network completions manpages help" -f -a "network" -d 'Network connectivity utilities'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config network completions manpages help" -f -a "completions" -d 'Generate shell completions'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config network completions manpages help" -f -a "manpages" -d 'Generate manpages'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and not __fish_seen_subcommand_from version repl create join accept drop folders devices config network completions manpages help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "set" -d 'Set a configuration value'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "show" -d 'Show configuration, optionally limited to a section'
complete -c syncweb -n "__fish_syncweb_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "test-relay" -d 'Test a Syncthing relay TCP connection'
