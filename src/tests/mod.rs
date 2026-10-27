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
fn renders_horizontal_chart_with_scaling_and_alignment() {
    assert_eq!(
        message(&["--hbar-chart=float=left;width=10;data=test:10,reading:20",]),
        "test    | ##### 10\nreading | ########## 20\n"
    );
}

#[test]
fn keeps_positive_horizontal_values_visible_after_scaling() {
    assert_eq!(
        message(&["--hbar-chart=float=left;width=4;data=large:100,small:1"]),
        "large | #### 100\nsmall | # 1\n"
    );
}

#[test]
fn renders_horizontal_flow_chart_as_a_box_chain() {
    assert_eq!(
        message(&["--hflow-chart=float=left;data=Parse,Validate,Commit"]),
        concat!(
            "+----------+      +----------+      +----------+\n",
            "|  Parse   | --> | Validate | --> |  Commit  |\n",
            "+----------+      +----------+      +----------+\n",
        )
    );
}

#[test]
fn renders_vertical_flow_chart_as_a_box_chain() {
    assert_eq!(
        message(&["--vflow-chart=float=left;data=Parse,Validate"]),
        concat!(
            "+----------+\n",
            "|  Parse   |\n",
            "+----------+\n",
            "      |\n",
            "      v\n",
            "+----------+\n",
            "| Validate |\n",
            "+----------+\n",
        )
    );
}

#[test]
fn flow_chart_requires_data_clause_when_configured_and_two_boxes() {
    assert!(compose(args(&["--hflow-chart=float=right;Parse,Validate"]))
        .unwrap_err()
        .contains("requires data= when chart settings are specified"));
    assert_eq!(
        compose(args(&["--vflow-chart=only"])).unwrap_err(),
        "--vflow-chart requires at least two boxes"
    );
}

#[test]
fn renders_speech_and_thought_bubbles_with_each_speaker() {
    assert_eq!(
        message(&["--bubble=speaker=cat;style=voice;speaker-side=below;bubble-side=center;phrase=Mooooo;float=left"]),
        concat!(
            " .-------.\n",
            "( Mooooo )\n",
            " '-------'\n",
            "    /\n",
            "   /\n",
            "  /\\_/\\\n",
            " ( o.o )\n",
            "  > ^ <\n",
        )
    );
    let dog = message(&["--bubble=speaker=dog;style=voice;phrase=Mooooo;float=left"]);
    assert!(dog.contains("(    @\\___"));
    let person = message(&[
        "--bubble=speaker=stick-figure;style=voice;speaker-side=below;bubble-side=center;phrase=Hello;float=left",
    ]);
    assert!(person.contains("    O\n"));
    assert!(person.contains("   /|\\\n"));

    let thought = message(&[
        "--bubble=speaker=cat;style=thought;speaker-side=below;bubble-side=center;phrase=Hmm;float=left",
    ]);
    assert!(thought.contains("   O\n   o\n   .\n"));
}

#[test]
fn bubble_alignment_supports_left_center_and_right() {
    let left = message(&["--bubble=speaker=cat;style=voice;speaker-side=below;bubble-side=center;phrase=Hi;float=left"]);
    let centered = message(&["--bubble=speaker=cat;style=voice;speaker-side=below;bubble-side=center;phrase=Hi;float=center"]);
    let right = message(&["--bubble=speaker=cat;style=voice;speaker-side=below;bubble-side=center;phrase=Hi;float=right"]);
    assert!(left.starts_with(" .---."));
    assert!(centered.starts_with(&format!("{} .---.", " ".repeat(32))));
    assert!(right.starts_with(&format!("{} .---.", " ".repeat(65))));
}

#[test]
fn bubble_speaker_can_sit_to_either_side() {
    assert_eq!(
        message(&["--bubble=speaker=cat;style=voice;phrase=Hi;speaker-side=left;bubble-side=center;float=left"]),
        concat!(
            " /\\_/\\       .---.\n",
            "( o.o ) <-- ( Hi )\n",
            " > ^ <       '---'\n",
        )
    );
    assert!(
        message(&["--bubble=speaker=cat;style=voice;phrase=Hi;speaker-side=right;bubble-side=center;float=left"])
            .contains("( Hi ) --> ( o.o )")
    );
}

#[test]
fn bubble_side_shifts_the_bubble_over_the_speaker() {
    let left = message(&[
        "--bubble=speaker=stick-figure;style=voice;phrase=Hi;speaker-side=below;bubble-side=left;float=left",
    ]);
    let center = message(&[
        "--bubble=speaker=stick-figure;style=voice;phrase=Hi;speaker-side=below;bubble-side=center;float=left",
    ]);
    let right = message(&[
        "--bubble=speaker=stick-figure;style=voice;phrase=Hi;speaker-side=below;bubble-side=right;float=left",
    ]);
    assert!(left.contains("      O\n"));
    assert!(center.contains("   O\n"));
    assert!(right.contains("  O\n"));
}

#[test]
fn bubble_requires_speaker_and_phrase() {
    assert_eq!(
        compose(args(&["--bubble=speaker=cat"])).unwrap_err(),
        "--bubble requires phrase=..."
    );
    assert_eq!(
        compose(args(&["--bubble=speaker=fox;phrase=Hello"])).unwrap_err(),
        "--bubble speaker must be 'cat', 'dog', or 'stick-figure'"
    );
    assert!(message(&["--bubble=phrase=Hello"]).contains("Hello"));
}

