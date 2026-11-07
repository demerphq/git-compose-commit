#!/bin/sh
set -eu

usage='Usage: install.sh [--force] VERSION (for example v0.2.0)'
force=false
version=
for arg in "$@"; do
    case "$arg" in
        --force) force=true ;;
        v*) [ -z "$version" ] || { echo "$usage" >&2; exit 2; }; version=$arg ;;
        *) echo "$usage" >&2; exit 2 ;;
    esac
done
[ -n "$version" ] || { echo "$usage" >&2; exit 2; }
case "$version" in *[!A-Za-z0-9.-]*|v) echo "Invalid version" >&2; exit 2;; esac
case "$(uname -s):$(uname -m)" in
    Linux:x86_64) target=x86_64-unknown-linux-musl ;;
    Linux:aarch64|Linux:arm64) target=aarch64-unknown-linux-musl ;;
    Darwin:x86_64) target=x86_64-apple-darwin ;;
    Darwin:arm64) target=aarch64-apple-darwin ;;
    *) echo "Unsupported operating system or architecture" >&2; exit 1 ;;
esac

destination=${GIT_COMPOSE_COMMIT_INSTALL_DIR:-"$HOME/.local/bin"}
for name in git-compose-commit cli-compose-text; do
    existing="$destination/$name"
    if [ -L "$existing" ] || { [ -e "$existing" ] && [ ! -f "$existing" ]; }; then
        echo "Refusing to replace a symlink or non-file: $existing" >&2; exit 1
    fi
    if [ -e "$existing" ] && [ "$force" != true ]; then
        echo "Already exists: $existing (use --force to replace)" >&2; exit 1
    fi
    found=$(command -v "$name" 2>/dev/null || true)
    if [ -n "$found" ] && [ "$found" != "$existing" ]; then
        echo "Note: $name is already on PATH at $found; check PATH order." >&2
    fi
done

base=${GIT_COMPOSE_COMMIT_RELEASE_BASE:-https://github.com/demerphq/git-compose-commit/releases/download}
archive="git-compose-commit-$version-$target.tar.gz"
tempdir=$(mktemp -d)
stage=
lock=
installed_git=false
installed_text=false
complete=false
cleanup() {
    status=$?
    restore_failed=false
    trap - EXIT HUP INT TERM
    if [ "$complete" != true ] && [ -n "$stage" ]; then
        if [ "$installed_text" = true ]; then
            if [ -f "$stage/old-cli-compose-text" ]; then
                mv -f "$stage/old-cli-compose-text" "$destination/cli-compose-text" || restore_failed=true
            else
                rm -f "$destination/cli-compose-text"
            fi
        fi
        if [ "$installed_git" = true ]; then
            if [ -f "$stage/old-git-compose-commit" ]; then
                mv -f "$stage/old-git-compose-commit" "$destination/git-compose-commit" || restore_failed=true
            else
                rm -f "$destination/git-compose-commit"
            fi
        fi
    fi
    if [ "$restore_failed" = true ]; then
        echo "Could not restore all binaries; backups remain in $stage" >&2
        status=1
    else
        [ -z "$stage" ] || rm -rf "$stage"
    fi
    [ -z "$lock" ] || rmdir "$lock" 2>/dev/null || true
    rm -rf "$tempdir"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM

curl --fail --location --silent --show-error --connect-timeout 10 --max-time 120 "$base/$version/$archive" -o "$tempdir/$archive"
curl --fail --location --silent --show-error --connect-timeout 10 --max-time 120 "$base/$version/$archive.sha256" -o "$tempdir/checksum"
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
tar -xzf "$tempdir/$archive" -C "$tempdir" git-compose-commit cli-compose-text
[ -f "$tempdir/git-compose-commit" ] && [ -f "$tempdir/cli-compose-text" ] || {
    echo "Archive is missing a binary" >&2; exit 1
}

mkdir -p "$destination"
lock="$destination/.git-compose-commit-install.lock"
if ! mkdir "$lock"; then
    lock=
    echo "Another installation may be in progress" >&2; exit 1
fi
stage=$(mktemp -d "$destination/.git-compose-commit-install.XXXXXXXX")
for name in git-compose-commit cli-compose-text; do
    existing="$destination/$name"
    if [ -L "$existing" ] || { [ -e "$existing" ] && [ ! -f "$existing" ]; }; then
        echo "Refusing to replace a symlink or non-file: $existing" >&2; exit 1
    fi
    if [ -e "$existing" ]; then
        [ "$force" = true ] || { echo "Already exists: $existing (use --force to replace)" >&2; exit 1; }
        cp -p "$existing" "$stage/old-$name"
    fi
    install -m 755 "$tempdir/$name" "$stage/new-$name"
done
mv -f "$stage/new-git-compose-commit" "$destination/git-compose-commit"
installed_git=true
mv -f "$stage/new-cli-compose-text" "$destination/cli-compose-text"
installed_text=true
complete=true
echo "Installed git-compose-commit and cli-compose-text in $destination"
echo "Ensure $destination is on your PATH."
