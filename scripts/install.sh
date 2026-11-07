#!/bin/sh
set -eu
version=${1:?Usage: install.sh VERSION (for example v0.1.0)}
case "$version" in v*) ;; *) echo "Version must start with v" >&2; exit 2;; esac
case "$version" in *[!A-Za-z0-9.-]*|v) echo "Invalid version" >&2; exit 2;; esac
case "$(uname -s):$(uname -m)" in
    Linux:x86_64) target=x86_64-unknown-linux-musl ;;
    Linux:aarch64|Linux:arm64) target=aarch64-unknown-linux-musl ;;
    Darwin:x86_64) target=x86_64-apple-darwin ;;
    Darwin:arm64) target=aarch64-apple-darwin ;;
    *) echo "Unsupported operating system or architecture" >&2; exit 1 ;;
esac
base=${GIT_COMPOSE_COMMIT_RELEASE_BASE:-https://github.com/demerphq/git-compose-commit/releases/download}
archive="git-compose-commit-$version-$target.tar.gz"
tempdir=$(mktemp -d)
trap 'rm -rf "$tempdir"' EXIT HUP INT TERM
curl --fail --location --silent --show-error "$base/$version/$archive" -o "$tempdir/$archive"
curl --fail --location --silent --show-error "$base/$version/$archive.sha256" -o "$tempdir/checksum"
expected=$(awk '{print $1}' "$tempdir/checksum")
case "$expected" in *[!0-9a-fA-F]*|'') echo "Invalid checksum" >&2; exit 1;; esac
[ "${#expected}" -eq 64 ] || { echo "Invalid checksum length" >&2; exit 1; }
if command -v sha256sum >/dev/null 2>&1; then
    actual=$(sha256sum "$tempdir/$archive" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
    actual=$(shasum -a 256 "$tempdir/$archive" | awk '{print $1}')
else
    echo "sha256sum or shasum is required" >&2; exit 1
fi
[ "$actual" = "$expected" ] || { echo "Checksum mismatch" >&2; exit 1; }
tar -xzf "$tempdir/$archive" -C "$tempdir" git-compose-commit compose-text
destination=${GIT_COMPOSE_COMMIT_INSTALL_DIR:-"$HOME/.local/bin"}
mkdir -p "$destination"
install -m 755 "$tempdir/git-compose-commit" "$destination/git-compose-commit"
install -m 755 "$tempdir/compose-text" "$destination/compose-text"
echo "Installed git-compose-commit and compose-text in $destination"
echo "Ensure $destination is on your PATH."
