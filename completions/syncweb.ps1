
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
            [CompletionResult]::new('ls', 'ls', [CompletionResultType]::ParameterValue, 'List files in a local folder')
            [CompletionResult]::new('find', 'find', [CompletionResultType]::ParameterValue, 'Search local files')
            [CompletionResult]::new('sort', 'sort', [CompletionResultType]::ParameterValue, 'Sort local files by discovery criteria')
            [CompletionResult]::new('stat', 'stat', [CompletionResultType]::ParameterValue, 'Show detailed metadata for a local file')
            [CompletionResult]::new('download', 'download', [CompletionResultType]::ParameterValue, 'Download a local file to a destination')
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize a folder and print a shareable URL')
            [CompletionResult]::new('automatic', 'automatic', [CompletionResultType]::ParameterValue, 'Run rules-based automatic synchronization')
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Subscribe to a folder with event filters')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish a folder or blob for public read access')
            [CompletionResult]::new('unpublish', 'unpublish', [CompletionResultType]::ParameterValue, 'Remove a public blob pin')
            [CompletionResult]::new('collection', 'collection', [CompletionResultType]::ParameterValue, 'Create and publish versioned content collections')
            [CompletionResult]::new('package', 'package', [CompletionResultType]::ParameterValue, 'Manage locally installed collection packages')
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
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Add the created folder to a named network')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--relay-fallback', '--relay-fallback', [CompletionResultType]::ParameterName, 'Enable Syncthing relay fallback for this folder')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;join' {
            [CompletionResult]::new('--mode', '--mode', [CompletionResultType]::ParameterName, 'mode')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Add the joined folder to a named network')
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
        'syncweb;ls' {
            [CompletionResult]::new('--sort', '--sort', [CompletionResultType]::ParameterName, 'Collect and sort output instead of streaming it')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'Scanner threads (1 disables parallelism, 0 uses all available CPUs)')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;find' {
            [CompletionResult]::new('--kind', '--kind', [CompletionResultType]::ParameterName, 'kind')
            [CompletionResult]::new('--max-depth', '--max-depth', [CompletionResultType]::ParameterName, 'max-depth')
            [CompletionResult]::new('--min-size', '--min-size', [CompletionResultType]::ParameterName, 'min-size')
            [CompletionResult]::new('--max-size', '--max-size', [CompletionResultType]::ParameterName, 'max-size')
            [CompletionResult]::new('--extension', '--extension', [CompletionResultType]::ParameterName, 'extension')
            [CompletionResult]::new('--type', '--type', [CompletionResultType]::ParameterName, 'type')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'Scanner threads (1 disables parallelism, 0 uses all available CPUs)')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;sort' {
            [CompletionResult]::new('--by', '--by', [CompletionResultType]::ParameterName, 'by')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'Scanner threads (1 disables parallelism, 0 uses all available CPUs)')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;stat' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'format')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'Scanner threads (1 disables parallelism, 0 uses all available CPUs)')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--terse', '--terse', [CompletionResultType]::ParameterName, 'terse')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;download' {
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'Copy threads (1 disables parallelism, 0 uses all available CPUs)')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;init' {
            [CompletionResult]::new('--mode', '--mode', [CompletionResultType]::ParameterName, 'mode')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;automatic' {
            [CompletionResult]::new('--paths', '--paths', [CompletionResultType]::ParameterName, 'Paths evaluated by --dry-run')
            [CompletionResult]::new('--filters', '--filters', [CompletionResultType]::ParameterName, 'Filter configuration (defaults to DATA_DIR/filters.toml)')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--show-filters', '--show-filters', [CompletionResultType]::ParameterName, 'Print the active filter configuration and exit')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Evaluate paths without starting the daemon')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;subscribe' {
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'prefix')
            [CompletionResult]::new('--glob', '--glob', [CompletionResultType]::ParameterName, 'glob')
            [CompletionResult]::new('--max-count', '--max-count', [CompletionResultType]::ParameterName, 'max-count')
            [CompletionResult]::new('--max-size', '--max-size', [CompletionResultType]::ParameterName, 'max-size')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--ingest-only', '--ingest-only', [CompletionResultType]::ParameterName, 'Only deliver entries ingested after subscription')
            [CompletionResult]::new('--ignore-self', '--ignore-self', [CompletionResultType]::ParameterName, 'Ignore events emitted by this subscription session')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;publish' {
            [CompletionResult]::new('--blob', '--blob', [CompletionResultType]::ParameterName, 'Publish this content hash as an unauthenticated blob ticket')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;unpublish' {
            [CompletionResult]::new('--blob', '--blob', [CompletionResultType]::ParameterName, 'blob')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;collection' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize a directory as a versioned collection')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Scan files and update the local collection manifest')
            [CompletionResult]::new('versions', 'versions', [CompletionResultType]::ParameterValue, 'Create a new collection manifest version')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Store a collection manifest and mutable head in a folder')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;collection;init' {
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'version')
            [CompletionResult]::new('--name', '--name', [CompletionResultType]::ParameterName, 'name')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;collection;add' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;collection;versions' {
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'version')
            [CompletionResult]::new('--changelog', '--changelog', [CompletionResultType]::ParameterName, 'changelog')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;collection;publish' {
            [CompletionResult]::new('--namespace', '--namespace', [CompletionResultType]::ParameterName, 'namespace')
            [CompletionResult]::new('--sequence', '--sequence', [CompletionResultType]::ParameterName, 'sequence')
            [CompletionResult]::new('--bootstrap', '--bootstrap', [CompletionResultType]::ParameterName, 'bootstrap')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;collection;help' {
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize a directory as a versioned collection')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Scan files and update the local collection manifest')
            [CompletionResult]::new('versions', 'versions', [CompletionResultType]::ParameterValue, 'Create a new collection manifest version')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Store a collection manifest and mutable head in a folder')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;collection;help;init' {
            break
        }
        'syncweb;collection;help;add' {
            break
        }
        'syncweb;collection;help;versions' {
            break
        }
        'syncweb;collection;help;publish' {
            break
        }
        'syncweb;collection;help;help' {
            break
        }
        'syncweb;package' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'List locally installed packages, optionally filtering by text')
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'Show a collection manifest')
            [CompletionResult]::new('install', 'install', [CompletionResultType]::ParameterValue, 'Verify, stage, and atomically install a collection version')
            [CompletionResult]::new('upgrade', 'upgrade', [CompletionResultType]::ParameterValue, 'Install a newer collection manifest version')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove a non-current installed collection version')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify an installed collection version against its manifest')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List locally installed collections')
            [CompletionResult]::new('versions', 'versions', [CompletionResultType]::ParameterValue, 'List installed versions for a collection')
            [CompletionResult]::new('switch', 'switch', [CompletionResultType]::ParameterValue, 'Switch the active installed collection version')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;package;search' {
            [CompletionResult]::new('--bootstrap', '--bootstrap', [CompletionResultType]::ParameterName, 'bootstrap')
            [CompletionResult]::new('--timeout-ms', '--timeout-ms', [CompletionResultType]::ParameterName, 'timeout-ms')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;info' {
            [CompletionResult]::new('--ticket', '--ticket', [CompletionResultType]::ParameterName, 'ticket')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;install' {
            [CompletionResult]::new('--ticket', '--ticket', [CompletionResultType]::ParameterName, 'ticket')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;upgrade' {
            [CompletionResult]::new('--ticket', '--ticket', [CompletionResultType]::ParameterName, 'ticket')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;remove' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;verify' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;list' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;versions' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;switch' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;help' {
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'List locally installed packages, optionally filtering by text')
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'Show a collection manifest')
            [CompletionResult]::new('install', 'install', [CompletionResultType]::ParameterValue, 'Verify, stage, and atomically install a collection version')
            [CompletionResult]::new('upgrade', 'upgrade', [CompletionResultType]::ParameterValue, 'Install a newer collection manifest version')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove a non-current installed collection version')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify an installed collection version against its manifest')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List locally installed collections')
            [CompletionResult]::new('versions', 'versions', [CompletionResultType]::ParameterValue, 'List installed versions for a collection')
            [CompletionResult]::new('switch', 'switch', [CompletionResultType]::ParameterValue, 'Switch the active installed collection version')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;package;help;search' {
            break
        }
        'syncweb;package;help;info' {
            break
        }
        'syncweb;package;help;install' {
            break
        }
        'syncweb;package;help;upgrade' {
            break
        }
        'syncweb;package;help;remove' {
            break
        }
        'syncweb;package;help;verify' {
            break
        }
        'syncweb;package;help;list' {
            break
        }
        'syncweb;package;help;versions' {
            break
        }
        'syncweb;package;help;switch' {
            break
        }
        'syncweb;package;help;help' {
            break
        }
        'syncweb;network' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a named network')
            [CompletionResult]::new('ls', 'ls', [CompletionResultType]::ParameterValue, 'List networks or inspect one')
            [CompletionResult]::new('join', 'join', [CompletionResultType]::ParameterValue, 'Join a network from an invitation')
            [CompletionResult]::new('leave', 'leave', [CompletionResultType]::ParameterValue, 'Leave a network')
            [CompletionResult]::new('invite', 'invite', [CompletionResultType]::ParameterValue, 'Generate a network invitation')
            [CompletionResult]::new('kick', 'kick', [CompletionResultType]::ParameterValue, 'Remove a device from a network')
            [CompletionResult]::new('test-relay', 'test-relay', [CompletionResultType]::ParameterValue, 'Test a Syncthing relay TCP connection')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;network;create' {
            [CompletionResult]::new('--label', '--label', [CompletionResultType]::ParameterName, 'label')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--invite-only', '--invite-only', [CompletionResultType]::ParameterName, 'invite-only')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;network;ls' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;network;join' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;network;leave' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;network;invite' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;network;kick' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
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
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a named network')
            [CompletionResult]::new('ls', 'ls', [CompletionResultType]::ParameterValue, 'List networks or inspect one')
            [CompletionResult]::new('join', 'join', [CompletionResultType]::ParameterValue, 'Join a network from an invitation')
            [CompletionResult]::new('leave', 'leave', [CompletionResultType]::ParameterValue, 'Leave a network')
            [CompletionResult]::new('invite', 'invite', [CompletionResultType]::ParameterValue, 'Generate a network invitation')
            [CompletionResult]::new('kick', 'kick', [CompletionResultType]::ParameterValue, 'Remove a device from a network')
            [CompletionResult]::new('test-relay', 'test-relay', [CompletionResultType]::ParameterValue, 'Test a Syncthing relay TCP connection')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;network;help;create' {
            break
        }
        'syncweb;network;help;ls' {
            break
        }
        'syncweb;network;help;join' {
            break
        }
        'syncweb;network;help;leave' {
            break
        }
        'syncweb;network;help;invite' {
            break
        }
        'syncweb;network;help;kick' {
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
            [CompletionResult]::new('ls', 'ls', [CompletionResultType]::ParameterValue, 'List files in a local folder')
            [CompletionResult]::new('find', 'find', [CompletionResultType]::ParameterValue, 'Search local files')
            [CompletionResult]::new('sort', 'sort', [CompletionResultType]::ParameterValue, 'Sort local files by discovery criteria')
            [CompletionResult]::new('stat', 'stat', [CompletionResultType]::ParameterValue, 'Show detailed metadata for a local file')
            [CompletionResult]::new('download', 'download', [CompletionResultType]::ParameterValue, 'Download a local file to a destination')
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize a folder and print a shareable URL')
            [CompletionResult]::new('automatic', 'automatic', [CompletionResultType]::ParameterValue, 'Run rules-based automatic synchronization')
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Subscribe to a folder with event filters')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish a folder or blob for public read access')
            [CompletionResult]::new('unpublish', 'unpublish', [CompletionResultType]::ParameterValue, 'Remove a public blob pin')
            [CompletionResult]::new('collection', 'collection', [CompletionResultType]::ParameterValue, 'Create and publish versioned content collections')
            [CompletionResult]::new('package', 'package', [CompletionResultType]::ParameterValue, 'Manage locally installed collection packages')
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
        'syncweb;help;ls' {
            break
        }
        'syncweb;help;find' {
            break
        }
        'syncweb;help;sort' {
            break
        }
        'syncweb;help;stat' {
            break
        }
        'syncweb;help;download' {
            break
        }
        'syncweb;help;init' {
            break
        }
        'syncweb;help;automatic' {
            break
        }
        'syncweb;help;subscribe' {
            break
        }
        'syncweb;help;publish' {
            break
        }
        'syncweb;help;unpublish' {
            break
        }
        'syncweb;help;collection' {
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize a directory as a versioned collection')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Scan files and update the local collection manifest')
            [CompletionResult]::new('versions', 'versions', [CompletionResultType]::ParameterValue, 'Create a new collection manifest version')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Store a collection manifest and mutable head in a folder')
            break
        }
        'syncweb;help;collection;init' {
            break
        }
        'syncweb;help;collection;add' {
            break
        }
        'syncweb;help;collection;versions' {
            break
        }
        'syncweb;help;collection;publish' {
            break
        }
        'syncweb;help;package' {
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'List locally installed packages, optionally filtering by text')
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'Show a collection manifest')
            [CompletionResult]::new('install', 'install', [CompletionResultType]::ParameterValue, 'Verify, stage, and atomically install a collection version')
            [CompletionResult]::new('upgrade', 'upgrade', [CompletionResultType]::ParameterValue, 'Install a newer collection manifest version')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove a non-current installed collection version')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify an installed collection version against its manifest')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List locally installed collections')
            [CompletionResult]::new('versions', 'versions', [CompletionResultType]::ParameterValue, 'List installed versions for a collection')
            [CompletionResult]::new('switch', 'switch', [CompletionResultType]::ParameterValue, 'Switch the active installed collection version')
            break
        }
        'syncweb;help;package;search' {
            break
        }
        'syncweb;help;package;info' {
            break
        }
        'syncweb;help;package;install' {
            break
        }
        'syncweb;help;package;upgrade' {
            break
        }
        'syncweb;help;package;remove' {
            break
        }
        'syncweb;help;package;verify' {
            break
        }
        'syncweb;help;package;list' {
            break
        }
        'syncweb;help;package;versions' {
            break
        }
        'syncweb;help;package;switch' {
            break
        }
        'syncweb;help;network' {
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a named network')
            [CompletionResult]::new('ls', 'ls', [CompletionResultType]::ParameterValue, 'List networks or inspect one')
            [CompletionResult]::new('join', 'join', [CompletionResultType]::ParameterValue, 'Join a network from an invitation')
            [CompletionResult]::new('leave', 'leave', [CompletionResultType]::ParameterValue, 'Leave a network')
            [CompletionResult]::new('invite', 'invite', [CompletionResultType]::ParameterValue, 'Generate a network invitation')
            [CompletionResult]::new('kick', 'kick', [CompletionResultType]::ParameterValue, 'Remove a device from a network')
            [CompletionResult]::new('test-relay', 'test-relay', [CompletionResultType]::ParameterValue, 'Test a Syncthing relay TCP connection')
            break
        }
        'syncweb;help;network;create' {
            break
        }
        'syncweb;help;network;ls' {
            break
        }
        'syncweb;help;network;join' {
            break
        }
        'syncweb;help;network;leave' {
            break
        }
        'syncweb;help;network;invite' {
            break
        }
        'syncweb;help;network;kick' {
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
