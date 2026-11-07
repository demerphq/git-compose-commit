"""Local tests for packaging and installers. No network or publication."""
import os
from pathlib import Path
import platform
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
            archive = dist / f"git-compose-commit-v{VERSION}-x86_64-unknown-linux-musl.tar.gz"
            archive.write_bytes(archive.read_bytes() + b"tampered")
            invalid = subprocess.run(["python3", "scripts/validate-release.py", str(dist)],
                                     cwd=ROOT, capture_output=True)
            self.assertNotEqual(invalid.returncode, 0)
            failure = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, text=True)
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
