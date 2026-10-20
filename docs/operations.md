# Daemon operations

The daemon owns the node identity, blob store, document store, watchers, and
IPC socket for one data directory. Run only one daemon for a given
`--data-dir` and `--network` pair.

## Operating modes

| Mode | Command | Ownership | Use |
|------|---------|-----------|-----|
| Foreground | `syncweb --data-dir /srv/syncweb start` | The invoking terminal or service manager | Development, containers, and supervised deployments |
| Detached convenience | `syncweb --data-dir /srv/syncweb start --bg` | A child process started by syncweb | Interactive shells when no service manager is available |
| Client command | `syncweb --data-dir /srv/syncweb status` | An already-running daemon | Read health and progress through IPC |

Foreground mode is the preferred mode for supervision. `start --bg` waits for
the IPC socket before reporting success, but it is not a service manager and
does not restart a daemon after an external kill. `stop --yes` requests a
graceful shutdown; add `--force` only when the daemon cannot finish normally.
The daemon also handles `SIGINT` and `SIGTERM` on Unix and shuts down its
watchers, sync intents, and IPC server before releasing the data directory.

Client commands reuse an already-running daemon and start one automatically
when needed. Use `--no-daemon` to force embedded operation. Read-only status
checks never start a daemon.

## Supported supervision

syncweb documents two native service-manager integrations:

* systemd user or system services on Linux. Keep the daemon in the
  foreground and let systemd restart it after an unexpected failure.
* launchd agents or daemons on macOS. Keep the daemon in the foreground
  and use `KeepAlive` for restart policy.

Other supervisors may run the same foreground command if they preserve the
data directory, forward `SIGTERM`, and capture stdout/stderr. syncweb does not
install unit files, register a Windows Service, or manage supervisor
configuration automatically.

### systemd

Create `~/.config/systemd/user/syncweb.service` for a user service. Replace
the binary and data-directory paths for the deployment:

```ini
[Unit]
Description=syncweb daemon
After=network-online.target
Wants=network-online.target

[Service]
ExecStart=/usr/local/bin/syncweb --data-dir %h/.local/share/syncweb start
ExecStop=/usr/local/bin/syncweb --data-dir %h/.local/share/syncweb --yes stop
Restart=on-failure
RestartSec=5s
KillSignal=SIGTERM
TimeoutStopSec=30s

[Install]
WantedBy=default.target
```

Enable and inspect it with:

```sh
systemctl --user daemon-reload
systemctl --user enable --now syncweb.service
systemctl --user status syncweb.service
```

For a headless account that must run without an interactive login, enable
lingering with the host's normal systemd policy before enabling the unit.
Use a system unit instead when the daemon should run as a dedicated service
account; do not run it as root unless the data directory requires it.

### launchd

Create `~/Library/LaunchAgents/com.syncweb.daemon.plist` with an absolute
binary and data-directory path:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.syncweb.daemon</string>
  <key>ProgramArguments</key>
  <array>
    <string>/usr/local/bin/syncweb</string>
    <string>--data-dir</string>
    <string>/Users/USER/Library/Application Support/syncweb</string>
    <string>start</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>/tmp/syncweb.stdout.log</string>
  <key>StandardErrorPath</key>
  <string>/tmp/syncweb.stderr.log</string>
</dict>
</plist>
```

Load or unload the agent using the user's launch domain:

```sh
launchctl bootstrap "gui/$(id -u)" ~/Library/LaunchAgents/com.syncweb.daemon.plist
launchctl print "gui/$(id -u)/com.syncweb.daemon"
launchctl bootout "gui/$(id -u)" ~/Library/LaunchAgents/com.syncweb.daemon.plist
```

Use `syncweb status` and `syncweb stop --yes` for application-level health
and graceful shutdown. If a supervisor reports a stale PID or an IPC version
mismatch, stop the old process, verify that its data directory is not in use,
and start the foreground command again; do not delete the PID or database
files while a process may still own the store.
