
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
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('version', 'version', [CompletionResultType]::ParameterValue, 'Show syncweb version information')
            [CompletionResult]::new('start', 'start', [CompletionResultType]::ParameterValue, 'Start the local syncweb daemon')
            [CompletionResult]::new('shutdown', 'shutdown', [CompletionResultType]::ParameterValue, 'Stop the local syncweb node')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show the local daemon status')
            [CompletionResult]::new('reload', 'reload', [CompletionResultType]::ParameterValue, 'Ask the local daemon to reload configuration')
            [CompletionResult]::new('daemon-sync', 'daemon-sync', [CompletionResultType]::ParameterValue, 'Ask the local daemon to trigger synchronization')
            [CompletionResult]::new('unwatch', 'unwatch', [CompletionResultType]::ParameterValue, 'Stop watching a folder for local changes')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a synchronized folder')
            [CompletionResult]::new('join', 'join', [CompletionResultType]::ParameterValue, 'Join a folder from an Iroh document ticket')
            [CompletionResult]::new('leave', 'leave', [CompletionResultType]::ParameterValue, 'Leave and remove a synchronized folder')
            [CompletionResult]::new('unsubscribe', 'unsubscribe', [CompletionResultType]::ParameterValue, 'Unsubscribe from a folder''s live sync loop')
            [CompletionResult]::new('folders', 'folders', [CompletionResultType]::ParameterValue, 'List managed folders')
            [CompletionResult]::new('devices', 'devices', [CompletionResultType]::ParameterValue, 'Show this device''s Iroh and Syncthing identities')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Show or update local configuration')
            [CompletionResult]::new('ls', 'ls', [CompletionResultType]::ParameterValue, 'List files in a local folder')
            [CompletionResult]::new('find', 'find', [CompletionResultType]::ParameterValue, 'Search local files')
            [CompletionResult]::new('sort', 'sort', [CompletionResultType]::ParameterValue, 'Sort local files by discovery criteria')
            [CompletionResult]::new('stat', 'stat', [CompletionResultType]::ParameterValue, 'Show detailed metadata for a local file')
            [CompletionResult]::new('download', 'download', [CompletionResultType]::ParameterValue, 'Download folder content or copy a local file')
            [CompletionResult]::new('import', 'import', [CompletionResultType]::ParameterValue, 'Import local files into a synchronized folder')
            [CompletionResult]::new('snapshot', 'snapshot', [CompletionResultType]::ParameterValue, 'Manage content-addressed snapshots')
            [CompletionResult]::new('health', 'health', [CompletionResultType]::ParameterValue, 'Show seeding status per folder blob')
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize a folder and print a shareable URL')
            [CompletionResult]::new('automatic', 'automatic', [CompletionResultType]::ParameterValue, 'Run rules-based automatic synchronization')
            [CompletionResult]::new('watch', 'watch', [CompletionResultType]::ParameterValue, 'Watch a folder and import filesystem changes')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Show persisted bandwidth accounting')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Re-check local folder blob integrity')
            [CompletionResult]::new('schedule', 'schedule', [CompletionResultType]::ParameterValue, 'Show or update synchronization schedules')
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Subscribe to a folder with event filters')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish a folder or blob for public read access')
            [CompletionResult]::new('unpublish', 'unpublish', [CompletionResultType]::ParameterValue, 'Remove a public blob pin')
            [CompletionResult]::new('collection', 'collection', [CompletionResultType]::ParameterValue, 'Create and publish versioned content collections')
            [CompletionResult]::new('package', 'package', [CompletionResultType]::ParameterValue, 'Manage locally installed collection packages')
            [CompletionResult]::new('network', 'network', [CompletionResultType]::ParameterValue, 'Network connectivity utilities')
            [CompletionResult]::new('indexing', 'indexing', [CompletionResultType]::ParameterValue, 'Manage opt-in indexing, catalogs, and metadata')
            [CompletionResult]::new('link', 'link', [CompletionResultType]::ParameterValue, 'Create and resolve stable syncweb links')
            [CompletionResult]::new('provider', 'provider', [CompletionResultType]::ParameterValue, 'Manage blob provider registrations')
            [CompletionResult]::new('trust', 'trust', [CompletionResultType]::ParameterValue, 'Inspect and delegate local trust')
            [CompletionResult]::new('attest', 'attest', [CompletionResultType]::ParameterValue, 'Sign content provenance attestations')
            [CompletionResult]::new('report', 'report', [CompletionResultType]::ParameterValue, 'Submit a local moderation report')
            [CompletionResult]::new('moderation', 'moderation', [CompletionResultType]::ParameterValue, 'Manage local moderation decisions')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completions')
            [CompletionResult]::new('manpages', 'manpages', [CompletionResultType]::ParameterValue, 'Generate manpages')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;version' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;start' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Override the global persistent data directory')
            [CompletionResult]::new('--log-file', '--log-file', [CompletionResultType]::ParameterName, 'Write daemon logs to this file')
            [CompletionResult]::new('--max-threads', '--max-threads', [CompletionResultType]::ParameterName, 'max-threads')
            [CompletionResult]::new('--sync-interval', '--sync-interval', [CompletionResultType]::ParameterName, 'sync-interval')
            [CompletionResult]::new('--bg', '--bg', [CompletionResultType]::ParameterName, 'Run in the background (daemon mode)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;shutdown' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--force', '--force', [CompletionResultType]::ParameterName, 'Skip graceful shutdown')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;status' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;reload' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;daemon-sync' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;unwatch' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;create' {
            [CompletionResult]::new('--mode', '--mode', [CompletionResultType]::ParameterName, 'Sync mode: sendreceive, receiveonly, sendonly, or publicreadonly')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Add the created folder to a named network')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--relay-fallback', '--relay-fallback', [CompletionResultType]::ParameterName, 'Enable Syncthing relay fallback for this folder')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;join' {
            [CompletionResult]::new('--mode', '--mode', [CompletionResultType]::ParameterName, 'mode')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Add the joined folder to a named network')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Parent directory prepended to the path argument')
            [CompletionResult]::new('--sync-prefix', '--sync-prefix', [CompletionResultType]::ParameterName, 'Area prefix filter for subscription entries')
            [CompletionResult]::new('--glob', '--glob', [CompletionResultType]::ParameterName, 'glob')
            [CompletionResult]::new('--max-count', '--max-count', [CompletionResultType]::ParameterName, 'max-count')
            [CompletionResult]::new('--max-size', '--max-size', [CompletionResultType]::ParameterName, 'max-size')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--relay-fallback', '--relay-fallback', [CompletionResultType]::ParameterName, 'Enable Syncthing relay fallback for this folder')
            [CompletionResult]::new('--once', '--once', [CompletionResultType]::ParameterName, 'Exit after joining without entering the sync loop')
            [CompletionResult]::new('--ingest-only', '--ingest-only', [CompletionResultType]::ParameterName, 'Only deliver entries ingested after subscription')
            [CompletionResult]::new('--ignore-self', '--ignore-self', [CompletionResultType]::ParameterName, 'Ignore events emitted by this subscription session')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;leave' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;unsubscribe' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;folders' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;devices' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;config' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
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
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;config;show' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
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
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;find' {
            [CompletionResult]::new('--kind', '--kind', [CompletionResultType]::ParameterName, 'kind')
            [CompletionResult]::new('--depth', '--depth', [CompletionResultType]::ParameterName, 'Depth constraints: N, +N (min), -N (max)')
            [CompletionResult]::new('--min-depth', '--min-depth', [CompletionResultType]::ParameterName, 'Alternative min depth notation')
            [CompletionResult]::new('--max-depth', '--max-depth', [CompletionResultType]::ParameterName, 'Alternative max depth notation')
            [CompletionResult]::new('--sizes', '--sizes', [CompletionResultType]::ParameterName, 'Size constraints: N, -N, +N, N%10, +5GB, etc.')
            [CompletionResult]::new('--modified-within', '--modified-within', [CompletionResultType]::ParameterName, 'Newer than: ''3 days'', ''2 weeks''')
            [CompletionResult]::new('--modified-before', '--modified-before', [CompletionResultType]::ParameterName, 'Older than: ''3 years'', ''1 month''')
            [CompletionResult]::new('--time-modified', '--time-modified', [CompletionResultType]::ParameterName, 'Time modified: ''-3 days'' (newer), ''+3 days'' (older)')
            [CompletionResult]::new('-e', '-e', [CompletionResultType]::ParameterName, 'File extensions to include')
            [CompletionResult]::new('--extension', '--extension', [CompletionResultType]::ParameterName, 'File extensions to include')
            [CompletionResult]::new('--type', '--type', [CompletionResultType]::ParameterName, 'Filter by type: f=file, d=dir, l=symlink')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'Scanner threads (1 disables parallelism, 0 uses all available CPUs)')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Case insensitive search')
            [CompletionResult]::new('--ignore-case', '--ignore-case', [CompletionResultType]::ParameterName, 'Case insensitive search')
            [CompletionResult]::new('-s', '-s', [CompletionResultType]::ParameterName, 'Case sensitive search')
            [CompletionResult]::new('--case-sensitive', '--case-sensitive', [CompletionResultType]::ParameterName, 'Case sensitive search')
            [CompletionResult]::new('-F', '-F ', [CompletionResultType]::ParameterName, 'Treat patterns as literal strings')
            [CompletionResult]::new('--fixed-strings', '--fixed-strings', [CompletionResultType]::ParameterName, 'Treat patterns as literal strings')
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'Search full path (default: filename only)')
            [CompletionResult]::new('--full-path', '--full-path', [CompletionResultType]::ParameterName, 'Search full path (default: filename only)')
            [CompletionResult]::new('-H', '-H ', [CompletionResultType]::ParameterName, 'Search hidden files and directories')
            [CompletionResult]::new('--hidden', '--hidden', [CompletionResultType]::ParameterName, 'Search hidden files and directories')
            [CompletionResult]::new('-L', '-L ', [CompletionResultType]::ParameterName, 'Follow symbolic links')
            [CompletionResult]::new('--follow-links', '--follow-links', [CompletionResultType]::ParameterName, 'Follow symbolic links')
            [CompletionResult]::new('-a', '-a', [CompletionResultType]::ParameterName, 'Print absolute paths')
            [CompletionResult]::new('--absolute-path', '--absolute-path', [CompletionResultType]::ParameterName, 'Print absolute paths')
            [CompletionResult]::new('-d', '-d', [CompletionResultType]::ParameterName, 'Exclude sendonly/publicreadonly folders from search')
            [CompletionResult]::new('--download', '--download', [CompletionResultType]::ParameterName, 'Exclude sendonly/publicreadonly folders from search')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;sort' {
            [CompletionResult]::new('--by', '--by', [CompletionResultType]::ParameterName, 'by')
            [CompletionResult]::new('--min-seeders', '--min-seeders', [CompletionResultType]::ParameterName, 'Filter files with fewer than N seeders')
            [CompletionResult]::new('--max-seeders', '--max-seeders', [CompletionResultType]::ParameterName, 'Filter files with more than N seeders')
            [CompletionResult]::new('--niche', '--niche', [CompletionResultType]::ParameterName, 'Ideal popularity (peer count) for niche scoring')
            [CompletionResult]::new('--frecency-weight', '--frecency-weight', [CompletionResultType]::ParameterName, 'Divisor for recency weighting in frecency calculation')
            [CompletionResult]::new('--limit-size', '--limit-size', [CompletionResultType]::ParameterName, 'Quit after printing N bytes of files')
            [CompletionResult]::new('--depth', '--depth', [CompletionResultType]::ParameterName, 'Constrain folder aggregates by depth: N, +N (min), -N (max)')
            [CompletionResult]::new('--min-depth', '--min-depth', [CompletionResultType]::ParameterName, 'Alternative min depth notation')
            [CompletionResult]::new('--max-depth', '--max-depth', [CompletionResultType]::ParameterName, 'Alternative max depth notation')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'Scanner threads (1 disables parallelism, 0 uses all available CPUs)')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
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
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;download' {
            [CompletionResult]::new('--max-peers', '--max-peers', [CompletionResultType]::ParameterName, 'Fetch only blobs with at most N observed peers')
            [CompletionResult]::new('--min-peers', '--min-peers', [CompletionResultType]::ParameterName, 'Fetch only blobs with at least N observed peers')
            [CompletionResult]::new('--min-count', '--min-count', [CompletionResultType]::ParameterName, 'Minimum number of blobs to fetch')
            [CompletionResult]::new('--max-count', '--max-count', [CompletionResultType]::ParameterName, 'Maximum number of blobs to fetch')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'Copy threads (1 disables parallelism, 0 uses all available CPUs)')
            [CompletionResult]::new('--hash', '--hash', [CompletionResultType]::ParameterName, 'Content hash to download (single blob mode)')
            [CompletionResult]::new('--from', '--from', [CompletionResultType]::ParameterName, 'Blob ticket(s) for providers (can repeat, requires --hash)')
            [CompletionResult]::new('--provider', '--provider', [CompletionResultType]::ParameterName, 'Blob ticket(s) for providers (can repeat, requires --hash)')
            [CompletionResult]::new('--min-providers', '--min-providers', [CompletionResultType]::ParameterName, 'Minimum providers for healthy replication')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--no-sharing', '--no-sharing', [CompletionResultType]::ParameterName, 'Do not share or seed the downloaded content')
            [CompletionResult]::new('--no-seeding', '--no-seeding', [CompletionResultType]::ParameterName, 'Do not share or seed the downloaded content')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;import' {
            [CompletionResult]::new('--folder', '--folder', [CompletionResultType]::ParameterName, 'Folder namespace; defaults to the only managed folder')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'Import threads (1 disables parallelism, 0 uses all available CPUs)')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;snapshot' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a content-addressed snapshot')
            [CompletionResult]::new('restore', 'restore', [CompletionResultType]::ParameterValue, 'Restore a snapshot to a folder or directory')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List local snapshots')
            [CompletionResult]::new('diff', 'diff', [CompletionResultType]::ParameterValue, 'Compare two snapshots')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a snapshot and release its pins')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;snapshot;create' {
            [CompletionResult]::new('--description', '--description', [CompletionResultType]::ParameterName, 'description')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'Scanner threads (1 disables parallelism, 0 uses all available CPUs)')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;snapshot;restore' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;snapshot;list' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;snapshot;diff' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;snapshot;delete' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;snapshot;help' {
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a content-addressed snapshot')
            [CompletionResult]::new('restore', 'restore', [CompletionResultType]::ParameterValue, 'Restore a snapshot to a folder or directory')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List local snapshots')
            [CompletionResult]::new('diff', 'diff', [CompletionResultType]::ParameterValue, 'Compare two snapshots')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a snapshot and release its pins')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;snapshot;help;create' {
            break
        }
        'syncweb;snapshot;help;restore' {
            break
        }
        'syncweb;snapshot;help;list' {
            break
        }
        'syncweb;snapshot;help;diff' {
            break
        }
        'syncweb;snapshot;help;delete' {
            break
        }
        'syncweb;snapshot;help;help' {
            break
        }
        'syncweb;health' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;init' {
            [CompletionResult]::new('--mode', '--mode', [CompletionResultType]::ParameterName, 'mode')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
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
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;watch' {
            [CompletionResult]::new('--debounce-ms', '--debounce-ms', [CompletionResultType]::ParameterName, 'Debounce changes in milliseconds')
            [CompletionResult]::new('--exclude', '--exclude', [CompletionResultType]::ParameterName, 'Ignore a path glob; may be repeated')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--once', '--once', [CompletionResultType]::ParameterName, 'Process one event and exit')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;stats' {
            [CompletionResult]::new('--folder', '--folder', [CompletionResultType]::ParameterName, 'Limit display to a folder or namespace')
            [CompletionResult]::new('--peer', '--peer', [CompletionResultType]::ParameterName, 'Limit display to a peer node ID')
            [CompletionResult]::new('--period', '--period', [CompletionResultType]::ParameterName, 'Retained for compatibility; counters are persisted since period start')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--reset', '--reset', [CompletionResultType]::ParameterName, 'Reset persisted counters before displaying them')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;verify' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;schedule' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Update the global schedule')
            [CompletionResult]::new('folder', 'folder', [CompletionResultType]::ParameterValue, 'Set schedule overrides for a named folder')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;schedule;set' {
            [CompletionResult]::new('--active', '--active', [CompletionResultType]::ParameterName, 'active')
            [CompletionResult]::new('--bandwidth', '--bandwidth', [CompletionResultType]::ParameterName, 'Bandwidth rate (e.g. ''500K'', ''2M'')')
            [CompletionResult]::new('--period', '--period', [CompletionResultType]::ParameterName, 'Time window for the bandwidth limit (e.g. ''08:00-18:00'')')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;schedule;folder' {
            [CompletionResult]::new('--active', '--active', [CompletionResultType]::ParameterName, 'active')
            [CompletionResult]::new('--max-upload', '--max-upload', [CompletionResultType]::ParameterName, 'max-upload')
            [CompletionResult]::new('--max-download', '--max-download', [CompletionResultType]::ParameterName, 'max-download')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;schedule;help' {
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Update the global schedule')
            [CompletionResult]::new('folder', 'folder', [CompletionResultType]::ParameterValue, 'Set schedule overrides for a named folder')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;schedule;help;set' {
            break
        }
        'syncweb;schedule;help;folder' {
            break
        }
        'syncweb;schedule;help;help' {
            break
        }
        'syncweb;subscribe' {
            [CompletionResult]::new('--sync-prefix', '--sync-prefix', [CompletionResultType]::ParameterName, 'Area prefix filter for subscription entries')
            [CompletionResult]::new('--glob', '--glob', [CompletionResultType]::ParameterName, 'glob')
            [CompletionResult]::new('--max-count', '--max-count', [CompletionResultType]::ParameterName, 'max-count')
            [CompletionResult]::new('--max-size', '--max-size', [CompletionResultType]::ParameterName, 'max-size')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--ingest-only', '--ingest-only', [CompletionResultType]::ParameterName, 'Only deliver entries ingested after subscription')
            [CompletionResult]::new('--ignore-self', '--ignore-self', [CompletionResultType]::ParameterName, 'Ignore events emitted by this subscription session')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;publish' {
            [CompletionResult]::new('--blob', '--blob', [CompletionResultType]::ParameterName, 'Publish this content hash as an unauthenticated blob ticket')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;unpublish' {
            [CompletionResult]::new('--blob', '--blob', [CompletionResultType]::ParameterName, 'Blob content hash to unpublish')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;collection' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
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
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;collection;add' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;collection;versions' {
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'version')
            [CompletionResult]::new('--changelog', '--changelog', [CompletionResultType]::ParameterName, 'changelog')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
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
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
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
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export one or more package directories as compressed CAR archive files')
            [CompletionResult]::new('import', 'import', [CompletionResultType]::ParameterValue, 'Import and install a compressed CAR archive file')
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
        'syncweb;package;export' {
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'version')
            [CompletionResult]::new('--filter', '--filter', [CompletionResultType]::ParameterName, 'filter')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;import' {
            [CompletionResult]::new('--filter', '--filter', [CompletionResultType]::ParameterName, 'filter')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;search' {
            [CompletionResult]::new('--bootstrap', '--bootstrap', [CompletionResultType]::ParameterName, 'bootstrap')
            [CompletionResult]::new('--timeout-ms', '--timeout-ms', [CompletionResultType]::ParameterName, 'timeout-ms')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;info' {
            [CompletionResult]::new('--ticket', '--ticket', [CompletionResultType]::ParameterName, 'ticket')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;install' {
            [CompletionResult]::new('--ticket', '--ticket', [CompletionResultType]::ParameterName, 'ticket')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;upgrade' {
            [CompletionResult]::new('--ticket', '--ticket', [CompletionResultType]::ParameterName, 'ticket')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;remove' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;verify' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;list' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;versions' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;switch' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;help' {
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export one or more package directories as compressed CAR archive files')
            [CompletionResult]::new('import', 'import', [CompletionResultType]::ParameterValue, 'Import and install a compressed CAR archive file')
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
        'syncweb;package;help;export' {
            break
        }
        'syncweb;package;help;import' {
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
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
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
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;network;ls' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;network;join' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;network;leave' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;network;invite' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;network;kick' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;network;test-relay' {
            [CompletionResult]::new('--relay-url', '--relay-url', [CompletionResultType]::ParameterName, 'relay-url')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
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
        'syncweb;indexing' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('enable', 'enable', [CompletionResultType]::ParameterValue, 'Opt a synchronized folder into indexing')
            [CompletionResult]::new('disable', 'disable', [CompletionResultType]::ParameterValue, 'Remove a folder from the local index')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish folder metadata to a catalog')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Search subscribed catalogs')
            [CompletionResult]::new('health', 'health', [CompletionResultType]::ParameterValue, 'Show verified provider health for a content hash')
            [CompletionResult]::new('meta', 'meta', [CompletionResultType]::ParameterValue, 'Manage signed metadata')
            [CompletionResult]::new('filter', 'filter', [CompletionResultType]::ParameterValue, 'Manage local and federated denylists')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;indexing;enable' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;indexing;disable' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;indexing;publish' {
            [CompletionResult]::new('--catalog', '--catalog', [CompletionResultType]::ParameterName, 'catalog')
            [CompletionResult]::new('--tag', '--tag', [CompletionResultType]::ParameterName, 'tag')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;indexing;search' {
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'limit')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;indexing;health' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;indexing;meta' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Append signed metadata to a content hash')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;indexing;meta;add' {
            [CompletionResult]::new('--sequence', '--sequence', [CompletionResultType]::ParameterName, 'sequence')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;indexing;meta;help' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Append signed metadata to a content hash')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;indexing;meta;help;add' {
            break
        }
        'syncweb;indexing;meta;help;help' {
            break
        }
        'syncweb;indexing;filter' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a device, file, or hash denylist rule')
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Import a signed federated filter list')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;indexing;filter;add' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;indexing;filter;subscribe' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;indexing;filter;help' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a device, file, or hash denylist rule')
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Import a signed federated filter list')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;indexing;filter;help;add' {
            break
        }
        'syncweb;indexing;filter;help;subscribe' {
            break
        }
        'syncweb;indexing;filter;help;help' {
            break
        }
        'syncweb;indexing;help' {
            [CompletionResult]::new('enable', 'enable', [CompletionResultType]::ParameterValue, 'Opt a synchronized folder into indexing')
            [CompletionResult]::new('disable', 'disable', [CompletionResultType]::ParameterValue, 'Remove a folder from the local index')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish folder metadata to a catalog')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Search subscribed catalogs')
            [CompletionResult]::new('health', 'health', [CompletionResultType]::ParameterValue, 'Show verified provider health for a content hash')
            [CompletionResult]::new('meta', 'meta', [CompletionResultType]::ParameterValue, 'Manage signed metadata')
            [CompletionResult]::new('filter', 'filter', [CompletionResultType]::ParameterValue, 'Manage local and federated denylists')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;indexing;help;enable' {
            break
        }
        'syncweb;indexing;help;disable' {
            break
        }
        'syncweb;indexing;help;publish' {
            break
        }
        'syncweb;indexing;help;search' {
            break
        }
        'syncweb;indexing;help;health' {
            break
        }
        'syncweb;indexing;help;meta' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Append signed metadata to a content hash')
            break
        }
        'syncweb;indexing;help;meta;add' {
            break
        }
        'syncweb;indexing;help;filter' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a device, file, or hash denylist rule')
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Import a signed federated filter list')
            break
        }
        'syncweb;indexing;help;filter;add' {
            break
        }
        'syncweb;indexing;help;filter;subscribe' {
            break
        }
        'syncweb;indexing;help;help' {
            break
        }
        'syncweb;link' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create an immutable, private, or mutable link')
            [CompletionResult]::new('resolve', 'resolve', [CompletionResultType]::ParameterValue, 'Resolve a stable link')
            [CompletionResult]::new('revoke', 'revoke', [CompletionResultType]::ParameterValue, 'Revoke a private capability link')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;link;create' {
            [CompletionResult]::new('--name', '--name', [CompletionResultType]::ParameterName, 'name')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'version')
            [CompletionResult]::new('--sequence', '--sequence', [CompletionResultType]::ParameterName, 'sequence')
            [CompletionResult]::new('--expires', '--expires', [CompletionResultType]::ParameterName, 'Private-link expiration as a Unix timestamp')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--private', '--private', [CompletionResultType]::ParameterName, 'private')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;link;resolve' {
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'version')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;link;revoke' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;link;help' {
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create an immutable, private, or mutable link')
            [CompletionResult]::new('resolve', 'resolve', [CompletionResultType]::ParameterValue, 'Resolve a stable link')
            [CompletionResult]::new('revoke', 'revoke', [CompletionResultType]::ParameterValue, 'Revoke a private capability link')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;link;help;create' {
            break
        }
        'syncweb;link;help;resolve' {
            break
        }
        'syncweb;link;help;revoke' {
            break
        }
        'syncweb;link;help;help' {
            break
        }
        'syncweb;provider' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Register a blob ticket as an alternate provider')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;provider;add' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;provider;help' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Register a blob ticket as an alternate provider')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;provider;help;add' {
            break
        }
        'syncweb;provider;help;help' {
            break
        }
        'syncweb;trust' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show trust and moderation state')
            [CompletionResult]::new('delegate', 'delegate', [CompletionResultType]::ParameterValue, 'Delegate trust to a publisher identity')
            [CompletionResult]::new('provider', 'provider', [CompletionResultType]::ParameterValue, 'Manage provider trust and bans')
            [CompletionResult]::new('stream', 'stream', [CompletionResultType]::ParameterValue, 'Publish or subscribe to provider trust signals')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;trust;show' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;trust;delegate' {
            [CompletionResult]::new('--expires', '--expires', [CompletionResultType]::ParameterName, 'expires')
            [CompletionResult]::new('--scope', '--scope', [CompletionResultType]::ParameterName, 'scope')
            [CompletionResult]::new('--sequence', '--sequence', [CompletionResultType]::ParameterName, 'sequence')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;trust;provider' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show provider reputation, bans, and trust records')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List providers known to the local index')
            [CompletionResult]::new('ban', 'ban', [CompletionResultType]::ParameterValue, 'Ban a provider globally or for one content hash')
            [CompletionResult]::new('unban', 'unban', [CompletionResultType]::ParameterValue, 'Remove a provider''s global and scoped bans')
            [CompletionResult]::new('vouch', 'vouch', [CompletionResultType]::ParameterValue, 'Vouch for a provider')
            [CompletionResult]::new('distrust', 'distrust', [CompletionResultType]::ParameterValue, 'Distrust a provider')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;trust;provider;show' {
            [CompletionResult]::new('--hash', '--hash', [CompletionResultType]::ParameterName, 'Evaluate content-scoped trust for this hash')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;trust;provider;list' {
            [CompletionResult]::new('--hash', '--hash', [CompletionResultType]::ParameterName, 'Evaluate content-scoped trust for this hash')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;trust;provider;ban' {
            [CompletionResult]::new('--hash', '--hash', [CompletionResultType]::ParameterName, 'hash')
            [CompletionResult]::new('--reason', '--reason', [CompletionResultType]::ParameterName, 'reason')
            [CompletionResult]::new('--duration', '--duration', [CompletionResultType]::ParameterName, 'Ban duration in seconds')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;trust;provider;unban' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;trust;provider;vouch' {
            [CompletionResult]::new('--scope', '--scope', [CompletionResultType]::ParameterName, 'scope')
            [CompletionResult]::new('--reason', '--reason', [CompletionResultType]::ParameterName, 'reason')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;trust;provider;distrust' {
            [CompletionResult]::new('--scope', '--scope', [CompletionResultType]::ParameterName, 'scope')
            [CompletionResult]::new('--reason', '--reason', [CompletionResultType]::ParameterName, 'reason')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;trust;provider;help' {
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show provider reputation, bans, and trust records')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List providers known to the local index')
            [CompletionResult]::new('ban', 'ban', [CompletionResultType]::ParameterValue, 'Ban a provider globally or for one content hash')
            [CompletionResult]::new('unban', 'unban', [CompletionResultType]::ParameterValue, 'Remove a provider''s global and scoped bans')
            [CompletionResult]::new('vouch', 'vouch', [CompletionResultType]::ParameterValue, 'Vouch for a provider')
            [CompletionResult]::new('distrust', 'distrust', [CompletionResultType]::ParameterValue, 'Distrust a provider')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;trust;provider;help;show' {
            break
        }
        'syncweb;trust;provider;help;list' {
            break
        }
        'syncweb;trust;provider;help;ban' {
            break
        }
        'syncweb;trust;provider;help;unban' {
            break
        }
        'syncweb;trust;provider;help;vouch' {
            break
        }
        'syncweb;trust;provider;help;distrust' {
            break
        }
        'syncweb;trust;provider;help;help' {
            break
        }
        'syncweb;trust;stream' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Subscribe to a provider trust stream ticket or file')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish a signed provider trust signal')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;trust;stream;subscribe' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;trust;stream;publish' {
            [CompletionResult]::new('--provider', '--provider', [CompletionResultType]::ParameterName, 'provider')
            [CompletionResult]::new('--signal', '--signal', [CompletionResultType]::ParameterName, 'signal')
            [CompletionResult]::new('--hash', '--hash', [CompletionResultType]::ParameterName, 'hash')
            [CompletionResult]::new('--sequence', '--sequence', [CompletionResultType]::ParameterName, 'sequence')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;trust;stream;help' {
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Subscribe to a provider trust stream ticket or file')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish a signed provider trust signal')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;trust;stream;help;subscribe' {
            break
        }
        'syncweb;trust;stream;help;publish' {
            break
        }
        'syncweb;trust;stream;help;help' {
            break
        }
        'syncweb;trust;help' {
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show trust and moderation state')
            [CompletionResult]::new('delegate', 'delegate', [CompletionResultType]::ParameterValue, 'Delegate trust to a publisher identity')
            [CompletionResult]::new('provider', 'provider', [CompletionResultType]::ParameterValue, 'Manage provider trust and bans')
            [CompletionResult]::new('stream', 'stream', [CompletionResultType]::ParameterValue, 'Publish or subscribe to provider trust signals')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;trust;help;show' {
            break
        }
        'syncweb;trust;help;delegate' {
            break
        }
        'syncweb;trust;help;provider' {
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show provider reputation, bans, and trust records')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List providers known to the local index')
            [CompletionResult]::new('ban', 'ban', [CompletionResultType]::ParameterValue, 'Ban a provider globally or for one content hash')
            [CompletionResult]::new('unban', 'unban', [CompletionResultType]::ParameterValue, 'Remove a provider''s global and scoped bans')
            [CompletionResult]::new('vouch', 'vouch', [CompletionResultType]::ParameterValue, 'Vouch for a provider')
            [CompletionResult]::new('distrust', 'distrust', [CompletionResultType]::ParameterValue, 'Distrust a provider')
            break
        }
        'syncweb;trust;help;provider;show' {
            break
        }
        'syncweb;trust;help;provider;list' {
            break
        }
        'syncweb;trust;help;provider;ban' {
            break
        }
        'syncweb;trust;help;provider;unban' {
            break
        }
        'syncweb;trust;help;provider;vouch' {
            break
        }
        'syncweb;trust;help;provider;distrust' {
            break
        }
        'syncweb;trust;help;stream' {
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Subscribe to a provider trust stream ticket or file')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish a signed provider trust signal')
            break
        }
        'syncweb;trust;help;stream;subscribe' {
            break
        }
        'syncweb;trust;help;stream;publish' {
            break
        }
        'syncweb;trust;help;help' {
            break
        }
        'syncweb;attest' {
            [CompletionResult]::new('--license', '--license', [CompletionResultType]::ParameterName, 'license')
            [CompletionResult]::new('--provenance', '--provenance', [CompletionResultType]::ParameterName, 'Provenance attestation type')
            [CompletionResult]::new('--derivative', '--derivative', [CompletionResultType]::ParameterName, 'Derivative work attestation type')
            [CompletionResult]::new('--sequence', '--sequence', [CompletionResultType]::ParameterName, 'sequence')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;report' {
            [CompletionResult]::new('--reason', '--reason', [CompletionResultType]::ParameterName, 'Reason for the report')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;moderation' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('ls', 'ls', [CompletionResultType]::ParameterValue, 'List local moderation records')
            [CompletionResult]::new('hide', 'hide', [CompletionResultType]::ParameterValue, 'Hide a content record locally')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;moderation;ls' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;moderation;hide' {
            [CompletionResult]::new('--reason', '--reason', [CompletionResultType]::ParameterName, 'reason')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;moderation;help' {
            [CompletionResult]::new('ls', 'ls', [CompletionResultType]::ParameterValue, 'List local moderation records')
            [CompletionResult]::new('hide', 'hide', [CompletionResultType]::ParameterValue, 'Hide a content record locally')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;moderation;help;ls' {
            break
        }
        'syncweb;moderation;help;hide' {
            break
        }
        'syncweb;moderation;help;help' {
            break
        }
        'syncweb;completions' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;manpages' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;help' {
            [CompletionResult]::new('version', 'version', [CompletionResultType]::ParameterValue, 'Show syncweb version information')
            [CompletionResult]::new('start', 'start', [CompletionResultType]::ParameterValue, 'Start the local syncweb daemon')
            [CompletionResult]::new('shutdown', 'shutdown', [CompletionResultType]::ParameterValue, 'Stop the local syncweb node')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show the local daemon status')
            [CompletionResult]::new('reload', 'reload', [CompletionResultType]::ParameterValue, 'Ask the local daemon to reload configuration')
            [CompletionResult]::new('daemon-sync', 'daemon-sync', [CompletionResultType]::ParameterValue, 'Ask the local daemon to trigger synchronization')
            [CompletionResult]::new('unwatch', 'unwatch', [CompletionResultType]::ParameterValue, 'Stop watching a folder for local changes')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a synchronized folder')
            [CompletionResult]::new('join', 'join', [CompletionResultType]::ParameterValue, 'Join a folder from an Iroh document ticket')
            [CompletionResult]::new('leave', 'leave', [CompletionResultType]::ParameterValue, 'Leave and remove a synchronized folder')
            [CompletionResult]::new('unsubscribe', 'unsubscribe', [CompletionResultType]::ParameterValue, 'Unsubscribe from a folder''s live sync loop')
            [CompletionResult]::new('folders', 'folders', [CompletionResultType]::ParameterValue, 'List managed folders')
            [CompletionResult]::new('devices', 'devices', [CompletionResultType]::ParameterValue, 'Show this device''s Iroh and Syncthing identities')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Show or update local configuration')
            [CompletionResult]::new('ls', 'ls', [CompletionResultType]::ParameterValue, 'List files in a local folder')
            [CompletionResult]::new('find', 'find', [CompletionResultType]::ParameterValue, 'Search local files')
            [CompletionResult]::new('sort', 'sort', [CompletionResultType]::ParameterValue, 'Sort local files by discovery criteria')
            [CompletionResult]::new('stat', 'stat', [CompletionResultType]::ParameterValue, 'Show detailed metadata for a local file')
            [CompletionResult]::new('download', 'download', [CompletionResultType]::ParameterValue, 'Download folder content or copy a local file')
            [CompletionResult]::new('import', 'import', [CompletionResultType]::ParameterValue, 'Import local files into a synchronized folder')
            [CompletionResult]::new('snapshot', 'snapshot', [CompletionResultType]::ParameterValue, 'Manage content-addressed snapshots')
            [CompletionResult]::new('health', 'health', [CompletionResultType]::ParameterValue, 'Show seeding status per folder blob')
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize a folder and print a shareable URL')
            [CompletionResult]::new('automatic', 'automatic', [CompletionResultType]::ParameterValue, 'Run rules-based automatic synchronization')
            [CompletionResult]::new('watch', 'watch', [CompletionResultType]::ParameterValue, 'Watch a folder and import filesystem changes')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Show persisted bandwidth accounting')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Re-check local folder blob integrity')
            [CompletionResult]::new('schedule', 'schedule', [CompletionResultType]::ParameterValue, 'Show or update synchronization schedules')
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Subscribe to a folder with event filters')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish a folder or blob for public read access')
            [CompletionResult]::new('unpublish', 'unpublish', [CompletionResultType]::ParameterValue, 'Remove a public blob pin')
            [CompletionResult]::new('collection', 'collection', [CompletionResultType]::ParameterValue, 'Create and publish versioned content collections')
            [CompletionResult]::new('package', 'package', [CompletionResultType]::ParameterValue, 'Manage locally installed collection packages')
            [CompletionResult]::new('network', 'network', [CompletionResultType]::ParameterValue, 'Network connectivity utilities')
            [CompletionResult]::new('indexing', 'indexing', [CompletionResultType]::ParameterValue, 'Manage opt-in indexing, catalogs, and metadata')
            [CompletionResult]::new('link', 'link', [CompletionResultType]::ParameterValue, 'Create and resolve stable syncweb links')
            [CompletionResult]::new('provider', 'provider', [CompletionResultType]::ParameterValue, 'Manage blob provider registrations')
            [CompletionResult]::new('trust', 'trust', [CompletionResultType]::ParameterValue, 'Inspect and delegate local trust')
            [CompletionResult]::new('attest', 'attest', [CompletionResultType]::ParameterValue, 'Sign content provenance attestations')
            [CompletionResult]::new('report', 'report', [CompletionResultType]::ParameterValue, 'Submit a local moderation report')
            [CompletionResult]::new('moderation', 'moderation', [CompletionResultType]::ParameterValue, 'Manage local moderation decisions')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completions')
            [CompletionResult]::new('manpages', 'manpages', [CompletionResultType]::ParameterValue, 'Generate manpages')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'syncweb;help;version' {
            break
        }
        'syncweb;help;start' {
            break
        }
        'syncweb;help;shutdown' {
            break
        }
        'syncweb;help;status' {
            break
        }
        'syncweb;help;reload' {
            break
        }
        'syncweb;help;daemon-sync' {
            break
        }
        'syncweb;help;unwatch' {
            break
        }
        'syncweb;help;create' {
            break
        }
        'syncweb;help;join' {
            break
        }
        'syncweb;help;leave' {
            break
        }
        'syncweb;help;unsubscribe' {
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
        'syncweb;help;import' {
            break
        }
        'syncweb;help;snapshot' {
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a content-addressed snapshot')
            [CompletionResult]::new('restore', 'restore', [CompletionResultType]::ParameterValue, 'Restore a snapshot to a folder or directory')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List local snapshots')
            [CompletionResult]::new('diff', 'diff', [CompletionResultType]::ParameterValue, 'Compare two snapshots')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a snapshot and release its pins')
            break
        }
        'syncweb;help;snapshot;create' {
            break
        }
        'syncweb;help;snapshot;restore' {
            break
        }
        'syncweb;help;snapshot;list' {
            break
        }
        'syncweb;help;snapshot;diff' {
            break
        }
        'syncweb;help;snapshot;delete' {
            break
        }
        'syncweb;help;health' {
            break
        }
        'syncweb;help;init' {
            break
        }
        'syncweb;help;automatic' {
            break
        }
        'syncweb;help;watch' {
            break
        }
        'syncweb;help;stats' {
            break
        }
        'syncweb;help;verify' {
            break
        }
        'syncweb;help;schedule' {
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Update the global schedule')
            [CompletionResult]::new('folder', 'folder', [CompletionResultType]::ParameterValue, 'Set schedule overrides for a named folder')
            break
        }
        'syncweb;help;schedule;set' {
            break
        }
        'syncweb;help;schedule;folder' {
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
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export one or more package directories as compressed CAR archive files')
            [CompletionResult]::new('import', 'import', [CompletionResultType]::ParameterValue, 'Import and install a compressed CAR archive file')
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
        'syncweb;help;package;export' {
            break
        }
        'syncweb;help;package;import' {
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
        'syncweb;help;indexing' {
            [CompletionResult]::new('enable', 'enable', [CompletionResultType]::ParameterValue, 'Opt a synchronized folder into indexing')
            [CompletionResult]::new('disable', 'disable', [CompletionResultType]::ParameterValue, 'Remove a folder from the local index')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish folder metadata to a catalog')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Search subscribed catalogs')
            [CompletionResult]::new('health', 'health', [CompletionResultType]::ParameterValue, 'Show verified provider health for a content hash')
            [CompletionResult]::new('meta', 'meta', [CompletionResultType]::ParameterValue, 'Manage signed metadata')
            [CompletionResult]::new('filter', 'filter', [CompletionResultType]::ParameterValue, 'Manage local and federated denylists')
            break
        }
        'syncweb;help;indexing;enable' {
            break
        }
        'syncweb;help;indexing;disable' {
            break
        }
        'syncweb;help;indexing;publish' {
            break
        }
        'syncweb;help;indexing;search' {
            break
        }
        'syncweb;help;indexing;health' {
            break
        }
        'syncweb;help;indexing;meta' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Append signed metadata to a content hash')
            break
        }
        'syncweb;help;indexing;meta;add' {
            break
        }
        'syncweb;help;indexing;filter' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a device, file, or hash denylist rule')
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Import a signed federated filter list')
            break
        }
        'syncweb;help;indexing;filter;add' {
            break
        }
        'syncweb;help;indexing;filter;subscribe' {
            break
        }
        'syncweb;help;link' {
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create an immutable, private, or mutable link')
            [CompletionResult]::new('resolve', 'resolve', [CompletionResultType]::ParameterValue, 'Resolve a stable link')
            [CompletionResult]::new('revoke', 'revoke', [CompletionResultType]::ParameterValue, 'Revoke a private capability link')
            break
        }
        'syncweb;help;link;create' {
            break
        }
        'syncweb;help;link;resolve' {
            break
        }
        'syncweb;help;link;revoke' {
            break
        }
        'syncweb;help;provider' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Register a blob ticket as an alternate provider')
            break
        }
        'syncweb;help;provider;add' {
            break
        }
        'syncweb;help;trust' {
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show trust and moderation state')
            [CompletionResult]::new('delegate', 'delegate', [CompletionResultType]::ParameterValue, 'Delegate trust to a publisher identity')
            [CompletionResult]::new('provider', 'provider', [CompletionResultType]::ParameterValue, 'Manage provider trust and bans')
            [CompletionResult]::new('stream', 'stream', [CompletionResultType]::ParameterValue, 'Publish or subscribe to provider trust signals')
            break
        }
        'syncweb;help;trust;show' {
            break
        }
        'syncweb;help;trust;delegate' {
            break
        }
        'syncweb;help;trust;provider' {
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show provider reputation, bans, and trust records')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List providers known to the local index')
            [CompletionResult]::new('ban', 'ban', [CompletionResultType]::ParameterValue, 'Ban a provider globally or for one content hash')
            [CompletionResult]::new('unban', 'unban', [CompletionResultType]::ParameterValue, 'Remove a provider''s global and scoped bans')
            [CompletionResult]::new('vouch', 'vouch', [CompletionResultType]::ParameterValue, 'Vouch for a provider')
            [CompletionResult]::new('distrust', 'distrust', [CompletionResultType]::ParameterValue, 'Distrust a provider')
            break
        }
        'syncweb;help;trust;provider;show' {
            break
        }
        'syncweb;help;trust;provider;list' {
            break
        }
        'syncweb;help;trust;provider;ban' {
            break
        }
        'syncweb;help;trust;provider;unban' {
            break
        }
        'syncweb;help;trust;provider;vouch' {
            break
        }
        'syncweb;help;trust;provider;distrust' {
            break
        }
        'syncweb;help;trust;stream' {
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Subscribe to a provider trust stream ticket or file')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish a signed provider trust signal')
            break
        }
        'syncweb;help;trust;stream;subscribe' {
            break
        }
        'syncweb;help;trust;stream;publish' {
            break
        }
        'syncweb;help;attest' {
            break
        }
        'syncweb;help;report' {
            break
        }
        'syncweb;help;moderation' {
            [CompletionResult]::new('ls', 'ls', [CompletionResultType]::ParameterValue, 'List local moderation records')
            [CompletionResult]::new('hide', 'hide', [CompletionResultType]::ParameterValue, 'Hide a content record locally')
            break
        }
        'syncweb;help;moderation;ls' {
            break
        }
        'syncweb;help;moderation;hide' {
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