#[test]
fn random_seed_reproduces_choices_and_is_not_forwarded_to_git() {
    let first = compose(args(&[
        "--rand-seed",
        "12345",
        "--bubble=phrase=Hello",
        "--allow-empty",
    ]))
    .unwrap();
    let second = compose(args(&[
        "--bubble=phrase=Hello",
        "--allow-empty",
        "--compose-rand-seed=12345",
    ]))
    .unwrap();
    assert_eq!(first, second);
    assert_eq!(first.1, args(&["--allow-empty"]));
}

#[test]
fn rejects_invalid_and_duplicate_random_seeds() {
    assert_eq!(
        compose(args(&["--rand-seed=nope"])).unwrap_err(),
        "--rand-seed must be an unsigned integer"
    );
    assert_eq!(
        compose(args(&["--rand-seed=1", "--compose-rand-seed=2"])).unwrap_err(),
        "--rand-seed may only be specified once"
    );
}

#[test]
fn renders_fields_callouts_and_custom_wrap_width() {
    assert_eq!(
        message(&[
            "--wrap-width=32",
            "--text=A long paragraph that will wrap at the requested margin.",
            "--field=Backend:sqlite",
            "--field=Endpoint:https://example.test/api/path",
            "--note=Keep existing configuration files compatible with this change.",
            "--warning=This migration removes the old index after validation.",
        ]),
        concat!(
            "A long paragraph that will wrap\n",
            "at the requested margin.\n\n",
            "Backend : sqlite\n",
            "Endpoint: https://example.test/api/path\n\n",
            "Note: Keep existing\n",
            "      configuration files\n",
            "      compatible with this\n",
            "      change.\n\n",
            "Warning: This migration removes\n",
            "         the old index after\n",
            "         validation.\n",
        )
    );
}

#[test]
fn rejects_invalid_wrap_width_and_field_syntax() {
    assert_eq!(
        compose(args(&["--wrap-width=19"])).unwrap_err(),
        "--wrap-width must be an integer from 20 to 256"
    );
    assert_eq!(
        compose(args(&["--field=missing-separator"])).unwrap_err(),
        "--field requires KEY:VALUE"
    );
    assert_eq!(
        compose(args(&["--wrap-width=72", "--compose-wrap-width=80"])).unwrap_err(),
        "--wrap-width may only be specified once"
    );
}

#[test]
fn parses_toml_user_defaults_and_rejects_duplicate_keys() {
    let path = std::env::temp_dir().join(format!(
        "git-compose-commit-config-{}-{}.toml",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(
        &path,
        concat!(
            "# User formatting defaults\n",
            "wrap_width = 68\n",
            "rand_seed = 123\n",
            "bubble_speaker = \"cat\"\n",
            "bubble_style = \"thought\"\n",
            "bubble_speaker_side = \"below\"\n",
            "bubble_side = \"center\"\n",
            "bubble_float = \"left\"\n",
        ),
    )
    .expect("write config fixture");
    let config = load_config_from(&path).expect("parse config fixture");
    assert_eq!(config.wrap_width, Some(68));
    assert_eq!(config.rand_seed, Some(123));
    assert_eq!(config.bubble_speaker, Some(BubbleSpeaker::Cat));
    assert_eq!(config.bubble_style, Some(BubbleStyle::Thought));
    assert_eq!(config.bubble_speaker_side, Some(SpeakerSide::Below));
    assert_eq!(config.bubble_side, Some(BubbleSide::Center));
    assert_eq!(config.bubble_float, Some(ChartAlignment::Left));

    std::fs::write(&path, "wrap_width = 68\nwrap_width = 72\n")
        .expect("write duplicate key fixture");
    let error = load_config_from(&path).unwrap_err();
    assert!(error.contains("duplicate setting wrap_width"));
    std::fs::remove_file(path).expect("remove config fixture");
}

#[test]
fn renders_vertical_chart_with_height_and_default_center_alignment() {
    assert_eq!(
        message(&["--vbar-chart=height=4;data=test:10,reading:20,writing:1"]),
        concat!(
            "                      20 |            #\n",
            "                         |            #\n",
            "                         |    #       #\n",
            "                         |    #       #       #\n",
            "                       0 +------------------------\n",
            "                            test   reading writing\n",
        )
    );
}

#[test]
fn bare_chart_data_defaults_to_centering() {
    assert_eq!(
        message(&["--hbar-chart=test:1,reading:2"]),
        concat!(
            "                  test    | ############ 1\n",
            "                  reading | ######################## 2\n",
        )
    );
}

#[test]
fn rejects_chart_with_fewer_than_two_data_items() {
    assert_eq!(
        compose(args(&["--vbar-chart=test:1"])).unwrap_err(),
        "--vbar-chart requires at least two data items"
    );
    assert!(compose(args(&["--hbar-chart=height=4;data=test:1,read:2"]))
        .unwrap_err()
        .contains("uses width= for its bar scale"));
    assert!(compose(args(&["--vbar-chart=height=4;test:1,read:2"]))
        .unwrap_err()
        .contains("requires data= when chart settings are specified"));
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
fn numlist_and_bullets_aliases_select_list_styles() {
    assert_eq!(
        message(&[
            "--numlist",
            "--list-item=first",
            "--bullets",
            "--list-item=nested",
            "--list-end",
            "--compose-list-item=second",
        ]),
        "1. first\n  * nested\n2. second\n"
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
