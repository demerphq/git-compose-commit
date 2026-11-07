# Release preparation

The repository includes a manual GitHub Actions workflow that builds five
native-runner targets: Linux x86-64 and ARM64 (musl), macOS Intel and Apple
Silicon, and Windows x86-64.
Graphviz remains an optional external dependency for branching flowcharts.
Each archive and installer includes both `cli-compose-text` and
`git-compose-commit`.

## Why this workflow

[dist](https://axodotdev.github.io/cargo-dist/book/) can generate builds,
installers, checksums and Homebrew publication. This repository uses a small
manual workflow instead: it keeps the release step opt-in and produces the
same archives for the installers, cargo-binstall, and the Homebrew formula.
No tap credentials or automatic publishing are configured.

## Validate before publishing

1. Update the version in `Cargo.toml` and `Cargo.lock`, review the license
   attribution and release notes, and commit the changes.
2. Run formatting, Clippy, tests and `cargo package --locked`. Confirm the
   publishing account has ownership of the crate on crates.io.
3. Run **Release artifacts** from GitHub Actions with `publish` unchecked.
   It builds the five targets, smoke-tests each binary, validates archive
   contents and SHA-256 checksums, and uploads a `validated-release` artifact.
   It tests both installers, the cargo-binstall mappings, and the generated
   Homebrew formula against local copies of the artifacts. The Homebrew test
   uses a temporary runner-local tap. Inspect the artifacts as well. No tag is
   needed for this dry run and no GitHub Release is created.
4. Inspect the generated `git-compose-commit.rb` formula. It uses the same
   validated archives; Graphviz is not a mandatory dependency. Publishing the
   real tap is intentionally not automated.

The workflow uses GitHub-hosted ARM Linux and Intel macOS runners.
The shell and PowerShell installers use SHA-256 files from the same release as
the archives. Checksums detect transfer corruption but do not establish an
independent signature of the release. The binaries are not code-signed or
notarized; macOS and Windows may show security prompts. Arrange signing and
notarization separately if your distribution requires them.

## Publish a release

1. Merge the reviewed release commit into the intended release branch. Run the
   dry workflow above on that exact commit and inspect the artifacts.
2. Create and push an annotated `vVERSION` tag on that commit. The version
   must match `Cargo.toml`. Do this only when ready to publish.
3. Run **Release artifacts** on that commit with `publish` checked. The job
   verifies that the existing tag points to the checked-out commit, then
   creates the GitHub Release with archives, checksums, installers, and the
   generated formula. It does not create the tag or publish to crates.io.
4. After checking the GitHub Release, run `cargo publish --locked` with an
   authorized crates.io account.
5. Place the generated formula in the existing `demerphq/homebrew-tap`
   repository at `Formula/git-compose-commit.rb`, verify its checksums against
   the release, and push it to the tap. Then test
   `brew install demerphq/tap/git-compose-commit`. Updates are not automated.
6. Review the drafts in [announcements](announcements.md) and post only where
   and when you choose.

## Installation after publication

For a particular published version, download the corresponding installer from
the GitHub Release, inspect it, and run it. For example:

```sh
curl -fLO https://github.com/demerphq/git-compose-commit/releases/download/v0.2.1/install.sh
sh install.sh v0.2.1
```

```powershell
Invoke-WebRequest https://github.com/demerphq/git-compose-commit/releases/download/v0.2.1/install.ps1 -OutFile install.ps1
./install.ps1 -Version v0.2.1
```

The shell installer writes to `~/.local/bin` by default; set
`GIT_COMPOSE_COMMIT_INSTALL_DIR` to choose another directory. PowerShell
accepts `-InstallDir`. Both check the archive checksum and do not require
administrator rights or Rust. They refuse to replace existing executables by
default; pass `--force` to the shell installer or `-Force` to PowerShell for a
deliberate upgrade. They stage both files, restore earlier files if a replacement
fails, and warn about other executables found on `PATH`. A machine failure can
still interrupt an upgrade. Ensure the chosen directory is on `PATH`.

`cargo install git-compose-commit --locked` and
`cargo binstall git-compose-commit` are alternatives. The latter uses the
release archive naming in Cargo metadata.
