#!/usr/bin/env python3
"""Create one versioned archive and SHA-256 file from a built executable."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import tarfile
import zipfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--version", required=True)
parser.add_argument("--target", required=True)
parser.add_argument("--binary", type=Path, required=True)
parser.add_argument("--text-binary", type=Path, required=True)
parser.add_argument("--out", type=Path, required=True)
args = parser.parse_args()
if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?", args.version):
    parser.error("invalid version")
if not re.fullmatch(r"[a-zA-Z0-9_-]+", args.target):
    parser.error("invalid target")
windows = args.target.endswith("-windows-msvc")
binary_name = "git-compose-commit.exe" if windows else "git-compose-commit"
text_binary_name = "compose-text.exe" if windows else "compose-text"
if not args.binary.is_file() or not args.text_binary.is_file():
    parser.error("both binaries must exist")
args.out.mkdir(parents=True, exist_ok=True)
extension = "zip" if windows else "tar.gz"
archive = args.out / f"git-compose-commit-v{args.version}-{args.target}.{extension}"
files = [(args.binary, binary_name), (args.text_binary, text_binary_name), (Path("scripts/extract-agent-skills.py"), "extract-agent-skills.py"), (Path("README.md"), "README.md"), (Path("LICENSE"), "LICENSE")]
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--locked", "--offline", "--format-version", "1"], text=True
))
notices = []
for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
    if package["source"] is None:
        continue
    directory = Path(package["manifest_path"]).parent
    licenses = sorted(path for path in directory.iterdir() if path.is_file() and
                      path.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "UNLICENSE")))
    if not licenses:
        raise SystemExit(f"no license text found for {package['name']} {package['version']}")
    for license_path in licenses:
        notices.append(f"\n--- {package['name']} {package['version']}: {license_path.name} ---\n\n")
        notices.append(license_path.read_text(encoding="utf-8"))
        notices.append("\n")
third_party = "".join(notices).encode("utf-8")
if windows:
    with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as z:
        for source, name in files:
            z.write(source, name)
        z.writestr("THIRD-PARTY-LICENSES.txt", third_party)
else:
    with tarfile.open(archive, "w:gz") as tar:
        for source, name in files:
            info = tar.gettarinfo(str(source), arcname=name)
            info.mode = 0o755 if name in (binary_name, text_binary_name) else 0o644
            with source.open("rb") as data:
                tar.addfile(info, data)
        info = tarfile.TarInfo("THIRD-PARTY-LICENSES.txt")
        info.size = len(third_party)
        info.mode = 0o644
        tar.addfile(info, io.BytesIO(third_party))
digest = hashlib.sha256(archive.read_bytes()).hexdigest()
(archive.parent / (archive.name + ".sha256")).write_text(f"{digest}  {archive.name}\n", encoding="ascii")
print(archive)
