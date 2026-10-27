use super::*;
use std::ffi::OsString;

fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

fn message(values: &[&str]) -> String {
    compose(args(values)).unwrap().0
}

#[test]
fn formats_subject_and_wrapped_text() {
    assert_eq!(
        message(&[
            "--compose-text",
            "A paragraph with   extra spaces that should become one line.",
            "--compose-subject=Make output readable",
        ]),
        "Make output readable\n\nA paragraph with extra spaces that should become one line.\n"
    );
}

#[test]
fn wraps_long_text_at_the_commit_margin() {
    let text = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty";
    let wrapped = wrap_paragraph(text);
    assert!(wrapped
        .lines()
        .all(|line| line.chars().count() <= WRAP_WIDTH));
    assert_eq!(wrapped.replace('\n', " "), text);
}

#[test]
fn renders_aligned_ascii_table() {
    assert_eq!(
        message(&[
            "--compose-th=Name,State",
            "--compose-tr=cache,ready",
            "--compose-tr=worker,waiting",
        ]),
        "+--------+---------+\n| Name   | State   |\n+--------+---------+\n| cache  | ready   |\n| worker | waiting |\n+--------+---------+\n"
    );
}

#[test]
fn renders_semicolon_separated_table_rows() {
    assert_eq!(
        message(&["--table-head=A,B", "--table-rows=x,y;long,z"]),
        "+------+---+\n| A    | B |\n+------+---+\n| x    | y |\n| long | z |\n+------+---+\n"
    );
}

#[test]
fn supports_unprefixed_message_and_footer_aliases() {
    assert_eq!(
        message(&[
            "--subject=Alias support",
            "--text=See [#1] for context.",
            "--line=Refs: #2",
            "--footer-1=Additional context",
        ]),
        "Alias support\n\nSee [#1] for context.\n\nRefs: #2\n\n[#1] Additional context\n"
    );
}

#[test]
fn default_list_is_numbered_and_nested_list_can_return_to_parent() {
    assert_eq!(
        message(&[
            "--compose-list-item=first",
            "--compose-list-item=second",
            "--compose-list=bullets",
            "--compose-list-item=nested",
            "--compose-list-end",
            "--compose-list-item=third",
        ]),
        "1. first\n2. second\n  * nested\n3. third\n"
    );
}

#[test]
fn outputs_aligned_footers_and_ignores_backtick_references() {
    assert_eq!(
        message(&[
            "--compose-text=See `[#9]` and [#1] for details.",
            "--compose-footer-1=This explanation is intentionally long enough to wrap across multiple lines cleanly.",
        ]),
        "See `[#9]` and [#1] for details.\n\n[#1] This explanation is intentionally long enough to wrap across\n     multiple lines cleanly.\n"
    );
}

#[test]
fn rejects_missing_and_unreferenced_footers() {
    assert!(compose(args(&["--compose-text=Missing [#2] hook"]))
        .unwrap_err()
        .contains("no matching --compose-footer-2"));

    assert!(compose(args(&[
        "--compose-footer-2=orphan",
        "--compose-text=no reference",
    ]))
    .unwrap_err()
    .contains("no matching [#2] reference"));
}

#[test]
fn preserves_git_arguments_and_path_separator() {
    let (_, git_args) = compose(args(&[
        "--compose-subject",
        "subject",
        "--all",
        "--",
        "--compose-text",
    ]))
    .unwrap();
    assert_eq!(git_args, args(&["--all", "--", "--compose-text"]));
}
