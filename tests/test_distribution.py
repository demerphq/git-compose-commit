"""Local tests for packaging and installers. No network or publication."""
import os
from pathlib import Path
import platform
import re
import subprocess
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
VERSION = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
BINARY = ROOT / "target/debug/git-compose-commit"
TEXT_BINARY = ROOT / "target/debug/cli-compose-text"
TARGETS = [
    "x86_64-unknown-linux-musl", "aarch64-unknown-linux-musl",
    "x86_64-apple-darwin", "aarch64-apple-darwin", "x86_64-pc-windows-msvc",
]


class DistributionTests(unittest.TestCase):
    def test_documented_skill_install_does_not_overwrite(self):
        guide = (ROOT / "docs/agents.md").read_text()
        section = guide.split("## Install skill documents", 1)[1]
        script = re.search(r"```sh\n(.*?)\n```", section, re.S).group(1)
        with tempfile.TemporaryDirectory() as tmp:
            env = {**os.environ, "PATH": str(ROOT / "target/debug") + os.pathsep + os.environ["PATH"]}
            result = subprocess.run(["sh", "-c", script], cwd=tmp, env=env, capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            files = list((Path(tmp) / ".agents/skills").glob("*/SKILL.md"))
            self.assertEqual(len(files), 11)
            before = {path: path.read_bytes() for path in files}
            result = subprocess.run(["sh", "-c", script], cwd=tmp, env=env, capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(before, {path: path.read_bytes() for path in files})
        with tempfile.TemporaryDirectory() as tmp:
            env = {**os.environ, "PATH": str(ROOT / "target/debug") + os.pathsep + os.environ["PATH"]}
            existing = Path(tmp) / ".agents/skills/cli-compose-bubbles/SKILL.md"
            existing.parent.mkdir(parents=True)
            existing.write_text("keep this")
            result = subprocess.run(["sh", "-c", script], cwd=tmp, env=env, capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(existing.read_text(), "keep this")
            self.assertEqual(list((Path(tmp) / ".agents/skills").glob("*/SKILL.md")), [existing])

    def test_artifacts_and_shell_installer(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = root / "releases" / ("v" + VERSION)
            for target in TARGETS:
                subprocess.run(["python3", "scripts/package-release.py", "--version", VERSION,
                                "--target", target, "--binary", str(BINARY),
                                "--text-binary", str(TEXT_BINARY), "--out", str(dist)],
                               cwd=ROOT, check=True, capture_output=True)
            subprocess.run(["python3", "scripts/validate-release.py", str(dist)],
                           cwd=ROOT, check=True, capture_output=True)
            formula = subprocess.run(["python3", "scripts/render-homebrew.py", str(dist)],
                                     cwd=ROOT, check=True, capture_output=True, text=True).stdout
            self.assertEqual(formula.count("sha256 "), 4)
            if platform.system() != "Linux" or platform.machine() != "x86_64":
                self.skipTest("shell installer smoke test needs Linux x86-64")
            dest = root / "bin"
            env = {**os.environ, "GIT_COMPOSE_COMMIT_RELEASE_BASE": (root / "releases").as_uri(),
                   "GIT_COMPOSE_COMMIT_INSTALL_DIR": str(dest)}
            command = ["sh", "scripts/install.sh", "v" + VERSION]
            subprocess.run(command, cwd=ROOT, env=env, check=True, capture_output=True)
            installed = dest / "git-compose-commit"
            self.assertEqual(installed.read_bytes(), BINARY.read_bytes())
            self.assertEqual((dest / "cli-compose-text").read_bytes(), TEXT_BINARY.read_bytes())
            conflict = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, text=True)
            self.assertNotEqual(conflict.returncode, 0)
            self.assertIn("use --force", conflict.stderr)
            self.assertEqual(installed.read_bytes(), BINARY.read_bytes())
            subprocess.run(command + ["--force"], cwd=ROOT, env=env, check=True, capture_output=True)

            partial = root / "partial"
            partial.mkdir()
            (partial / "git-compose-commit").write_bytes(b"existing")
            partial_env = {**env, "GIT_COMPOSE_COMMIT_INSTALL_DIR": str(partial)}
            conflict = subprocess.run(command, cwd=ROOT, env=partial_env, capture_output=True)
            self.assertNotEqual(conflict.returncode, 0)
            self.assertEqual((partial / "git-compose-commit").read_bytes(), b"existing")
            self.assertFalse((partial / "cli-compose-text").exists())

            link_dir = root / "links"
            link_dir.mkdir()
            (link_dir / "git-compose-commit").symlink_to(partial / "git-compose-commit")
            link_env = {**env, "GIT_COMPOSE_COMMIT_INSTALL_DIR": str(link_dir)}
            linked = subprocess.run(command + ["--force"], cwd=ROOT, env=link_env, capture_output=True)
            self.assertNotEqual(linked.returncode, 0)
            self.assertEqual((partial / "git-compose-commit").read_bytes(), b"existing")

            installed.write_bytes(b"old git")
            (dest / "cli-compose-text").write_bytes(b"old text")
            shim_dir = root / "shim"
            shim_dir.mkdir()
            shim = shim_dir / "mv"
            shim.write_text('#!/bin/sh\ncase "$*" in *new-cli-compose-text*) exit 1;; esac\nexec /bin/mv "$@"\n')
            shim.chmod(0o755)
            failure_env = {**env, "PATH": str(shim_dir) + os.pathsep + env["PATH"]}
            failed_upgrade = subprocess.run(command + ["--force"], cwd=ROOT, env=failure_env, capture_output=True)
            self.assertNotEqual(failed_upgrade.returncode, 0)
            self.assertEqual(installed.read_bytes(), b"old git")
            self.assertEqual((dest / "cli-compose-text").read_bytes(), b"old text")
            subprocess.run(command + ["--force"], cwd=ROOT, env=env, check=True, capture_output=True)
            archive = dist / f"git-compose-commit-v{VERSION}-x86_64-unknown-linux-musl.tar.gz"
            archive.write_bytes(archive.read_bytes() + b"tampered")
            invalid = subprocess.run(["python3", "scripts/validate-release.py", str(dist)],
                                     cwd=ROOT, capture_output=True)
            self.assertNotEqual(invalid.returncode, 0)
            failure = subprocess.run(command + ["--force"], cwd=ROOT, env=env, capture_output=True, text=True)
            self.assertNotEqual(failure.returncode, 0)
            self.assertIn("Checksum mismatch", failure.stderr)
            self.assertEqual(installed.read_bytes(), BINARY.read_bytes())

    def test_extractor_rejects_traversal(self):
        with tempfile.TemporaryDirectory() as tmp:
            for stream in [b"AGENT-USAGE/1\n8 1\n../x.txtX0 0\n",
                           b"AGENT-USAGE/1\n5 10\na.txtshort",
                           b"AGENT-USAGE/1\n5 1\na.txtX5 1\na.txtY0 0\n"]:
                result = subprocess.run(["python3", "scripts/extract-agent-skills.py", tmp],
                                        cwd=ROOT, input=stream, capture_output=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((Path(tmp) / "a.txt").exists())


if __name__ == "__main__":
    unittest.main()
