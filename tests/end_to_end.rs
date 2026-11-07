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
        (
            "09-flow-diagram-branching",
            include_str!("fixtures/09-flow-diagram-branching.txt"),
        ),
        (
            "10-flow-diagram-horizontal",
            include_str!("fixtures/10-flow-diagram-horizontal.txt"),
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
        (
            "10-flow-diagram-no-edge",
            include_str!("fixtures/errors/10-flow-diagram-no-edge.txt"),
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
    assert!(short.contains("Create a commit:"));
    assert!(short.contains("Preview without committing:"));
    assert!(short.contains("Focus with --help=TOPIC"));
    assert!(!short.contains("--footer-N TEXT"));
    assert!(short.lines().all(|line| line.len() <= 80));

    let detailed = run_composer(repo.path(), &["--help"]);
    assert!(detailed.status.success());
    let detailed = String::from_utf8(detailed.stdout).expect("help output is UTF-8");
    assert!(detailed.contains("--footer-N TEXT"));
    assert!(detailed.contains("--hbar-chart DATA"));
    assert!(detailed.contains("--vbar-chart DATA"));
    assert!(detailed.contains("--hflow-chart DATA"));
    assert!(detailed.contains("--vflow-chart DATA"));
    assert!(detailed.contains("--flow-chart OPTIONS"));
    assert!(detailed.contains("--flow-edge EDGE"));
    assert!(detailed.contains("--output FILE"));
    assert!(detailed.contains("--bubble OPTIONS"));
    assert!(detailed.contains("--rand-seed N"));
    assert!(detailed.contains("--field KEY:VALUE"));
    assert!(detailed.contains("--note TEXT"));
    assert!(detailed.contains("--warning TEXT"));
    assert!(detailed.contains("--wrap-width N"));
    assert!(detailed.contains("CSV quoting and escaping"));
    assert!(detailed.contains("HOW IT WORKS"));
    assert!(detailed.contains("EXAMPLES"));
    assert!(detailed.contains("TABLES\n"));
    assert!(detailed.contains("LISTS\n"));
    assert!(detailed.contains("FOOTERS\n"));
    assert!(detailed.contains("BRANCHING FLOWCHARTS"));
    assert!(detailed.contains("SPEECH AND THOUGHT BUBBLES"));
    assert!(detailed.contains("speaker-side=SIDE   below (default)"));
    assert!(detailed.contains("--output /dev/stdout"));
    assert!(detailed.contains("--stdout"));
    assert!(detailed.contains("Backend: sqlite"));
    assert!(detailed.contains("( Start )--------------->( Done )"));
    assert!(detailed.lines().all(|line| line.len() <= 80));

    let forced_short = run_composer(repo.path(), &["--help=short"]);
    assert!(String::from_utf8_lossy(&forced_short.stdout).contains("Use --help"));
    let forced_long = run_composer(repo.path(), &["--help=long"]);
    assert!(String::from_utf8_lossy(&forced_long.stdout).contains("--table-rows ROWS"));
    assert_eq!(forced_long.stdout, detailed.as_bytes());
    let man = run_composer(repo.path(), &["--man"]);
    assert!(man.status.success());
    assert_eq!(man.stdout, detailed.as_bytes());

    let version = run_composer(repo.path(), &["--version"]);
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).expect("version output is UTF-8"),
        format!("git-compose-commit {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn help_topics_show_only_relevant_reference_and_examples() {
    let repo = TestRepo::new();
    for (topic, section, example) in [
        ("message", "MESSAGE OPTIONS", "3. Fields and a note"),
        ("output", "OUTPUT AND CONTROL", "2. Preview only"),
        ("tables", "TABLES", "4. Table"),
        ("lists", "LISTS", "5. Nested list"),
        ("footers", "FOOTERS", "6. Numbered footer"),
        ("charts", "BAR CHARTS", "7. Horizontal bar chart"),
        (
            "flowcharts",
            "BRANCHING FLOWCHARTS",
            "11. Branching flowchart",
        ),
        ("visuals", "VISUAL DECORATIONS", "15. Heading and badges"),
        ("bubbles", "SPEECH AND THOUGHT BUBBLES", "12. Speech bubble"),
    ] {
        let output = run_composer(repo.path(), &[&format!("--help={topic}")]);
        assert!(output.status.success(), "{topic}: {output:?}");
        let help = String::from_utf8(output.stdout).expect("topic help is UTF-8");
        assert!(help.contains(section), "{topic}: {help}");
        assert!(help.contains(example), "{topic}: {help}");
        assert!(help.lines().all(|line| line.len() <= 80));
    }

    let bubbles = run_composer(repo.path(), &["--help=bubbles"]);
    let bubbles = String::from_utf8(bubbles.stdout).unwrap();
    assert!(!bubbles.contains("--table-rows"));
    assert!(bubbles.contains("13. Thought bubble"));

    let charts = run_composer(repo.path(), &["--help=charts"]);
    let charts = String::from_utf8(charts.stdout).unwrap();
    assert!(charts.contains("8. Vertical bar chart"));

    let flowcharts = run_composer(repo.path(), &["--help=flowcharts"]);
    let flowcharts = String::from_utf8(flowcharts.stdout).unwrap();
    assert!(flowcharts.contains("10. Vertical flow chart"));

    let examples = run_composer(repo.path(), &["--help=examples"]);
    let examples = String::from_utf8(examples.stdout).unwrap();
    assert!(examples.contains("1. Basic commit"));
    assert!(examples.contains("8. Vertical bar chart"));
    assert!(examples.contains("10. Vertical flow chart"));
    assert!(examples.contains("14. Combine paragraphs"));
    assert!(!examples.contains("MESSAGE OPTIONS"));

    let unknown = run_composer(repo.path(), &["--help=unknown"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("unknown help topic 'unknown'"));
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

#[test]
fn writes_branching_ascii_flowchart_to_file_without_running_git() {
    let repo = TestRepo::new();
    let output_path = repo.path().join("preview.txt");
    let output_arg = output_path.to_str().expect("temporary path is UTF-8");
    let output = run_composer(
        repo.path(),
        &[
            "--subject=Validate uploads",
            "--flow-chart=direction=vertical;float=left;nodesep=0.8;ranksep=1.0",
            "--flow-terminal=start:Start",
            "--flow-node=read:Read input",
            "--flow-decision=valid:Valid?",
            "--flow-node=save:Save",
            "--flow-node=reject:Report error",
            "--flow-terminal=done:Done",
            "--flow-edge=start -> read",
            "--flow-edge=read -> valid",
            "--flow-edge=valid -> save [label=\"yes\"]",
            "--flow-edge=valid -> reject [label=\"no\"]",
            "--flow-edge=save -> done",
            "--flow-edge=reject -> done",
            "--compose-output",
            output_arg,
        ],
    );
    assert!(
        output.status.success(),
        "composer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let message = fs::read_to_string(&output_path).expect("read composed preview");
    assert!(message.starts_with("Validate uploads\n\n"));
    for label in [
        "Start",
        "Read input",
        "Valid?",
        "Save",
        "Report error",
        "Done",
    ] {
        assert!(message.contains(label), "missing {label:?} in {message:?}");
    }
    assert!(
        message.is_ascii(),
        "diagram output should be ASCII: {message:?}"
    );

    let head = Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .current_dir(repo.path())
        .output()
        .expect("check that output mode did not create a commit");
    assert!(!head.status.success());
}

#[test]
fn output_mode_rejects_git_commit_options() {
    let repo = TestRepo::new();
    let output_path = repo.path().join("preview.txt");
    let output_arg = output_path.to_str().expect("temporary path is UTF-8");
    let output = run_composer(
        repo.path(),
        &["--subject=Preview", "--output", output_arg, "--all"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("--output cannot be combined with Git commit options"));
    assert!(!output_path.exists());
}

#[test]
fn stdout_mode_prints_without_committing_and_rejects_git_options() {
    let repo = TestRepo::new();
    for option in ["--stdout", "--compose-stdout"] {
        let output = run_composer(
            repo.path(),
            &["--subject=Preview", "--text=No commit is made.", option],
        );
        assert!(output.status.success());
        assert_eq!(output.stdout, b"Preview\n\nNo commit is made.\n");
        assert!(output.stderr.is_empty());
    }

    let output = run_composer(repo.path(), &["--subject=Preview", "--stdout", "--all"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("--stdout cannot be combined with Git commit options"));

    let output = run_composer(
        repo.path(),
        &["--subject=Preview", "--stdout", "--output=preview.txt"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("--output and --stdout may only be specified once"));

    let output = run_composer(repo.path(), &["--subject=Preview", "--stdout=true"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--stdout does not take a value"));

    let head = Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .current_dir(repo.path())
        .output()
        .expect("check that stdout mode did not create a commit");
    assert!(!head.status.success());
}

#[test]
fn agent_skills_are_standalone_deterministic_and_extractable() {
    use std::io::Write;
    use std::process::Stdio;

    let repo = TestRepo::new();
    fs::write(repo.path().join(".compose-commit.toml"), "invalid = true\n").unwrap();
    let list = run_composer_with_home(repo.path(), repo.path(), &["--agent-usage-list-skills"]);
    assert!(list.status.success());
    assert!(list.stderr.is_empty());
    let names = std::str::from_utf8(&list.stdout)
        .unwrap()
        .lines()
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 10);
    let mut files = fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/skills"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    files.sort();
    let mut listed = names.clone();
    listed.sort();
    assert_eq!(files, listed);
    for name in &names {
        let option = format!("--agent-usage-show-skill={name}");
        let shown = run_composer(repo.path(), &[&option]);
        assert!(shown.status.success());
        let text = std::str::from_utf8(&shown.stdout).unwrap();
        assert!(text.starts_with(&format!("---\nname: {name}\ndescription: ")));
        assert!(text.contains("\n---\n"));
        assert!(shown.stderr.is_empty());
    }
    let all = run_composer(repo.path(), &["--agent-usage-show-skill=ALL"]);
    let repeated = run_composer(repo.path(), &["--agent-usage-show-skill=ALL"]);
    assert!(all.status.success());
    assert_eq!(all.stdout, repeated.stdout);
    let destination = repo.path().join("extracted");
    let mut child = Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/scripts/extract-agent-skills.py"
        ))
        .arg(&destination)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&all.stdout).unwrap();
    let extraction = child.wait_with_output().unwrap();
    assert!(
        extraction.status.success(),
        "{}",
        String::from_utf8_lossy(&extraction.stderr)
    );
    for name in &names {
        let actual = fs::read(destination.join(name).join("SKILL.md")).unwrap();
        let option = format!("--agent-usage-show-skill={name}");
        assert_eq!(actual, run_composer(repo.path(), &[&option]).stdout);
    }
    assert!(destination
        .join("git-compose-commit/scripts/extract-agent-skills.py")
        .exists());
    for args in [
        vec!["--agent-usage-show-skill=missing"],
        vec!["--agent-usage-show-skill="],
        vec!["--agent-usage-list-skills", "--stdout"],
        vec!["--agent-usage-unknown"],
    ] {
        let failure = run_composer(repo.path(), &args);
        assert!(!failure.status.success(), "{args:?}");
        assert!(failure.stdout.is_empty(), "{args:?}");
        assert!(!failure.stderr.is_empty(), "{args:?}");
    }
    let ordinary = run_composer(
        repo.path(),
        &["--subject", "--agent-usage-list-skills", "--stdout"],
    );
    assert!(ordinary.status.success());
    assert_eq!(ordinary.stdout, b"--agent-usage-list-skills\n");
}

#[test]
fn compose_text_writes_without_invoking_git_and_matches_wrapper() {
    let repo = TestRepo::new();
    let options = [
        "--subject",
        "Refresh cache",
        "--text",
        "Keep readers updated.",
    ];
    let text = Command::new(env!("CARGO_BIN_EXE_compose-text"))
        .args(options)
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(
        text.status.success(),
        "{}",
        String::from_utf8_lossy(&text.stderr)
    );
    let wrapper = run_composer(
        repo.path(),
        &[
            "--subject",
            "Refresh cache",
            "--text",
            "Keep readers updated.",
            "--stdout",
        ],
    );
    assert_eq!(text.stdout, wrapper.stdout);
    assert_eq!(run_git(repo.path(), &["status", "--porcelain"]).stdout, b"");
    let destination = repo.path().join("message.txt");
    let output = Command::new(env!("CARGO_BIN_EXE_compose-text"))
        .args(options)
        .arg("--output")
        .arg(&destination)
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read(destination).unwrap(), text.stdout);
    for extra in ["--all", "--signoff", "--", "path"] {
        let failure = Command::new(env!("CARGO_BIN_EXE_compose-text"))
            .args(options)
            .arg(extra)
            .current_dir(repo.path())
            .output()
            .unwrap();
        assert!(!failure.status.success(), "{extra}");
        assert!(failure.stdout.is_empty(), "{extra}");
    }
    let help = Command::new(env!("CARGO_BIN_EXE_compose-text"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&help.stdout).contains("without invoking Git"));
    let version = Command::new(env!("CARGO_BIN_EXE_compose-text"))
        .arg("--version")
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8(version.stdout).unwrap(),
        format!("compose-text {}\n", env!("CARGO_PKG_VERSION"))
    );
    let skills = Command::new(env!("CARGO_BIN_EXE_compose-text"))
        .arg("--agent-usage-list-skills")
        .output()
        .unwrap();
    assert_eq!(
        skills.stdout,
        run_composer(repo.path(), &["--agent-usage-list-skills"]).stdout
    );
    let literal = Command::new(env!("CARGO_BIN_EXE_compose-text"))
        .args(["--subject", "--stdout"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(literal.status.success());
    assert_eq!(literal.stdout, b"--stdout\n");
}
