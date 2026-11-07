//! Side-effect-free discovery of embedded Agent Skills.
use std::ffi::OsString;
use std::io::{self, Write};

const SKILLS: &[(&str, &str)] = &[
    (
        "git-compose-commit",
        include_str!("../skills/git-compose-commit/SKILL.md"),
    ),
    (
        "git-compose-message",
        include_str!("../skills/git-compose-message/SKILL.md"),
    ),
    (
        "git-compose-output",
        include_str!("../skills/git-compose-output/SKILL.md"),
    ),
    (
        "git-compose-tables",
        include_str!("../skills/git-compose-tables/SKILL.md"),
    ),
    (
        "git-compose-lists",
        include_str!("../skills/git-compose-lists/SKILL.md"),
    ),
    (
        "git-compose-footers",
        include_str!("../skills/git-compose-footers/SKILL.md"),
    ),
    (
        "git-compose-charts",
        include_str!("../skills/git-compose-charts/SKILL.md"),
    ),
    (
        "git-compose-flowcharts",
        include_str!("../skills/git-compose-flowcharts/SKILL.md"),
    ),
    (
        "git-compose-visuals",
        include_str!("../skills/git-compose-visuals/SKILL.md"),
    ),
    (
        "git-compose-bubbles",
        include_str!("../skills/git-compose-bubbles/SKILL.md"),
    ),
];

const EXTRACTOR: &str = include_str!("../scripts/extract-agent-skills.py");

fn record(out: &mut Vec<u8>, path: &str, body: &str) {
    out.extend_from_slice(format!("{} {}\n", path.len(), body.len()).as_bytes());
    out.extend_from_slice(path.as_bytes());
    out.extend_from_slice(body.as_bytes());
}

/// Return None when these are normal CLI arguments.
pub fn handle(args: &[OsString]) -> Option<Result<(), String>> {
    let is_agent_option = |arg: &OsString| {
        arg.to_str()
            .is_some_and(|s| s.starts_with("--agent-usage-"))
    };
    let mut index = 0;
    let mut found = false;
    while index < args.len() && args[index] != "--" {
        let arg = &args[index];
        if is_agent_option(arg) {
            found = true;
            break;
        }
        if super::git_option_takes_value(arg)
            || super::COMPOSE_VALUE_OPTIONS.iter().any(|name| arg == *name)
            || super::is_dynamic_footer_option(arg)
            || arg == "--output"
            || arg == "--compose-output"
            || arg == "--rand-seed"
            || arg == "--compose-rand-seed"
        {
            index += 1;
        }
        index += 1;
    }
    if !found {
        return None;
    }
    Some((|| {
        if args.len() != 1 {
            return Err("agent usage options must be used alone".to_owned());
        }
        let arg = args[0].to_str().ok_or("invalid agent usage option")?;
        let output = if arg == "--agent-usage-list-skills" {
            SKILLS
                .iter()
                .map(|(name, _)| format!("{name}\n"))
                .collect::<String>()
                .into_bytes()
        } else if let Some(name) = arg.strip_prefix("--agent-usage-show-skill=") {
            if name == "ALL" {
                let mut out = b"AGENT-USAGE/1\n".to_vec();
                for (skill, body) in SKILLS {
                    record(&mut out, &format!("{skill}/SKILL.md"), body);
                }
                record(
                    &mut out,
                    "git-compose-commit/scripts/extract-agent-skills.py",
                    EXTRACTOR,
                );
                out.extend_from_slice(b"0 0\n");
                out
            } else {
                SKILLS
                    .iter()
                    .find(|(skill, _)| *skill == name)
                    .map(|(_, body)| body.as_bytes().to_vec())
                    .ok_or_else(|| format!("unknown skill '{name}'"))?
            }
        } else {
            return Err(format!("unknown agent usage option '{arg}'"));
        };
        io::stdout()
            .lock()
            .write_all(&output)
            .map_err(|error| format!("could not write standard output: {error}"))
    })())
}
