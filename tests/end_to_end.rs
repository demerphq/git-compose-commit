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

    let commit = run_git(repo.path(), &["cat-file", "commit", "HEAD"]);
    let commit = String::from_utf8(commit.stdout).expect("commit object is UTF-8");
    let message = commit
        .split_once("\n\n")
        .expect("commit has a message separator")
        .1;
    assert_eq!(
        message,
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
