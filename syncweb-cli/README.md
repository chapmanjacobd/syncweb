# syncweb CLI

CLI for [syncweb](https://github.com/chapmanjacobd/syncweb) -- delay-tolerant peer-to-peer file synchronization.

## Install

```sh
cargo install --locked syncweb
```

Or from source:

```sh
git clone https://github.com/chapmanjacobd/syncweb.git
cd syncweb
cargo install --path syncweb-cli
```

## Usage

```sh
syncweb create ~/my-folder        # create a folder + ticket
syncweb join <ticket>             # join a folder
syncweb folders                   # list local folders
syncweb devices                   # show identity info
syncweb config                    # show/set configuration
syncweb network test-relay <url>  # test relay connectivity
syncweb repl                      # interactive REPL
```

## Shell Completions

```sh
syncweb completions bash > ~/.local/share/bash-completion/completions/syncweb
syncweb completions zsh  > ~/.zfunc/_syncweb
syncweb completions fish > ~/.config/fish/completions/syncweb.fish
```

## License

MIT
