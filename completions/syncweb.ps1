
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'syncweb' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'syncweb'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'syncweb' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('version', 'version', [CompletionResultType]::ParameterValue, 'Show syncweb version information')
            [CompletionResult]::new('repl', 'repl', [CompletionResultType]::ParameterValue, 'Start an interactive command shell')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a synchronized folder')
            [CompletionResult]::new('join', 'join', [CompletionResultType]::ParameterValue, 'Join a folder from an Iroh document ticket')
            [CompletionResult]::new('accept', 'accept', [CompletionResultType]::ParameterValue, 'Accept a locally available folder')
            [CompletionResult]::new('drop', 'drop', [CompletionResultType]::ParameterValue, 'Remove a local folder replica')
            [CompletionResult]::new('folders', 'folders', [CompletionResultType]::ParameterValue, 'List managed folders')
            [CompletionResult]::new('devices', 'devices', [CompletionResultType]::ParameterValue, 'Show this device''s Iroh and Syncthing identities')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Show or update local configuration')
            [CompletionResult]::new('network', 'network', [CompletionResultType]::ParameterValue, 'Network connectivity utilities')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completions')
            [CompletionResult]::new('manpages', 'manpages', [CompletionResultType]::ParameterValue, 'Generate manpages')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;version' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;repl' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;create' {
            [CompletionResult]::new('--mode', '--mode', [CompletionResultType]::ParameterName, 'mode')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--relay-fallback', '--relay-fallback', [CompletionResultType]::ParameterName, 'Enable Syncthing relay fallback for this folder')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;join' {
            [CompletionResult]::new('--mode', '--mode', [CompletionResultType]::ParameterName, 'mode')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--relay-fallback', '--relay-fallback', [CompletionResultType]::ParameterName, 'Enable Syncthing relay fallback for this folder')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;accept' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;drop' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;folders' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;devices' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;config' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set a configuration value')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show configuration, optionally limited to a section')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;config;set' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;config;show' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;config;help' {
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set a configuration value')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show configuration, optionally limited to a section')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;config;help;set' {
            break
        }
        'syncweb;config;help;show' {
            break
        }
        'syncweb;config;help;help' {
            break
        }
        'syncweb;network' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('test-relay', 'test-relay', [CompletionResultType]::ParameterValue, 'Test a Syncthing relay TCP connection')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;network;test-relay' {
            [CompletionResult]::new('--relay-url', '--relay-url', [CompletionResultType]::ParameterName, 'relay-url')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;network;help' {
            [CompletionResult]::new('test-relay', 'test-relay', [CompletionResultType]::ParameterValue, 'Test a Syncthing relay TCP connection')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;network;help;test-relay' {
            break
        }
        'syncweb;network;help;help' {
            break
        }
        'syncweb;completions' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;manpages' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;help' {
            [CompletionResult]::new('version', 'version', [CompletionResultType]::ParameterValue, 'Show syncweb version information')
            [CompletionResult]::new('repl', 'repl', [CompletionResultType]::ParameterValue, 'Start an interactive command shell')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a synchronized folder')
            [CompletionResult]::new('join', 'join', [CompletionResultType]::ParameterValue, 'Join a folder from an Iroh document ticket')
            [CompletionResult]::new('accept', 'accept', [CompletionResultType]::ParameterValue, 'Accept a locally available folder')
            [CompletionResult]::new('drop', 'drop', [CompletionResultType]::ParameterValue, 'Remove a local folder replica')
            [CompletionResult]::new('folders', 'folders', [CompletionResultType]::ParameterValue, 'List managed folders')
            [CompletionResult]::new('devices', 'devices', [CompletionResultType]::ParameterValue, 'Show this device''s Iroh and Syncthing identities')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Show or update local configuration')
            [CompletionResult]::new('network', 'network', [CompletionResultType]::ParameterValue, 'Network connectivity utilities')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completions')
            [CompletionResult]::new('manpages', 'manpages', [CompletionResultType]::ParameterValue, 'Generate manpages')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;help;version' {
            break
        }
        'syncweb;help;repl' {
            break
        }
        'syncweb;help;create' {
            break
        }
        'syncweb;help;join' {
            break
        }
        'syncweb;help;accept' {
            break
        }
        'syncweb;help;drop' {
            break
        }
        'syncweb;help;folders' {
            break
        }
        'syncweb;help;devices' {
            break
        }
        'syncweb;help;config' {
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set a configuration value')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show configuration, optionally limited to a section')
            break
        }
        'syncweb;help;config;set' {
            break
        }
        'syncweb;help;config;show' {
            break
        }
        'syncweb;help;network' {
            [CompletionResult]::new('test-relay', 'test-relay', [CompletionResultType]::ParameterValue, 'Test a Syncthing relay TCP connection')
            break
        }
        'syncweb;help;network;test-relay' {
            break
        }
        'syncweb;help;completions' {
            break
        }
        'syncweb;help;manpages' {
            break
        }
        'syncweb;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
