use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_REPO: AtomicUsize = AtomicUsize::new(0);

struct TestRepo(PathBuf);

impl TestRepo {
    fn new() -> Self {
        let id = NEXT_REPO.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "git-compose-commit-e2e-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create temporary repository directory");
        run_git(&path, &["init", "--quiet"]);
        run_git(&path, &["config", "user.name", "Integration Test"]);
        run_git(
            &path,
            &["config", "user.email", "integration@example.invalid"],
        );
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn stage_sample_file(&self) {
        fs::write(self.path().join("sample.txt"), "sample content\n").expect("write staged file");
        run_git(self.path(), &["add", "sample.txt"]);
    }
}

impl Drop for TestRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run_git(repo: &Path, args: &[&str]) -> Output {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn run_composer(repo: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_git-compose-commit"))
        .args(args)
        .current_dir(repo)
        .output()
        .expect("run git-compose-commit")
}

fn run_composer_with_home(repo: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_git-compose-commit"))
        .args(args)
        .env("HOME", home)
        .current_dir(repo)
        .output()
        .expect("run git-compose-commit with isolated HOME")
}

fn committed_message(repo: &Path) -> String {
    let commit = run_git(repo, &["cat-file", "commit", "HEAD"]);
    let commit = String::from_utf8(commit.stdout).expect("commit object is UTF-8");
    commit
        .split_once("\n\n")
        .expect("commit has a message separator")
        .1
        .to_owned()
}

#[test]
fn golden_fixtures_match_full_commit_messages() {
    let fixtures = [
        ("01-basic", include_str!("fixtures/01-basic.txt")),
        ("02-table", include_str!("fixtures/02-table.txt")),
        (
            "03-lists-and-footers",
            include_str!("fixtures/03-lists-and-footers.txt"),
        ),
        (
            "04-complete-message",
            include_str!("fixtures/04-complete-message.txt"),
        ),
        ("05-bar-charts", include_str!("fixtures/05-bar-charts.txt")),
        (
            "06-flow-charts",
            include_str!("fixtures/06-flow-charts.txt"),
        ),
        ("07-bubbles", include_str!("fixtures/07-bubbles.txt")),
        (
            "08-fields-callouts-wrap",
            include_str!("fixtures/08-fields-callouts-wrap.txt"),
        ),
    ];

    for (name, fixture) in fixtures {
        let (arguments, expected) = fixture
            .split_once("\n\n")
            .unwrap_or_else(|| panic!("fixture {name} needs an argument/output separator"));
        let args = arguments.lines().collect::<Vec<_>>();
        let repo = TestRepo::new();
        let output = run_composer(repo.path(), &args);
        assert!(
            output.status.success(),
            "fixture {name} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(committed_message(repo.path()), expected, "fixture {name}");
    }
}

#[test]
fn invalid_option_fixtures_match_error_output() {
    let fixtures = [
        (
            "01-duplicate-subject",
            include_str!("fixtures/errors/01-duplicate-subject.txt"),
        ),
        (
            "02-row-without-header",
            include_str!("fixtures/errors/02-row-without-header.txt"),
        ),
        (
            "03-orphan-footer",
            include_str!("fixtures/errors/03-orphan-footer.txt"),
        ),
        (
            "04-chart-too-few-items",
            include_str!("fixtures/errors/04-chart-too-few-items.txt"),
        ),
        (
            "05-chart-settings-require-data",
            include_str!("fixtures/errors/05-chart-settings-require-data.txt"),
        ),
        (
            "06-flow-too-few-boxes",
            include_str!("fixtures/errors/06-flow-too-few-boxes.txt"),
        ),
        (
            "07-bubble-empty-phrase",
            include_str!("fixtures/errors/07-bubble-empty-phrase.txt"),
        ),
        (
            "08-field-missing-value",
            include_str!("fixtures/errors/08-field-missing-value.txt"),
        ),
        (
            "09-invalid-wrap-width",
            include_str!("fixtures/errors/09-invalid-wrap-width.txt"),
        ),
    ];

    for (name, fixture) in fixtures {
        let (arguments, expected_stderr) = fixture
            .split_once("\n\n")
            .unwrap_or_else(|| panic!("error fixture {name} needs a separator"));
        let args = arguments.lines().collect::<Vec<_>>();
        let repo = TestRepo::new();
        let output = run_composer(repo.path(), &args);
        assert_eq!(output.status.code(), Some(2), "fixture {name}");
        assert!(output.stdout.is_empty(), "fixture {name} wrote stdout");
        assert_eq!(
            String::from_utf8(output.stderr).expect("error output is UTF-8"),
            expected_stderr,
            "fixture {name}"
        );
    }
}

#[test]
fn help_switches_between_synopsis_and_detailed_guide() {
    let repo = TestRepo::new();
    let short = run_composer(repo.path(), &["-h"]);
    assert!(short.status.success());
    let short = String::from_utf8(short.stdout).expect("help output is UTF-8");
    assert!(short.contains("Usage: git-compose-commit"));
    assert!(short.contains("Use --help for the full option guide."));
    assert!(!short.contains("--footer-N TEXT"));

    let detailed = run_composer(repo.path(), &["--help"]);
    assert!(detailed.status.success());
    let detailed = String::from_utf8(detailed.stdout).expect("help output is UTF-8");
    assert!(detailed.contains("--footer-N TEXT"));
    assert!(detailed.contains("--hbar-chart DATA"));
    assert!(detailed.contains("--vbar-chart DATA"));
    assert!(detailed.contains("--hflow-chart DATA"));
    assert!(detailed.contains("--vflow-chart DATA"));
    assert!(detailed.contains("--bubble OPTIONS"));
    assert!(detailed.contains("--rand-seed N"));
    assert!(detailed.contains("--field KEY:VALUE"));
    assert!(detailed.contains("--note TEXT"));
    assert!(detailed.contains("--warning TEXT"));
    assert!(detailed.contains("--wrap-width N"));
    assert!(detailed.contains("CSV quoting and escaping"));

    let forced_short = run_composer(repo.path(), &["--help=short"]);
    assert!(String::from_utf8_lossy(&forced_short.stdout).contains("Use --help"));
    let forced_long = run_composer(repo.path(), &["--help=long"]);
    assert!(String::from_utf8_lossy(&forced_long.stdout).contains("--table-rows ROWS"));

    let version = run_composer(repo.path(), &["--version"]);
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).expect("version output is UTF-8"),
        format!("git-compose-commit {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn creates_commit_with_composed_sections_and_forwarded_git_options() {
    let repo = TestRepo::new();
    repo.stage_sample_file();

    let output = run_composer(
        repo.path(),
        &[
            "--compose-subject",
            "Exercise the full command",
            "--compose-text",
            "Review `[#99]` and details [#1].",
            "--compose-table-head",
            "Name,State",
            "--compose-table-rows",
            "cache,ready;worker,waiting",
            "--compose-list-item",
            "Refresh the cache",
            "--compose-list-item",
            "Verify the state",
            "--compose-line",
            "Refs: #42",
            "--compose-footer-1",
            "Footer context.",
            "--no-verify",
        ],
    );
    assert!(
        output.status.success(),
        "composer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        committed_message(repo.path()),
        "Exercise the full command\n\nReview `[#99]` and details [#1].\n\n+--------+---------+\n| Name   | State   |\n+--------+---------+\n| cache  | ready   |\n| worker | waiting |\n+--------+---------+\n\n1. Refresh the cache\n2. Verify the state\n\nRefs: #42\n\n[#1] Footer context.\n"
    );
}

#[test]
fn rejects_missing_footer_before_invoking_git_commit() {
    let repo = TestRepo::new();
    repo.stage_sample_file();

    let output = run_composer(
        repo.path(),
        &[
            "--compose-subject",
            "Reject broken reference",
            "--compose-text",
            "Missing [#7] footer.",
            "--allow-empty",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("no matching --compose-footer-7"));
    let head = Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .current_dir(repo.path())
        .output()
        .expect("check for a commit");
    assert!(!head.status.success());
}

#[test]
fn reads_user_defaults_and_cli_overrides_them() {
    let repo = TestRepo::new();
    let home = repo.path().join("home");
    fs::create_dir(&home).expect("create isolated home");
    fs::write(
        home.join(".compose-commit.toml"),
        concat!(
            "wrap_width = 32\n",
            "rand_seed = 123\n",
            "bubble_speaker = \"cat\"\n",
            "bubble_style = \"thought\"\n",
            "bubble_speaker_side = \"below\"\n",
            "bubble_side = \"center\"\n",
            "bubble_float = \"left\"\n",
        ),
    )
    .expect("write user config");

    let output = run_composer_with_home(
        repo.path(),
        &home,
        &[
            "--subject=Configured defaults",
            "--bubble=phrase=Hello",
            "--allow-empty",
        ],
    );
    assert!(
        output.status.success(),
        "composer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let message = committed_message(repo.path());
    assert!(message.contains("( o.o )"), "{message:?}");
    assert!(message.contains("( Hello )"));

    let repo = TestRepo::new();
    let output = run_composer_with_home(
        repo.path(),
        &home,
        &[
            "--subject=Command line wins",
            "--wrap-width=40",
            "--text=one two three four five six seven eight nine ten eleven twelve",
            "--bubble=speaker=dog;style=speech;speaker-side=below;bubble-side=center;float=left;phrase=Hello",
            "--allow-empty",
        ],
    );
    assert!(
        output.status.success(),
        "composer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let message = committed_message(repo.path());
    assert!(
        message.contains("one two three four five six seven eight\nnine ten eleven twelve"),
        "{message:?}"
    );
    assert!(message.contains("(    @\\___"));
    assert!(message.contains("Hello"));
}
