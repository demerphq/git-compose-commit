#!/usr/bin/env python3
"""Generate a Homebrew formula from the validated release checksum files."""
from pathlib import Path
import re
import sys
import tomllib

root = Path(sys.argv[1])
with open("Cargo.toml", "rb") as manifest:
    version = tomllib.load(manifest)["package"]["version"]
targets = {
    "mac_intel": "x86_64-apple-darwin",
    "mac_arm": "aarch64-apple-darwin",
    "linux_intel": "x86_64-unknown-linux-musl",
    "linux_arm": "aarch64-unknown-linux-musl",
}
hashes = {}
for key, target in targets.items():
    name = f"git-compose-commit-v{version}-{target}.tar.gz"
    content = (root / (name + ".sha256")).read_text(encoding="ascii")
    match = re.fullmatch(r"([0-9a-f]{64})  " + re.escape(name) + r"\n", content)
    if not match:
        raise SystemExit(f"invalid checksum file for {name}")
    hashes[key] = match.group(1)
base = f"https://github.com/demerphq/git-compose-commit/releases/download/v{version}/git-compose-commit-v{version}"
print(f'''class GitComposeCommit < Formula
  desc "Compose structured Git commit messages"
  homepage "https://github.com/demerphq/git-compose-commit"
  version "{version}"
  license "MIT"

  on_macos do
    on_intel do
      url "{base}-{targets["mac_intel"]}.tar.gz"
      sha256 "{hashes["mac_intel"]}"
    end
    on_arm do
      url "{base}-{targets["mac_arm"]}.tar.gz"
      sha256 "{hashes["mac_arm"]}"
    end
  end

  on_linux do
    on_intel do
      url "{base}-{targets["linux_intel"]}.tar.gz"
      sha256 "{hashes["linux_intel"]}"
    end
    on_arm do
      url "{base}-{targets["linux_arm"]}.tar.gz"
      sha256 "{hashes["linux_arm"]}"
    end
  end

  def install
    bin.install "git-compose-commit", "cli-compose-text"
  end

  test do
    assert_match "git-compose-commit", shell_output("#{{bin}}/git-compose-commit --version")
  end
end''')
