
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
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Network name for scoped operations (uses data_dir/<network>/). Defaults to ''default'' if absent.')
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
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a synchronized folder and print a shareable URL')
            [CompletionResult]::new('join', 'join', [CompletionResultType]::ParameterValue, 'Join a folder from an Iroh document ticket')
            [CompletionResult]::new('leave', 'leave', [CompletionResultType]::ParameterValue, 'Leave a synchronized folder, optionally deleting its local files')
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
            [CompletionResult]::new('transfer', 'transfer', [CompletionResultType]::ParameterValue, 'Inspect and control durable transfer jobs')
            [CompletionResult]::new('watch', 'watch', [CompletionResultType]::ParameterValue, 'Watch a folder and import filesystem changes')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Show statistics for folders, files, and seeding status')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Re-check local folder blob integrity')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish a folder, blob, or catalog')
            [CompletionResult]::new('unpublish', 'unpublish', [CompletionResultType]::ParameterValue, 'Remove a public blob pin')
            [CompletionResult]::new('package', 'package', [CompletionResultType]::ParameterValue, 'Create, version, publish, and manage collection packages')
            [CompletionResult]::new('network', 'network', [CompletionResultType]::ParameterValue, 'Network connectivity utilities')
            [CompletionResult]::new('db', 'db', [CompletionResultType]::ParameterValue, 'Database maintenance: check, vacuum, stats, backup')
            [CompletionResult]::new('indexing', 'indexing', [CompletionResultType]::ParameterValue, 'Manage opt-in indexing, catalogs, and metadata')
            [CompletionResult]::new('link', 'link', [CompletionResultType]::ParameterValue, 'Create and resolve stable syncweb links')
            [CompletionResult]::new('mirror', 'mirror', [CompletionResultType]::ParameterValue, 'Mirror all blobs from a provider or network')
            [CompletionResult]::new('provider', 'provider', [CompletionResultType]::ParameterValue, 'Manage blob provider registrations')
            [CompletionResult]::new('trust', 'trust', [CompletionResultType]::ParameterValue, 'Inspect and delegate local trust')
            [CompletionResult]::new('attest', 'attest', [CompletionResultType]::ParameterValue, 'Sign content provenance attestations')
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
            [CompletionResult]::new('--beacon-port', '--beacon-port', [CompletionResultType]::ParameterName, 'Base UDP port the beacon spreads network scopes over')
            [CompletionResult]::new('--discovery-interface', '--discovery-interface', [CompletionResultType]::ParameterName, 'Restrict the beacon to a single network interface by name')
            [CompletionResult]::new('--media-listen', '--media-listen', [CompletionResultType]::ParameterName, 'Media HTTP server listen address (e.g. 127.0.0.1:9193)')
            [CompletionResult]::new('--bg', '--bg', [CompletionResultType]::ParameterName, 'Run in the background (daemon mode)')
            [CompletionResult]::new('--media-only', '--media-only', [CompletionResultType]::ParameterName, 'Run only the media HTTP server (standalone) and exit')
            [CompletionResult]::new('--no-relay', '--no-relay', [CompletionResultType]::ParameterName, 'Disable Iroh relay mode (no relay server connections)')
            [CompletionResult]::new('--no-mdns', '--no-mdns', [CompletionResultType]::ParameterName, 'Disable mDNS local peer discovery')
            [CompletionResult]::new('--no-beacon', '--no-beacon', [CompletionResultType]::ParameterName, 'Disable the UDP beacon local peer discovery')
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
        'syncweb;create' {
            [CompletionResult]::new('--mode', '--mode', [CompletionResultType]::ParameterName, 'Sync mode: sendreceive, receiveonly, or sendonly')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Add the created folder to a named network')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--relay-fallback', '--relay-fallback', [CompletionResultType]::ParameterName, 'Enable Syncthing relay fallback for this folder')
            [CompletionResult]::new('--import', '--import', [CompletionResultType]::ParameterName, 'Scan and import existing files in the directory')
            [CompletionResult]::new('--no-import', '--no-import', [CompletionResultType]::ParameterName, 'Skip scanning existing files in the directory')
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
            [CompletionResult]::new('--subscribe', '--subscribe', [CompletionResultType]::ParameterName, 'Track + enable live syncing (persisted subscribe-changes); idempotent on an existing folder')
            [CompletionResult]::new('--ingest-only', '--ingest-only', [CompletionResultType]::ParameterName, 'Only deliver entries ingested after live syncing is enabled')
            [CompletionResult]::new('--ignore-self', '--ignore-self', [CompletionResultType]::ParameterName, 'Ignore events emitted by this device''s own writes')
            [CompletionResult]::new('--download', '--download', [CompletionResultType]::ParameterName, 'Download matching existing content to the local folder after joining (one-shot; uses the same prefix/glob/max filters)')
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
            [CompletionResult]::new('--delete-files', '--delete-files', [CompletionResultType]::ParameterName, 'Also delete the folder''s local files')
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
            [CompletionResult]::new('schedule', 'schedule', [CompletionResultType]::ParameterValue, 'Show or update synchronization schedules')
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
        'syncweb;config;schedule' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Update the global schedule')
            [CompletionResult]::new('folder', 'folder', [CompletionResultType]::ParameterValue, 'Set schedule overrides for a named folder')
            break
        }
        'syncweb;config;schedule;set' {
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
        'syncweb;config;schedule;folder' {
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
            [CompletionResult]::new('-d', '-d', [CompletionResultType]::ParameterName, 'Exclude sendonly folders from search')
            [CompletionResult]::new('--download', '--download', [CompletionResultType]::ParameterName, 'Exclude sendonly folders from search')
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
            [CompletionResult]::new('--enrich', '--enrich', [CompletionResultType]::ParameterName, 'Query daemon for peer counts and frequency data to enrich niche/frecency/peers sorting')
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
            [CompletionResult]::new('--hash', '--hash', [CompletionResultType]::ParameterName, 'Content hash(es) to select (can repeat)')
            [CompletionResult]::new('--path-prefix', '--path-prefix', [CompletionResultType]::ParameterName, 'Only entries whose path starts with this prefix')
            [CompletionResult]::new('--glob', '--glob', [CompletionResultType]::ParameterName, 'Only entries whose path matches this glob pattern')
            [CompletionResult]::new('--from', '--from', [CompletionResultType]::ParameterName, 'Blob ticket(s) for providers (can repeat)')
            [CompletionResult]::new('--provider', '--provider', [CompletionResultType]::ParameterName, 'Blob ticket(s) for providers (can repeat)')
            [CompletionResult]::new('--min-providers', '--min-providers', [CompletionResultType]::ParameterName, 'Minimum providers for healthy replication')
            [CompletionResult]::new('--max-peers', '--max-peers', [CompletionResultType]::ParameterName, 'Fetch only blobs with at most N observed peers')
            [CompletionResult]::new('--min-peers', '--min-peers', [CompletionResultType]::ParameterName, 'Fetch only blobs with at least N observed peers')
            [CompletionResult]::new('--min-count', '--min-count', [CompletionResultType]::ParameterName, 'Minimum number of blobs to fetch')
            [CompletionResult]::new('--max-count', '--max-count', [CompletionResultType]::ParameterName, 'Maximum number of blobs to fetch')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'Copy threads (1 disables parallelism, 0 uses all available CPUs)')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--no-sharing', '--no-sharing', [CompletionResultType]::ParameterName, 'Do not share or seed downloaded content')
            [CompletionResult]::new('--no-seeding', '--no-seeding', [CompletionResultType]::ParameterName, 'Do not share or seed downloaded content')
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
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'Scanner threads (1 disables parallelism, 0 uses all available CPUs)')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--enrich', '--enrich', [CompletionResultType]::ParameterName, 'Query daemon for peer counts and frequency data to enrich niche/frecency/peers sorting')
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
        'syncweb;transfer' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'List durable transfer jobs')
            [CompletionResult]::new('remaining', 'remaining', [CompletionResultType]::ParameterValue, 'Show configured roots and remaining capacity')
            [CompletionResult]::new('root', 'root', [CompletionResultType]::ParameterValue, 'Add or update a materialization root')
            [CompletionResult]::new('enqueue', 'enqueue', [CompletionResultType]::ParameterValue, 'Enqueue an individually addressable file job')
            [CompletionResult]::new('allocate', 'allocate', [CompletionResultType]::ParameterValue, 'Allocate queued jobs to configured roots')
            [CompletionResult]::new('materialize', 'materialize', [CompletionResultType]::ParameterValue, 'Fetch and materialize assigned jobs through the daemon')
            [CompletionResult]::new('pause', 'pause', [CompletionResultType]::ParameterValue, 'Pause a transfer job')
            [CompletionResult]::new('resume', 'resume', [CompletionResultType]::ParameterValue, 'Resume a paused transfer job')
            [CompletionResult]::new('cancel', 'cancel', [CompletionResultType]::ParameterValue, 'Cancel a transfer job')
            [CompletionResult]::new('retry', 'retry', [CompletionResultType]::ParameterValue, 'Retry a failed transfer job')
            break
        }
        'syncweb;transfer;info' {
            [CompletionResult]::new('--namespace', '--namespace', [CompletionResultType]::ParameterName, 'Limit display to a namespace')
            [CompletionResult]::new('--state', '--state', [CompletionResultType]::ParameterName, 'Limit display to a lifecycle state')
            [CompletionResult]::new('--sort', '--sort', [CompletionResultType]::ParameterName, 'sort')
            [CompletionResult]::new('--group-by', '--group-by', [CompletionResultType]::ParameterName, 'group-by')
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
        'syncweb;transfer;remaining' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;transfer;root' {
            [CompletionResult]::new('--min-free', '--min-free', [CompletionResultType]::ParameterName, 'Free bytes to preserve on this root')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--disabled', '--disabled', [CompletionResultType]::ParameterName, 'Disable this root for allocation')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;transfer;enqueue' {
            [CompletionResult]::new('--namespace', '--namespace', [CompletionResultType]::ParameterName, 'namespace')
            [CompletionResult]::new('--path', '--path', [CompletionResultType]::ParameterName, 'Relative materialization path')
            [CompletionResult]::new('--hash', '--hash', [CompletionResultType]::ParameterName, '32-byte blob hash in hexadecimal')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;transfer;allocate' {
            [CompletionResult]::new('--namespace', '--namespace', [CompletionResultType]::ParameterName, 'Limit allocation to a namespace')
            [CompletionResult]::new('--path-prefix', '--path-prefix', [CompletionResultType]::ParameterName, 'Only allocate paths below this relative prefix')
            [CompletionResult]::new('--min-size', '--min-size', [CompletionResultType]::ParameterName, 'min-size')
            [CompletionResult]::new('--max-size', '--max-size', [CompletionResultType]::ParameterName, 'max-size')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Report allocations without persisting them')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;transfer;materialize' {
            [CompletionResult]::new('--namespace', '--namespace', [CompletionResultType]::ParameterName, 'Limit processing to a namespace')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;transfer;pause' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;transfer;resume' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;transfer;cancel' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;transfer;retry' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
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
            [CompletionResult]::new('--paths', '--paths', [CompletionResultType]::ParameterName, 'Paths evaluated by --dry-run')
            [CompletionResult]::new('--filters', '--filters', [CompletionResultType]::ParameterName, 'Filter configuration (defaults to DATA_DIR/filters.toml)')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--once', '--once', [CompletionResultType]::ParameterName, 'Process one event and exit')
            [CompletionResult]::new('--show-filters', '--show-filters', [CompletionResultType]::ParameterName, 'Print the active filter configuration and exit')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Evaluate paths against the filter rules without importing')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;stats' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('network', 'network', [CompletionResultType]::ParameterValue, 'Show persisted bandwidth accounting')
            [CompletionResult]::new('files', 'files', [CompletionResultType]::ParameterValue, 'Show file-level statistics for synced folder content')
            [CompletionResult]::new('seeding', 'seeding', [CompletionResultType]::ParameterValue, 'Show seeding status per folder blob')
            break
        }
        'syncweb;stats;network' {
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
        'syncweb;stats;files' {
            [CompletionResult]::new('--folder', '--folder', [CompletionResultType]::ParameterName, 'Namespace ID or path to a managed folder')
            [CompletionResult]::new('--by', '--by', [CompletionResultType]::ParameterName, 'by')
            [CompletionResult]::new('--top-largest', '--top-largest', [CompletionResultType]::ParameterName, 'Top N largest files by size')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;stats;seeding' {
            [CompletionResult]::new('--folder', '--folder', [CompletionResultType]::ParameterName, 'Namespace ID or path to a managed folder')
            [CompletionResult]::new('--hash', '--hash', [CompletionResultType]::ParameterName, 'Content hash(es) to select (can repeat)')
            [CompletionResult]::new('--path-prefix', '--path-prefix', [CompletionResultType]::ParameterName, 'Only entries whose path starts with this prefix')
            [CompletionResult]::new('--glob', '--glob', [CompletionResultType]::ParameterName, 'Only entries whose path matches this glob pattern')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;verify' {
            [CompletionResult]::new('--hash', '--hash', [CompletionResultType]::ParameterName, 'Content hash(es) to select (can repeat)')
            [CompletionResult]::new('--path-prefix', '--path-prefix', [CompletionResultType]::ParameterName, 'Only entries whose path starts with this prefix')
            [CompletionResult]::new('--glob', '--glob', [CompletionResultType]::ParameterName, 'Only entries whose path matches this glob pattern')
            [CompletionResult]::new('--from', '--from', [CompletionResultType]::ParameterName, 'Blob ticket(s) for providers (can repeat)')
            [CompletionResult]::new('--provider', '--provider', [CompletionResultType]::ParameterName, 'Blob ticket(s) for providers (can repeat)')
            [CompletionResult]::new('--min-providers', '--min-providers', [CompletionResultType]::ParameterName, 'Minimum providers for healthy replication')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--fix', '--fix', [CompletionResultType]::ParameterName, 'Attempt to repair corrupted blobs by re-downloading from peers')
            [CompletionResult]::new('--no-sharing', '--no-sharing', [CompletionResultType]::ParameterName, 'Do not share or seed downloaded content')
            [CompletionResult]::new('--no-seeding', '--no-seeding', [CompletionResultType]::ParameterName, 'Do not share or seed downloaded content')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;publish' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('folder', 'folder', [CompletionResultType]::ParameterValue, 'Publish a folder ticket for public read access')
            [CompletionResult]::new('blob', 'blob', [CompletionResultType]::ParameterValue, 'Publish a content hash as an unauthenticated blob ticket')
            [CompletionResult]::new('catalog', 'catalog', [CompletionResultType]::ParameterValue, 'Publish folder metadata to a catalog')
            break
        }
        'syncweb;publish;folder' {
            [CompletionResult]::new('--namespace', '--namespace', [CompletionResultType]::ParameterName, 'Namespace ID or managed folder path')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;publish;blob' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;publish;catalog' {
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
        'syncweb;package' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize one or more paths as a versioned package, scanning them in one shot')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Re-scan paths and update the local package manifest')
            [CompletionResult]::new('bump', 'bump', [CompletionResultType]::ParameterValue, 'Create a new package manifest version')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish a package manifest ticket and announce it to the catalog')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export one or more package directories as compressed CAR archive files')
            [CompletionResult]::new('import', 'import', [CompletionResultType]::ParameterValue, 'Import and install a compressed CAR archive file')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'List locally installed packages, optionally filtering by text')
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'Show a collection manifest from a ticket or blob hash')
            [CompletionResult]::new('install', 'install', [CompletionResultType]::ParameterValue, 'Verify, stage, and atomically install a collection version')
            [CompletionResult]::new('upgrade', 'upgrade', [CompletionResultType]::ParameterValue, 'Install a newer collection manifest version via ticket')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove a non-current installed collection version')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify an installed collection version')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List locally installed collections')
            [CompletionResult]::new('versions', 'versions', [CompletionResultType]::ParameterValue, 'List installed versions for a collection')
            [CompletionResult]::new('switch', 'switch', [CompletionResultType]::ParameterValue, 'Switch the active installed collection version')
            break
        }
        'syncweb;package;init' {
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'version')
            [CompletionResult]::new('--name', '--name', [CompletionResultType]::ParameterName, 'name')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Override the common root for logical path rebasing')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;add' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Override the common root for logical path rebasing')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;package;bump' {
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
        'syncweb;package;publish' {
            [CompletionResult]::new('--namespace', '--namespace', [CompletionResultType]::ParameterName, 'namespace')
            [CompletionResult]::new('--sequence', '--sequence', [CompletionResultType]::ParameterName, 'sequence')
            [CompletionResult]::new('--bootstrap', '--bootstrap', [CompletionResultType]::ParameterName, 'bootstrap')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Override the common root for logical path rebasing')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
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
            [CompletionResult]::new('--channel', '--channel', [CompletionResultType]::ParameterName, 'Search an editorial channel (uses catalog-backed persistence when the channel is configured in config.toml)')
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
            [CompletionResult]::new('--hash', '--hash', [CompletionResultType]::ParameterName, 'Blob hash of the manifest (requires --node-id)')
            [CompletionResult]::new('--node-id', '--node-id', [CompletionResultType]::ParameterName, 'Node ID hosting the manifest blob')
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
            [CompletionResult]::new('--path', '--path', [CompletionResultType]::ParameterName, 'path')
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
            [CompletionResult]::new('--path', '--path', [CompletionResultType]::ParameterName, 'path')
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
            [CompletionResult]::new('events', 'events', [CompletionResultType]::ParameterValue, 'Show recent network events')
            [CompletionResult]::new('health', 'health', [CompletionResultType]::ParameterValue, 'Show network connectivity health')
            [CompletionResult]::new('test-relay', 'test-relay', [CompletionResultType]::ParameterValue, 'Test a Syncthing relay TCP connection')
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
        'syncweb;network;events' {
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
        'syncweb;network;health' {
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'network')
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
        'syncweb;db' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('check', 'check', [CompletionResultType]::ParameterValue, 'Run integrity check on all databases')
            [CompletionResult]::new('vacuum', 'vacuum', [CompletionResultType]::ParameterValue, 'Run VACUUM to reclaim space in all databases')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Show database sizes and table statistics')
            [CompletionResult]::new('backup', 'backup', [CompletionResultType]::ParameterValue, 'Back up all databases to a directory')
            break
        }
        'syncweb;db;check' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;db;vacuum' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;db;stats' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;db;backup' {
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'output')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
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
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Search subscribed catalogs')
            [CompletionResult]::new('health', 'health', [CompletionResultType]::ParameterValue, 'Show verified provider health for a content hash')
            [CompletionResult]::new('meta', 'meta', [CompletionResultType]::ParameterValue, 'Manage signed metadata')
            [CompletionResult]::new('filter', 'filter', [CompletionResultType]::ParameterValue, 'Manage local and federated denylists')
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
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List signed metadata for a content hash')
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
        'syncweb;indexing;meta;list' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
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
            break
        }
        'syncweb;link;create' {
            [CompletionResult]::new('--name', '--name', [CompletionResultType]::ParameterName, 'name')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'version')
            [CompletionResult]::new('--sequence', '--sequence', [CompletionResultType]::ParameterName, 'sequence')
            [CompletionResult]::new('--expires', '--expires', [CompletionResultType]::ParameterName, 'Private-link expiration as a Unix timestamp')
            [CompletionResult]::new('--publish', '--publish', [CompletionResultType]::ParameterName, 'Namespace (folder) to publish the link into')
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
            [CompletionResult]::new('--broadcast', '--broadcast', [CompletionResultType]::ParameterName, 'Broadcast revocation to peers via gossip')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;mirror' {
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Network name or ID to mirror all blobs across')
            [CompletionResult]::new('--min-providers', '--min-providers', [CompletionResultType]::ParameterName, 'Minimum replication budget per blob (default 3)')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--no-sharing', '--no-sharing', [CompletionResultType]::ParameterName, 'Skip lease announcements after mirroring')
            [CompletionResult]::new('--no-seeding', '--no-seeding', [CompletionResultType]::ParameterName, 'Skip lease announcements after mirroring')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Report what would be mirrored without fetching')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
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
            [CompletionResult]::new('revoke-delegation', 'revoke-delegation', [CompletionResultType]::ParameterValue, 'Revoke a trust delegation')
            [CompletionResult]::new('provider', 'provider', [CompletionResultType]::ParameterValue, 'Manage provider trust and bans')
            [CompletionResult]::new('stream', 'stream', [CompletionResultType]::ParameterValue, 'Publish or subscribe to provider trust signals')
            break
        }
        'syncweb;trust;show' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--content', '--content', [CompletionResultType]::ParameterName, 'Treat the subject as a content hash rather than a publisher identity')
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
            [CompletionResult]::new('--max-depth', '--max-depth', [CompletionResultType]::ParameterName, 'Maximum delegation chain depth (1 = delegate only)')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;trust;revoke-delegation' {
            [CompletionResult]::new('--scope', '--scope', [CompletionResultType]::ParameterName, 'scope')
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
            [CompletionResult]::new('--broadcast', '--broadcast', [CompletionResultType]::ParameterName, 'Broadcast vouch via gossip trust stream')
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
            [CompletionResult]::new('--broadcast', '--broadcast', [CompletionResultType]::ParameterName, 'Broadcast distrust via gossip trust stream')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
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
        'syncweb;attest' {
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Sign and optionally broadcast a content attestation')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify attestations for content from the network')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List local attestations for a content hash')
            break
        }
        'syncweb;attest;create' {
            [CompletionResult]::new('--license', '--license', [CompletionResultType]::ParameterName, 'license')
            [CompletionResult]::new('--provenance', '--provenance', [CompletionResultType]::ParameterName, 'Provenance attestation type')
            [CompletionResult]::new('--derivative', '--derivative', [CompletionResultType]::ParameterName, 'Derivative work attestation type')
            [CompletionResult]::new('--sequence', '--sequence', [CompletionResultType]::ParameterName, 'sequence')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--broadcast', '--broadcast', [CompletionResultType]::ParameterName, 'Broadcast attestation via gossip')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;attest;verify' {
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Timeout in seconds for gossip collection')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'syncweb;attest;list' {
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
            [CompletionResult]::new('report', 'report', [CompletionResultType]::ParameterValue, 'Sign and submit a moderation report (broadcasts via gossip)')
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
        'syncweb;moderation;report' {
            [CompletionResult]::new('--reason', '--reason', [CompletionResultType]::ParameterName, 'Reason for the report')
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--broadcast', '--broadcast', [CompletionResultType]::ParameterName, 'Also broadcast to peers via gossip')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
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
            [CompletionResult]::new('--data-dir', '--data-dir', [CompletionResultType]::ParameterName, 'Directory used for persistent node identity and data')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose structured logging')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON where supported')
            [CompletionResult]::new('--no-daemon', '--no-daemon', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('--embedded', '--embedded', [CompletionResultType]::ParameterName, 'Bypass the daemon and use an embedded node for supported commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
