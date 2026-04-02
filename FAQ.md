# Frequently Asked Questions

## Installation

### How do I install Syncweb?

You can install using `go install`:

```sh
go install -tags noassets github.com/chapmanjacobd/syncweb/cmd/syncweb@latest
```

Or you can build from source:

```sh
git clone https://github.com/chapmanjacobd/syncweb.git
cd syncweb
make build
```

The binary will be available at `./syncweb`.

### How do I install syncweb-automatic?

Run the installation script:

```sh
curl -s https://raw.githubusercontent.com/chapmanjacobd/syncweb/refs/heads/main/examples/install.sh | bash
```

This installs a systemd user service that automatically accepts devices and folders.

## Usage

### How do I create a new syncweb folder?

```sh
syncweb create ./path/to/folder
```

This will output a sync URL like `sync://folder#DEVICEID` that you can share with others.

### How do I join an existing syncweb folder?

```sh
syncweb join sync://folder#DEVICEID
```

### How do I accept a device?

```sh
syncweb accept DEVICEID
```

To also share folders with the device:

```sh
syncweb accept --folders=folder1,folder2 DEVICEID
```

### How do I find files?

Use the `find` command with various filters:

```sh
# Find files by name
syncweb find Test

# Find files by type and extension
syncweb find -tf -eMKA

# Find files by size
syncweb find -S-20M  # Files smaller than 20MB
syncweb find -S+1G   # Files larger than 1GB

# Combine filters
syncweb find -tf -eMKA -S-20M -d=+2 Test
```

### How do I download files?

First, create a list of files to download:

```sh
syncweb find -tf -eMKA -S-20M | syncweb sort "balanced,frecency" > download_list.txt
```

Then download:

```sh
syncweb download --yes < download_list.txt
```

### How do I start the web UI?

```sh
syncweb serve
```

Then open http://localhost:8889 in your browser.

### What is the REPL?

The REPL (Read-Eval-Print Loop) is an interactive debugging mode that provides direct access to Syncthing API commands. Start it with:

```sh
syncweb repl
```

Type `help` to see available commands.

## Troubleshooting

### Syncthing temporary files

Syncthing's default is to [remove partial transfers](https://docs.syncthing.net/users/config.html#config-option-options.keeptemporariesh) when rescanning after 24 hours have passed since the transfer attempt but in Syncweb I have it set to 8 days.

If people are running low on disk space we could make a button somewhere which finds and deletes '.syncthing\.*\.tmp' among other things.

### "Folder path missing" error

This error occurs when Syncthing cannot find the folder path. Make sure:

1. The folder exists
2. The path is accessible
3. You have read/write permissions

To fix:

```sh
# Create the folder if it doesn't exist
mkdir -p /path/to/folder

# Or rejoin the folder
syncweb join sync://folder#DEVICEID
```

### Device not connecting

Check:

1. Both devices are running Syncweb/Syncthing
2. Firewall allows Syncthing traffic (default port 22000)
3. Device IDs are correctly entered
4. Devices have been accepted on both sides

### "No pending devices" when running automatic

The `syncweb-automatic` daemon only accepts devices marked as "local" by default. To accept all devices:

```sh
syncweb automatic --devices --local=false
```

## Development

### How do I build from source?

```sh
make build
```

For development with hot-reload:

```sh
make dev
```

### How do I run tests?

```sh
make test
```

### How do I contribute?

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `make test`
5. Submit a pull request

## REST API

### What is the REST Engine?

The REST Engine allows CLI commands to communicate with a running Syncweb server via HTTP REST API. This is useful when:

- You have `syncweb serve` already running
- You want to manage Syncweb remotely
- Multiple CLI instances need to coordinate through a central server

### How does the fallback mechanism work?

When you run a CLI command (e.g., `syncweb ls`), it first tries to acquire a lock on the Syncweb home directory. If the lock fails (because `syncweb serve` is running), it automatically falls back to the REST API.

The CLI reads the server address and API token from:
- `<home>/syncweb.addr` - Server address (e.g., `http://127.0.0.1:8889`)
- `<home>/syncweb.token` - API token for authentication

### What operations are supported via REST API?

The REST Engine supports all major operations:

**Folder Management:**
- List, add, delete folders
- Pause/resume folders
- Scan folder subdirectories
- Join/leave folder devices
- Get folder stats and completion

**Device Management:**
- List, add, delete devices
- Pause/resume devices
- Set device addresses

**File Operations:**
- Browse files (ls, find, stat)
- Download files
- Manage ignore patterns

**Monitoring:**
- Get events
- Check idle status
- Get pending devices/folders

### How do I use the REST API directly?

You can call the REST API directly using curl or any HTTP client:

```bash
# List folders
curl -H "X-Syncweb-Token: YOUR_TOKEN" \
  http://localhost:8889/api/syncweb/folders

# Pause a folder
curl -X POST -H "X-Syncweb-Token: YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"id": "folder-id"}' \
  http://localhost:8889/api/syncweb/folders/pause

# Get events
curl -H "X-Syncweb-Token: YOUR_TOKEN" \
  http://localhost:8889/api/syncweb/events
```

### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/syncweb/status` | GET | Get server status |
| `/api/syncweb/folders` | GET | List all folders |
| `/api/syncweb/folders/add` | POST | Add a new folder |
| `/api/syncweb/folders/delete` | POST | Delete a folder |
| `/api/syncweb/folders/pause` | POST | Pause a folder |
| `/api/syncweb/folders/resume` | POST | Resume a folder |
| `/api/syncweb/folders/join` | POST | Join a folder with a device |
| `/api/syncweb/folders/remove-devices` | POST | Remove devices from folder |
| `/api/syncweb/folders/scan-subdirs` | POST | Scan specific subdirectories |
| `/api/syncweb/devices` | GET | List all devices |
| `/api/syncweb/devices/add` | POST | Add a new device |
| `/api/syncweb/devices/delete` | POST | Delete a device |
| `/api/syncweb/devices/pause` | POST | Pause a device |
| `/api/syncweb/devices/resume` | POST | Resume a device |
| `/api/syncweb/devices/set-addresses` | POST | Set device addresses |
| `/api/syncweb/ls` | GET | List files in folder |
| `/api/syncweb/find` | GET | Search for files |
| `/api/syncweb/stat` | GET | Get file metadata |
| `/api/syncweb/download` | POST | Trigger file download |
| `/api/syncweb/ignores` | GET/POST | Get/set ignore patterns |
| `/api/syncweb/ignores/add` | POST | Add ignore patterns |
| `/api/syncweb/events` | GET | Get recent events |
| `/api/syncweb/pending` | GET | Get pending devices |
| `/api/syncweb/pending-folders` | GET | Get pending folders |
| `/api/syncweb/completion` | GET | Get folder completion |
| `/api/syncweb/idle` | GET | Check if folder is idle |
| `/api/syncweb/need` | GET | Get needed files |
| `/api/syncweb/remote-need` | GET | Get remote device needed files |
| `/api/syncweb/local-changed` | GET | Get locally changed files |
| `/api/syncweb/tree` | GET | Get folder tree structure |
