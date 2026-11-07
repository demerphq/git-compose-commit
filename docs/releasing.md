# Release preparation

Nothing in this document has been published yet. The repository includes a
manual GitHub Actions workflow that builds five native-runner targets: Linux
x86-64 and ARM64 (musl), macOS Intel and Apple Silicon, and Windows x86-64.
Graphviz remains an optional external dependency for branching flowcharts.
Each archive and installer includes both `compose-text` and
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
2. Run formatting, Clippy, tests and `cargo package --locked`. Confirm crate
   name availability and ownership on crates.io before attempting publication.
3. Run **Release artifacts** from GitHub Actions with `publish` unchecked.
   It builds the five targets, smoke-tests each binary, validates archive
   contents and SHA-256 checksums, and uploads a `validated-release` artifact.
   Inspect and test the artifacts on the supported platforms. No tag is needed
   for this dry run and no GitHub Release is created.
4. Inspect the generated `git-compose-commit.rb` formula and test it with
   Homebrew. It uses the same validated archives; Graphviz is not a mandatory
   dependency. The tap and its publication are intentionally not automated.

The workflow uses GitHub-hosted ARM Linux and Intel macOS runners; repository
availability and runner labels must be checked when the workflow is first run.
The shell and PowerShell installers use SHA-256 files from the same release as
the archives. Checksums detect transfer corruption but do not establish an
independent signature of the release. The binaries are not code-signed or
notarized; macOS and Windows may show security prompts. Arrange signing and
notarization separately if your distribution requires them.

## Publish the first release, when you choose

1. Merge the reviewed release commit into the intended release branch. Run the
   dry workflow above on that exact commit and inspect the artifacts.
2. Create and push an annotated `vVERSION` tag on that commit. The version
   must match `Cargo.toml`. Do this only when ready to publish.
3. Run **Release artifacts** on that commit with `publish` checked. The job
   verifies that the existing tag points to the checked-out commit, then
   creates the GitHub Release with archives, checksums, installers, and the
   generated formula. It does not create the tag or publish to crates.io.
4. After checking the GitHub Release, run `cargo publish --locked` with an
   authorized crates.io account. First verify the name is available and that
   you are authorized to publish the crate.
5. If desired, create `demerphq/homebrew-tap`, place the generated formula at
   `Formula/git-compose-commit.rb`, verify its checksums against the release,
   and test `brew install demerphq/tap/git-compose-commit`. Publish the tap
   separately. No tap is created by this repository.
6. Review the drafts in [announcements](announcements.md) and post only where
   and when you choose.

## Installation after publication

For a particular published version, download the corresponding installer from
the GitHub Release, inspect it, and run it. For example:

```sh
curl -fLO https://github.com/demerphq/git-compose-commit/releases/download/v0.1.0/install.sh
sh install.sh v0.1.0
```

```powershell
Invoke-WebRequest https://github.com/demerphq/git-compose-commit/releases/download/v0.1.0/install.ps1 -OutFile install.ps1
./install.ps1 -Version v0.1.0
```

The shell installer writes to `~/.local/bin` by default; set
`GIT_COMPOSE_COMMIT_INSTALL_DIR` to choose another directory. PowerShell
accepts `-InstallDir`. Both check the archive checksum and do not require
administrator rights or Rust. Ensure the chosen directory is on `PATH`.

Once published, `cargo install git-compose-commit` and
`cargo binstall git-compose-commit` are intended alternatives. The latter
uses the release archive naming in Cargo metadata. Confirm the actual first
publication before advertising any of these commands as available.
