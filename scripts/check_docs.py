#!/usr/bin/env python3
"""Check internal Markdown links and documented syncweb command names."""

from __future__ import annotations

import html
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
from urllib.parse import unquote, urlsplit


ROOT = Path(__file__).resolve().parents[1]
MARKDOWN_LINK = re.compile(r"!?\[[^\]]*\]\(([^)\n]+)\)")
REFERENCE_DEFINITION = re.compile(r"^\s{0,3}\[([^\]]+)\]:\s*(\S+)", re.MULTILINE)
REFERENCE_LINK = re.compile(r"!?\[[^\]]+\]\[([^\]]+)\]")
HEADING = re.compile(r"^\s{0,3}#{1,6}\s+(.+?)\s*#*\s*$")
COMMAND_LINE = re.compile(
    r"(?:^|[|;])\s*(?:[$#]\s*)?syncweb(?![-:])\b([^\n]*)"
)
COMMAND_AFTER_SEPARATOR = re.compile(
    r"[|;]\s*(?:[$#]\s*)?syncweb(?![-:])\b([^\n]*)"
)
INLINE_COMMAND = re.compile(r"`syncweb(?![-:])\b([^`]+)`")
COMMAND_NAME = re.compile(r"^[a-z][a-z0-9-]*$")

GLOBAL_OPTIONS_WITH_VALUES = {"--data-dir", "--network"}


def markdown_files() -> list[Path]:
    files = [ROOT / "README.md"]
    for directory in (ROOT / "docs", ROOT / "plans"):
        files.extend(directory.rglob("*.md"))
    return sorted(path for path in files if path.is_file())


def split_link_target(raw_target: str) -> tuple[str, str]:
    target = raw_target.strip()
    if target.startswith("<") and ">" in target:
        target = target[1 : target.index(">")]
    else:
        target = target.split(maxsplit=1)[0]
    parsed = urlsplit(target)
    return unquote(parsed.path), unquote(parsed.fragment)


def heading_slugs(path: Path) -> set[str]:
    slugs: set[str] = set()
    counts: dict[str, int] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        match = HEADING.match(line)
        if not match:
            continue
        heading = html.unescape(re.sub(r"`", "", match.group(1)).lower())
        heading = re.sub(r"[^\w\s-]", "", heading)
        heading = re.sub(r"\s+", "-", heading)
        count = counts.get(heading, 0)
        counts[heading] = count + 1
        slugs.add(heading if count == 0 else f"{heading}-{count}")
    return slugs


def check_links() -> list[str]:
    errors: list[str] = []
    for source in markdown_files():
        text = source.read_text(encoding="utf-8")
        links = [match.group(1) for match in MARKDOWN_LINK.finditer(text)]
        definitions = {
            match.group(1).casefold(): match.group(2)
            for match in REFERENCE_DEFINITION.finditer(text)
        }
        links.extend(
            definitions[label.casefold()]
            for label in REFERENCE_LINK.findall(text)
            if label.casefold() in definitions
        )

        for raw_target in links:
            parsed_target = urlsplit(raw_target.strip().strip("<").split(maxsplit=1)[0])
            if parsed_target.scheme or parsed_target.netloc:
                continue
            target, fragment = split_link_target(raw_target)
            if not target and not fragment:
                continue
            if not target:
                destination = source
            elif target.startswith(("/", "\\")):
                destination = ROOT / target.lstrip("/\\")
            else:
                destination = source.parent / target
            destination = Path(os.path.normpath(destination))

            if not destination.exists():
                errors.append(
                    f"{source.relative_to(ROOT)}: missing link target {raw_target!r}"
                )
                continue
            if fragment and destination.suffix.lower() == ".md":
                if fragment not in heading_slugs(destination):
                    errors.append(
                        f"{source.relative_to(ROOT)}: missing heading "
                        f"{fragment!r} in {destination.relative_to(ROOT)}"
                    )
    return errors


