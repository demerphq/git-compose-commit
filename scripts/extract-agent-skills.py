#!/usr/bin/env python3
"""Extract an agent-usage v1 stream from stdin into a destination directory."""
import argparse
import pathlib
import re
import sys


def read_exact(stream, size):
    data = stream.read(size)
    if len(data) != size:
        raise ValueError("truncated stream")
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination", type=pathlib.Path)
    args = parser.parse_args()
    stream = sys.stdin.buffer
    if stream.readline() != b"AGENT-USAGE/1\n":
        raise ValueError("invalid stream header")
    files = {}
    while True:
        header = stream.readline(100)
        if not re.fullmatch(rb"(0|[1-9][0-9]*) (0|[1-9][0-9]*)\n", header):
            raise ValueError("invalid record header")
        path_size, data_size = map(int, header.split())
        if path_size == 0 and data_size == 0:
            if stream.read(1):
                raise ValueError("trailing data")
            break
        if not 0 < path_size <= 4096 or data_size > 16 * 1024 * 1024:
            raise ValueError("record size out of range")
        path = read_exact(stream, path_size).decode("utf-8")
        parts = path.split("/")
        if any(not re.fullmatch(r"[a-zA-Z0-9_.-]+", p) or p in (".", "..") for p in parts):
            raise ValueError("unsafe path")
        if path in files:
            raise ValueError("duplicate path")
        files[path] = read_exact(stream, data_size)
    root = args.destination
    root.mkdir(parents=True, exist_ok=True)
    for path in files:
        target = root.joinpath(*path.split("/"))
        if target.exists() or target.is_symlink():
            raise ValueError(f"destination already exists: {target}")
        for parent in target.parents:
            if parent == root.parent:
                break
            if parent.is_symlink():
                raise ValueError(f"symlink in destination: {parent}")
    for path, data in files.items():
        target = root.joinpath(*path.split("/"))
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, UnicodeError) as error:
        print(f"extract-agent-skills: {error}", file=sys.stderr)
        sys.exit(1)
