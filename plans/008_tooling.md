# Plan 008 — Tooling Coverage (manpages / help / completions / version)

## Overview

Shell completions for all four shells and `version` are well tested. `manpages`, the `help`
subcommand, and the bare `config` invocation are not.

## Target file

- `syncweb-cli/tests/cli_test.rs`
- `syncweb-cli/tests/full_suite_test.rs`

## Missing coverage

### `manpages`
- `manpages <dir>` — entire command untested
- Options: default `dir` is `man`; no flags

### `help`
- `help <command>` subcommand — untested (only `--help` tested)
- Because `disable_help_subcommand = true` is set in `args.rs:245`, verify the help subcommand is
  actually disabled at the top level and only reachable via `--help`.

### `completions`
- All four shells tested (bash/zsh/fish/powershell); no gaps.

### `version`
- Tested (`--json` + plain).

## Proposed test cases

1. `manpages_generate_markdown`
   - `manpages <tmpdir>`; assert the directory is created and contains `.1` files (e.g.
     `syncweb.1`) matching the command set.

2. `manpages_default_directory`
   - `manpages` (default `man` dir) succeeds and writes files.

3. `help_subcommand_behavior`
   - Confirm whether `help` is disabled (top-level `--help` covers it); if reachable, assert
     `help version` prints version help.

4. `completions_reference_subcommands`
   - (Optional hardening) assert generated completions mention a representative subcommand such as
     `create` for each shell.