def command_names_from_help(help_text: str) -> list[str]:
    commands: list[str] = []
    in_commands = False
    for line in help_text.splitlines():
        stripped = line.strip()
        if re.fullmatch(r"[A-Za-z][A-Za-z &]+:", stripped):
            in_commands = stripped != "Options:"
            continue
        if in_commands:
            match = re.match(r"^\s{2,}([a-z][a-z0-9-]*)\s{2,}", line)
            if match:
                commands.append(match.group(1))
    return commands


def build_command_index(binary: Path) -> set[tuple[str, ...]]:
    subprocess.run(
        ["cargo", "build", "-q", "-p", "syncweb"],
        cwd=ROOT,
        check=True,
    )

    known: set[tuple[str, ...]] = {()}
    pending = [()]
    while pending:
        prefix = pending.pop()
        result = subprocess.run(
            [str(binary), *prefix, "--help"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        for command in command_names_from_help(result.stdout):
            path = (*prefix, command)
            if path not in known:
                known.add(path)
                pending.append(path)
    return known


def command_path(tokens: list[str], known: set[tuple[str, ...]]) -> tuple[str, ...]:
    words: list[str] = []
    index = 0
    while index < len(tokens):
        token = tokens[index]
        if token.startswith("-"):
            if token in GLOBAL_OPTIONS_WITH_VALUES:
                index += 1
            index += 1
            continue
        if not COMMAND_NAME.fullmatch(token):
            break
        words.append(token)
        index += 1

    for length in range(min(len(words), 4), 0, -1):
        candidate = tuple(words[:length])
        if candidate in known:
            return candidate
    return tuple(words[:2])


def accepts_command_path(binary: Path, path: tuple[str, ...]) -> bool:
    if not path:
        return False
    result = subprocess.run(
        [str(binary), *path, "--help"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return result.returncode == 0


def documented_command_references() -> list[tuple[Path, int, str]]:
    references: list[tuple[Path, int, str]] = []
    sources = [
        ROOT / "README.md",
        *(path for path in (ROOT / "docs").rglob("*.md") if path.name != "user-stories.md"),
    ]
    for source in sources:
        in_fence = False
        for line_number, line in enumerate(
            source.read_text(encoding="utf-8").splitlines(), start=1
        ):
            if line.strip().startswith("```"):
                in_fence = not in_fence
                continue
            matches = []
            if in_fence:
                matches.extend(match.group(1) for match in COMMAND_LINE.finditer(line))
            else:
                matches.extend(
                    match.group(1)
                    for match in COMMAND_AFTER_SEPARATOR.finditer(line)
                )
            matches.extend(
                match.group(1)
                for match in INLINE_COMMAND.finditer(line)
            )
            for remainder in matches:
                try:
                    tokens = shlex.split(remainder, comments=False)
                except ValueError:
                    continue
                references.append(
                    (source, line_number, "syncweb " + " ".join(tokens))
                )
    return references


def check_commands(binary: Path, known: set[tuple[str, ...]]) -> list[str]:
    errors: list[str] = []
    checked: dict[tuple[str, ...], bool] = {}
    for source, line_number, display in documented_command_references():
        remainder = display.removeprefix("syncweb ")
        try:
            tokens = shlex.split(remainder, comments=False)
        except ValueError:
            continue
        path = command_path(tokens, known)
        if path in known:
            continue
        valid = checked.get(path)
        if valid is None:
            valid = accepts_command_path(binary, path)
            checked[path] = valid
        if not valid:
            errors.append(
                f"{source.relative_to(ROOT)}:{line_number}: "
                f"unknown command {' '.join(path)!r}"
            )
    return errors


def main() -> int:
    binary = ROOT / "target" / "debug" / "syncweb"
    errors = check_links()
    known = build_command_index(binary)
    errors.extend(check_commands(binary, known))
    if errors:
        print("\n".join(errors), file=sys.stderr)
        print(f"{len(errors)} documentation check(s) failed", file=sys.stderr)
        return 1
    print(
        f"Documentation checks passed ({len(markdown_files())} Markdown files, "
        f"{len(known) - 1} command paths)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
