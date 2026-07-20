# syncweb

CLI for [syncweb](https://github.com/chapmanjacobd/syncweb) -- delay-tolerant peer-to-peer file synchronization.

## Installation

```sh
cargo install syncweb
```

Or from source:

```sh
git clone https://github.com/chapmanjacobd/syncweb.git
cd syncweb
cargo install --path syncweb-cli
```

## Usage

```sh
# Create a folder and get a sharing ticket
syncweb create ~/my-folder

# Join a folder via ticket
syncweb join <ticket>

# List local folders
syncweb folders

# Show device identity (iroh NodeId + Syncthing DeviceId)
syncweb devices

# Show/set configuration
syncweb config
syncweb config set <key> <value>

# Test relay connectivity
syncweb network test-relay <relay-url>

# Interactive REPL
syncweb repl
```

## Commands

| Command | Description |
|---------|-------------|
| `create` | Create a folder + namespace, output a sharing ticket |
| `join` | Join a folder via ticket |
| `accept` | Accept/grant capability for a namespace |
| `drop` | Remove/revoke a namespace |
| `folders` | List local folders with sync modes |
| `devices` | Show device identity |
| `config` | Show/set configuration |
| `network test-relay` | Test Syncthing relay connectivity |
| `repl` | Interactive REPL |
| `completions` | Generate shell completions (bash, zsh, fish, elvish, powershell) |
| `manpages` | Generate man pages |

## Shell Completions

```sh
# Bash
syncweb completions bash > ~/.local/share/bash-completion/completions/syncweb

# Zsh
syncweb completions zsh > ~/.zfunc/_syncweb

# Fish
syncweb completions fish > ~/.config/fish/completions/syncweb.fish
```

## License

[MIT](https://opensource.org/licenses/MIT)
