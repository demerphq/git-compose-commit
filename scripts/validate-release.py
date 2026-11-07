#!/usr/bin/env python3
"""Validate the complete set of release archives without publishing."""
import hashlib
from pathlib import Path
import sys
import tarfile
import tomllib
import zipfile

root = Path(sys.argv[1])
with open("Cargo.toml", "rb") as manifest:
    version = tomllib.load(manifest)["package"]["version"]
targets = [
    "x86_64-unknown-linux-musl", "aarch64-unknown-linux-musl",
    "x86_64-apple-darwin", "aarch64-apple-darwin", "x86_64-pc-windows-msvc",
]
expected_files = set()
for target in targets:
    windows = target.endswith("-windows-msvc")
    name = f"git-compose-commit-v{version}-{target}." + ("zip" if windows else "tar.gz")
    archive = root / name
    checksum = root / (name + ".sha256")
    expected_files.update([name, checksum.name])
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    if checksum.read_text(encoding="ascii") != f"{digest}  {name}\n":
        raise SystemExit(f"invalid checksum: {name}")
    if windows:
        with zipfile.ZipFile(archive) as packed:
            members = set(packed.namelist())
        binary = "git-compose-commit.exe"
        text_binary = "cli-compose-text.exe"
    else:
        with tarfile.open(archive, "r:gz") as packed:
            members = set(packed.getnames())
            binary = "git-compose-commit"
            text_binary = "cli-compose-text"
            if not all(packed.getmember(name).mode & 0o111 for name in (binary, text_binary)):
                raise SystemExit(f"binary is not executable: {name}")
    if members != {binary, text_binary, "README.md", "LICENSE", "THIRD-PARTY-LICENSES.txt", "extract-agent-skills.py"}:
        raise SystemExit(f"unexpected archive contents: {name}: {members}")
if {path.name for path in root.iterdir()} != expected_files:
    raise SystemExit("unexpected or missing release artifacts")
print("Validated five archives and their SHA-256 checksums")
