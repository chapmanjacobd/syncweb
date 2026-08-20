# Plan 003 — Files Coverage (ls / find / sort / stat / download / import / verify)

## Overview

The core file commands work, but each exposes a large option surface that is mostly untested. The
`find` engine and the shared `ContentFilter`/`ProviderSelector` flattened args are the biggest gaps.

## Target file

- `syncweb-cli/tests/cli_test.rs` (local filesystem tests)

## Missing coverage

### `ls`
- `--threads` (only `--sort` tested)

### `find` (only `--kind` tested)
- `--ignore-case` / `-i`, `--case-sensitive` / `-s`
- `--fixed-strings` / `-F`
- `--full-path` / `-p`
- `--hidden` / `-H`
- `--follow-links` / `-L`
- `--absolute-path` / `-a`
- `--download` / `-d` (exclude sendonly)
- `--depth` (`N`, `+N`, `-N`), `--min-depth`, `--max-depth`
- `--sizes` (`N`, `-N`, `+N`, `N%10`, `+5GB`), `--modified-within`, `--modified-before`, `--time-modified`
- `--extension` / `-e` (`--ext`, `--exts`, `--extensions` aliases)
- `--type` (`f`/`d`/`l`)
- `--threads`

### `sort` (only `--by` + `--enrich` tested)
- `--by` values: `time`, `date`, `week`, `month`, `year`, `size`, `folder-size`,
  `folder-avg-size`, `folder-date`, `folder-time`, `count`
- `--min-seeders`, `--max-seeders`, `--niche`, `--frecency-weight`, `--limit-size`
- `--depth`, `--min-depth`, `--max-depth`, `--threads`

### `stat` (only `--terse` tested)
- `--format <template>`
- `--threads`

### `download`
- `--hash`, `--path-prefix`, `--glob` (ContentFilter)
- `--from`, `--min-providers`, `--no-sharing` (ProviderSelector)
- `--max-peers`, `--min-peers`, `--min-count`, `--max-count`, `--threads`

### `import`
- `--folder`, `--threads`, `--enrich`

### `verify`
- `--fix`
- ContentFilter: `--hash`, `--path-prefix`, `--glob`
- ProviderSelector: `--from`, `--min-providers`, `--no-sharing`

## Proposed test cases

1. `find_case_sensitivity_and_fixed_strings`
   - `-i` matches case-insensitively; `-s` only exact case; `-F` treats `*.txt` literally (no match
     for a literal asterisk name).

2. `find_path_hidden_and_absolute`
   - `-p` matches full path; `-H` includes hidden files/dirs; `-a` prints absolute paths.

3. `find_depth_and_size_constraints`
   - `--depth -N` (max), `--min-depth`/`--max-depth`; `--sizes +N` / `-N` / `N%10`.

4. `find_extension_and_type`
   - `-e txt -e md`; `--type d` lists dirs only, `--type l` symlinks only.

5. `find_follow_links_and_downloadable`
   - `-L` follows symlinks; `-d` excludes sendonly folders.

6. `sort_additional_algorithms`
   - Loop over `time`, `date`, `week`, `month`, `year`, `size`, `folder-size`,
     `folder-avg-size`, `folder-date`, `folder-time`, `count`; each exits successfully and lists
     the expected files.

7. `sort_filters_and_scoring_tuning`
   - `--min-seeders`, `--max-seeders`, `--niche`, `--frecency-weight`, `--limit-size`,
     `--depth`/`--min-depth`/`--max-depth`, `--threads`.

8. `stat_format_template`
   - `stat --format '{size}'` renders the template without errors (and conflicts with `--terse`).

9. `download_filter_and_provider_options`
   - `download --hash <h>`, `--path-prefix`, `--glob`, `--from <ticket>`, `--min-providers`,
     `--no-sharing`, `--min-count`, `--max-count`, `--threads`; assert selected files land in dest.

10. `import_folder_threads_enrich`
    - `import --folder <ns>`, `--threads 1`, `--enrich` (warn + local fallback like `sort --enrich`).

11. `verify_fix_and_filters`
    - `verify --fix`, `verify --hash <h>`, `--path-prefix`, `--glob`; ProviderSelector flags.
