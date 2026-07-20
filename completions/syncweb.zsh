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
(( $+functions[_syncweb__subcmd__drop_commands] )) ||
_syncweb__subcmd__drop_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb drop commands' commands "$@"
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
(( $+functions[_syncweb__subcmd__help__subcmd__drop_commands] )) ||
_syncweb__subcmd__help__subcmd__drop_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help drop commands' commands "$@"
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
(( $+functions[_syncweb__subcmd__help__subcmd__join_commands] )) ||
_syncweb__subcmd__help__subcmd__join_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help join commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__manpages_commands] )) ||
_syncweb__subcmd__help__subcmd__manpages_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help manpages commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__network_commands] )) ||
_syncweb__subcmd__help__subcmd__network_commands() {
    local commands; commands=(
'test-relay:Test a Syncthing relay TCP connection' \
    )
    _describe -t commands 'syncweb help network commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__network__subcmd__test-relay_commands] )) ||
_syncweb__subcmd__help__subcmd__network__subcmd__test-relay_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help network test-relay commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__repl_commands] )) ||
_syncweb__subcmd__help__subcmd__repl_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help repl commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__help__subcmd__version_commands] )) ||
_syncweb__subcmd__help__subcmd__version_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb help version commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__join_commands] )) ||
_syncweb__subcmd__join_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb join commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__manpages_commands] )) ||
_syncweb__subcmd__manpages_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb manpages commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network_commands] )) ||
_syncweb__subcmd__network_commands() {
    local commands; commands=(
'test-relay:Test a Syncthing relay TCP connection' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb network commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__help_commands] )) ||
_syncweb__subcmd__network__subcmd__help_commands() {
    local commands; commands=(
'test-relay:Test a Syncthing relay TCP connection' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'syncweb network help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__help__subcmd__help_commands] )) ||
_syncweb__subcmd__network__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network help help commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__help__subcmd__test-relay_commands] )) ||
_syncweb__subcmd__network__subcmd__help__subcmd__test-relay_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network help test-relay commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__network__subcmd__test-relay_commands] )) ||
_syncweb__subcmd__network__subcmd__test-relay_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb network test-relay commands' commands "$@"
}
(( $+functions[_syncweb__subcmd__repl_commands] )) ||
_syncweb__subcmd__repl_commands() {
    local commands; commands=()
    _describe -t commands 'syncweb repl commands' commands "$@"
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
