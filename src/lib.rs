use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::sync::atomic::{AtomicU64, Ordering};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

mod agent_usage;

const WRAP_WIDTH: usize = 72;
const MAX_CHART_SIZE: usize = 256;
const FORTUNES: &[&str] = &[
    "Works on my machine; the machine has since left.",
    "One more cache should fix the other caches.",
    "Nothing is permanent except the temporary workaround.",
    "Tests passed. The tests would like a word.",
    "The bug was hiding behind a very confident comment.",
    "Ship it after the tests finish arguing.",
];
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
static BUBBLE_RANDOM_COUNTER: AtomicU64 = AtomicU64::new(0);

const COMPOSE_VALUE_OPTIONS: &[&str] = &[
    "--compose-subject",
    "--compose-text",
    "--compose-detail",
    "--compose-line",
    "--compose-field",
    "--compose-note",
    "--compose-warning",
    "--compose-wrap-width",
    "--compose-charset",
    "--compose-sparkline",
    "--compose-progress",
    "--compose-checklist-item",
    "--compose-banner",
    "--compose-badges",
    "--compose-fortune",
    "--compose-stamp",
    "--compose-table-head",
    "--compose-table-row",
    "--compose-table-rows",
    "--compose-hbar-chart",
    "--compose-vbar-chart",
    "--compose-hflow-chart",
    "--compose-vflow-chart",
    "--compose-flow-chart",
    "--compose-flow-node",
    "--compose-flow-decision",
    "--compose-flow-terminal",
    "--compose-flow-edge",
    "--compose-bubble",
    "--compose-th",
    "--compose-tr",
    "--compose-list",
    "--compose-list-item",
    "--subject",
    "--text",
    "--detail",
    "--line",
    "--field",
    "--note",
    "--warning",
    "--wrap-width",
    "--charset",
    "--sparkline",
    "--progress",
    "--checklist-item",
    "--banner",
    "--badges",
    "--fortune",
    "--stamp",
    "--table-head",
    "--table-row",
    "--table-rows",
    "--hbar-chart",
    "--vbar-chart",
    "--hflow-chart",
    "--vflow-chart",
    "--flow-chart",
    "--flow-node",
    "--flow-decision",
    "--flow-terminal",
    "--flow-edge",
    "--bubble",
    "--th",
    "--tr",
    "--list",
    "--list-item",
];

struct BubbleRng {
    state: u64,
}

impl BubbleRng {
    fn new(seed: Option<u64>) -> Self {
        let seed = seed.unwrap_or_else(|| {
            let clock = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos() as u64)
                .unwrap_or(0);
            clock
                ^ BUBBLE_RANDOM_COUNTER
                    .fetch_add(1, Ordering::Relaxed)
                    .wrapping_mul(0x9e37_79b9_7f4a_7c15)
                ^ u64::from(std::process::id())
        });
        Self {
            state: if seed == 0 {
                0xa076_1d64_78bd_642f
            } else {
                seed
            },
        }
    }

    fn index(&mut self, upper: usize) -> usize {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state as usize) % upper
    }
}

#[derive(Debug)]
enum Section {
    Text(String),
    Detail(String),
    Line(String),
    Fields(Vec<(String, String)>),
    Note(String),
    Warning(String),
    Table {
        head: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    BarChart(BarChart),
    Sparkline(Sparkline),
    Progress(Progress),
    Banner(StyledText),
    Badges(StyledText),
    Stamp(StyledText),
    FlowChart(FlowChart),
    FlowDiagram(FlowDiagram),
    Bubble(Bubble),
    List(Vec<ListEntry>),
}

#[derive(Debug)]
struct BarChart {
    orientation: ChartOrientation,
    alignment: ChartAlignment,
    size: usize,
    entries: Vec<(String, u64)>,
}

#[derive(Debug)]
struct Sparkline {
    values: Vec<f64>,
    label: Option<String>,
    min: Option<f64>,
    max: Option<f64>,
    endpoints: bool,
    alignment: ChartAlignment,
}

#[derive(Debug)]
struct Progress {
    fraction: f64,
    width: usize,
    label: Option<String>,
    alignment: ChartAlignment,
}

#[derive(Debug)]
struct StyledText {
    text: String,
    style: String,
    label: Option<String>,
    width: Option<usize>,
    alignment: ChartAlignment,
}

#[derive(Debug)]
struct FlowChart {
    orientation: ChartOrientation,
    alignment: ChartAlignment,
    steps: Vec<String>,
}

#[derive(Clone, Debug)]
struct FlowDiagram {
    direction: String,
    alignment: ChartAlignment,
    node_sep: f64,
    rank_sep: f64,
    nodes: Vec<FlowNode>,
    edges: Vec<FlowEdge>,
    node_ids: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct FlowNode {
    id: String,
    label: String,
    shape: FlowNodeShape,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FlowEdge {
    from: String,
    to: String,
    label: Option<String>,
}

#[derive(Debug)]
struct Bubble {
    speaker: BubbleSpeaker,
    speaker_side: SpeakerSide,
    bubble_side: BubbleSide,
    style: BubbleStyle,
    alignment: ChartAlignment,
    phrase: String,
}

#[derive(Debug, Default)]
struct ComposeConfig {
    wrap_width: Option<usize>,
    rand_seed: Option<u64>,
    bubble_speaker: Option<BubbleSpeaker>,
    bubble_style: Option<BubbleStyle>,
    bubble_speaker_side: Option<SpeakerSide>,
    bubble_side: Option<BubbleSide>,
    bubble_float: Option<ChartAlignment>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BubbleSpeaker {
    Cat,
    Dog,
    StickFigure,
    Cow,
    Robot,
    Owl,
    Dragon,
    Penguin,
    Duck,
    Ghost,
}

fn bubble_speaker(value: &str) -> Option<BubbleSpeaker> {
    Some(match value {
        "cat" => BubbleSpeaker::Cat,
        "dog" => BubbleSpeaker::Dog,
        "stick-figure" => BubbleSpeaker::StickFigure,
        "cow" => BubbleSpeaker::Cow,
        "robot" => BubbleSpeaker::Robot,
        "owl" => BubbleSpeaker::Owl,
        "dragon" => BubbleSpeaker::Dragon,
        "penguin" => BubbleSpeaker::Penguin,
        "duck" | "rubber-duck" => BubbleSpeaker::Duck,
        "ghost" => BubbleSpeaker::Ghost,
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BubbleStyle {
    Speech,
    Thought,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpeakerSide {
    Below,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BubbleSide {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChartOrientation {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FlowNodeShape {
    Box,
    Decision,
    Terminal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChartAlignment {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Charset {
    Ascii,
    Unicode,
}

struct Palette {
    horizontal: &'static str,
    vertical: &'static str,
    top: (&'static str, &'static str, &'static str),
    middle: (&'static str, &'static str, &'static str),
    bottom: (&'static str, &'static str, &'static str),
    bar: &'static str,
    arrow_right: &'static str,
    arrow_down: &'static str,
}

impl Charset {
    fn palette(self) -> Palette {
        match self {
            Self::Ascii => Palette {
                horizontal: "-",
                vertical: "|",
                top: ("+", "+", "+"),
                middle: ("+", "+", "+"),
                bottom: ("+", "+", "+"),
                bar: "#",
                arrow_right: " --> ",
                arrow_down: "v",
            },
            Self::Unicode => Palette {
                horizontal: "─",
                vertical: "│",
                top: ("┌", "┬", "┐"),
                middle: ("├", "┼", "┤"),
                bottom: ("└", "┴", "┘"),
                bar: "█",
                arrow_right: " → ",
                arrow_down: "↓",
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ListKind {
    Numbered,
    Bullets,
    Check(bool),
}

#[derive(Debug)]
struct ListEntry {
    kind: ListKind,
    depth: usize,
    text: String,
}

fn synopsis() -> &'static str {
    include_str!("help/short.txt").trim_end()
}

fn usage() -> &'static str {
    include_str!("help/long.txt").trim_end()
}

const HELP_HEADINGS: &[&str] = &[
    "HOW IT WORKS",
    "MESSAGE OPTIONS",
    "OUTPUT AND CONTROL",
    "TABLES",
    "LISTS",
    "FOOTERS",
    "BAR CHARTS",
    "LINEAR FLOW CHARTS",
    "BRANCHING FLOWCHARTS",
    "VISUAL DECORATIONS",
    "SPEECH AND THOUGHT BUBBLES",
    "ALIASES, DEFAULTS, AND HELP",
    "EXAMPLES",
];

fn help_section(guide: &str, heading: &str) -> Option<String> {
    let marker = format!("\n{heading}\n\n");
    let start = guide.find(&marker)? + 1;
    let end = HELP_HEADINGS
        .iter()
        .filter(|other| **other != heading)
        .filter_map(|other| {
            let marker = format!("\n{other}\n\n");
            guide[start + 1..]
                .find(&marker)
                .map(|offset| start + 1 + offset)
        })
        .min()
        .unwrap_or(guide.len());
    Some(guide[start..end].trim_end().to_owned())
}

fn help_example(guide: &str, number: usize) -> Option<String> {
    let marker = format!("\n  {number}. ");
    let start = guide.find(&marker)? + 1;
    let next = format!("\n  {}. ", number + 1);
    let end = guide[start..]
        .find(&next)
        .map(|offset| start + offset)
        .unwrap_or(guide.len());
    Some(guide[start..end].trim_end().to_owned())
}

fn topic_help(topic: &str) -> Result<String, String> {
    let (sections, examples): (&[&str], &[usize]) = match topic {
        "message" => (&["HOW IT WORKS", "MESSAGE OPTIONS"], &[1, 3, 14]),
        "output" => (&["OUTPUT AND CONTROL"], &[2]),
        "tables" => (&["TABLES"], &[4]),
        "lists" => (&["LISTS"], &[5]),
        "footers" => (&["FOOTERS"], &[6]),
        "charts" => (&["BAR CHARTS"], &[7, 8]),
        "flowcharts" => (
            &["LINEAR FLOW CHARTS", "BRANCHING FLOWCHARTS"],
            &[9, 10, 11],
        ),
        "visuals" => (&["VISUAL DECORATIONS"], &[15]),
        "bubbles" => (&["SPEECH AND THOUGHT BUBBLES"], &[12, 13]),
        "examples" => (&[], &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
        _ => {
            return Err(format!(
                "unknown help topic '{topic}'; choose message, output, tables, lists, footers, charts, flowcharts, visuals, bubbles, or examples"
            ));
        }
    };
    let guide = usage();
    let mut blocks = vec![format!("git-compose-commit - {topic}")];
    for heading in sections {
        blocks.push(
            help_section(guide, heading)
                .ok_or_else(|| format!("internal help error: section '{heading}' is missing"))?,
        );
    }
    blocks.push("EXAMPLES".to_owned());
    for number in examples {
        blocks.push(
            help_example(guide, *number)
                .ok_or_else(|| format!("internal help error: example {number} is missing"))?,
        );
    }
    blocks.push("Use --help=long for the complete guide.".to_owned());
    Ok(blocks.join("\n\n"))
}

fn as_text(value: &OsStr, option: &str) -> Result<String, String> {
    value
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("{option} value must be valid UTF-8"))
}

fn split_equals(arg: &OsStr, name: &str) -> Option<Option<String>> {
    let arg = arg.to_str()?;
    if arg == name {
        Some(None)
    } else {
        arg.strip_prefix(name)
            .and_then(|tail| tail.strip_prefix('='))
            .map(|value| Some(value.to_owned()))
    }
}

fn config_path() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(|home| PathBuf::from(home).join(".compose-commit.toml"))
}

fn parse_config_string(value: &str, key: &str, path: &std::path::Path) -> Result<String, String> {
    let value = value.trim();
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return Err(format!(
            "{}: {key} must be a quoted TOML string",
            path.display()
        ));
    }
    let inner = &value[1..value.len() - 1];
    let mut output = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let Some(escaped) = chars.next() else {
                return Err(format!("{}: invalid escape in {key}", path.display()));
            };
            match escaped {
                '"' => output.push('"'),
                '\\' => output.push('\\'),
                'n' => output.push('\n'),
                't' => output.push('\t'),
                _ => return Err(format!("{}: invalid escape in {key}", path.display())),
            }
        } else {
            output.push(ch);
        }
    }
    Ok(output)
}

fn load_config_from(path: &std::path::Path) -> Result<ComposeConfig, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(ComposeConfig::default())
        }
        Err(error) => return Err(format!("could not read {}: {error}", path.display())),
    };
    let mut config = ComposeConfig::default();
    let mut seen = BTreeSet::new();
    for (line_number, line) in contents.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!(
                "{}:{}: expected key = value",
                path.display(),
                line_number + 1
            ));
        };
        let key = key.trim();
        if !seen.insert(key.to_owned()) {
            return Err(format!(
                "{}:{}: duplicate setting {key}",
                path.display(),
                line_number + 1
            ));
        }
        let value = value
            .split_once('#')
            .map_or(value, |(before, _)| before)
            .trim();
        let invalid =
            |reason: &str| format!("{}:{}: {key} {reason}", path.display(), line_number + 1);
        match key {
            "wrap_width" => {
                let width = value
                    .replace('_', "")
                    .parse::<usize>()
                    .map_err(|_| invalid("must be an integer from 20 to 256"))?;
                if !(20..=256).contains(&width) {
                    return Err(invalid("must be an integer from 20 to 256"));
                }
                config.wrap_width = Some(width);
            }
            "rand_seed" => {
                config.rand_seed = Some(
                    value
                        .replace('_', "")
                        .parse::<u64>()
                        .map_err(|_| invalid("must be an unsigned integer"))?,
                );
            }
            "bubble_speaker" => {
                config.bubble_speaker = Some(
                    bubble_speaker(&parse_config_string(value, key, path)?)
                        .ok_or_else(|| invalid("must name a supported bubble speaker"))?,
                );
            }
            "bubble_style" => {
                config.bubble_style = Some(match parse_config_string(value, key, path)?.as_str() {
                    "speech" | "voice" => BubbleStyle::Speech,
                    "thought" => BubbleStyle::Thought,
                    _ => return Err(invalid("must be speech or thought")),
                });
            }
            "bubble_speaker_side" => {
                config.bubble_speaker_side =
                    Some(match parse_config_string(value, key, path)?.as_str() {
                        "below" => SpeakerSide::Below,
                        "left" => SpeakerSide::Left,
                        "right" => SpeakerSide::Right,
                        _ => return Err(invalid("must be below, left, or right")),
                    });
            }
            "bubble_side" => {
                config.bubble_side = Some(match parse_config_string(value, key, path)?.as_str() {
                    "left" => BubbleSide::Left,
                    "center" => BubbleSide::Center,
                    "right" => BubbleSide::Right,
                    _ => return Err(invalid("must be left, center, or right")),
                });
            }
            "bubble_float" => {
                config.bubble_float = Some(match parse_config_string(value, key, path)?.as_str() {
                    "left" => ChartAlignment::Left,
                    "center" => ChartAlignment::Center,
                    "right" => ChartAlignment::Right,
                    _ => return Err(invalid("must be left, center, or right")),
                });
            }
            _ => return Err(invalid("is not a recognized setting")),
        }
    }
    Ok(config)
}

fn load_user_config() -> Result<ComposeConfig, String> {
    match config_path() {
        Some(path) => load_config_from(&path),
        None => Ok(ComposeConfig::default()),
    }
}

fn git_option_takes_value(arg: &OsStr) -> bool {
    matches!(
        arg.to_str(),
        Some(
            "-m" | "--message"
                | "-F"
                | "--file"
                | "-C"
                | "--reuse-message"
                | "-c"
                | "--reedit-message"
                | "--author"
                | "--date"
                | "--cleanup"
                | "--template"
                | "--fixup"
                | "--squash"
                | "--trailer"
        )
    )
}

fn rand_seed_option(arg: &OsStr) -> Option<Option<String>> {
    split_equals(arg, "--rand-seed").or_else(|| split_equals(arg, "--compose-rand-seed"))
}

enum OutputTarget {
    File(OsString),
    Stdout,
}

fn extract_output_option(
    args: Vec<OsString>,
) -> Result<(Option<OutputTarget>, Vec<OsString>), String> {
    let mut output = None;
    let mut remaining = Vec::new();
    let mut i = 0;
    let mut after_separator = false;
    while i < args.len() {
        let arg = &args[i];
        if after_separator || arg == OsStr::new("--") {
            after_separator = true;
            remaining.push(arg.clone());
            i += 1;
            continue;
        }
        // An option-looking value belongs to its preceding option. In
        // particular, a subject or paragraph may literally be "--stdout".
        if git_option_takes_value(arg)
            || COMPOSE_VALUE_OPTIONS
                .iter()
                .any(|name| arg == OsStr::new(name))
            || is_dynamic_footer_option(arg)
            || arg == OsStr::new("--rand-seed")
            || arg == OsStr::new("--compose-rand-seed")
        {
            remaining.push(arg.clone());
            i += 1;
            if let Some(value) = args.get(i) {
                remaining.push(value.clone());
                i += 1;
            }
            continue;
        }
        if let Some(inline) =
            split_equals(arg, "--stdout").or_else(|| split_equals(arg, "--compose-stdout"))
        {
            if inline.is_some() {
                return Err("--stdout does not take a value".into());
            }
            if output.is_some() {
                return Err("--output and --stdout may only be specified once".into());
            }
            output = Some(OutputTarget::Stdout);
            i += 1;
            continue;
        }
        let parsed =
            split_equals(arg, "--output").or_else(|| split_equals(arg, "--compose-output"));
        if let Some(inline) = parsed {
            if output.is_some() {
                return Err("--output and --stdout may only be specified once".into());
            }
            let path = match inline {
                Some(path) => OsString::from(path),
                None => {
                    i += 1;
                    args.get(i)
                        .cloned()
                        .ok_or_else(|| "--output requires a file path".to_owned())?
                }
            };
            if path.is_empty() {
                return Err("--output requires a file path".into());
            }
            output = Some(OutputTarget::File(path));
        } else {
            remaining.push(arg.clone());
        }
        i += 1;
    }
    Ok((output, remaining))
}

fn is_dynamic_footer_option(arg: &OsStr) -> bool {
    let Some(value) = arg.to_str() else {
        return false;
    };
    let Some(suffix) = value
        .strip_prefix("--compose-footer-")
        .or_else(|| value.strip_prefix("--footer-"))
    else {
        return false;
    };
    !suffix.contains('=') && !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit())
}

fn extract_rand_seed(args: &[OsString]) -> Result<Option<u64>, String> {
    let mut seed = None;
    let mut saw_seed = false;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == OsStr::new("--") {
            break;
        }
        if let Some(inline) = rand_seed_option(arg) {
            if saw_seed {
                return Err("--rand-seed may only be specified once".into());
            }
            saw_seed = true;
            let value = match inline {
                Some(value) => value,
                None => {
                    i += 1;
                    let Some(value) = args.get(i) else {
                        return Err("--rand-seed requires a value".into());
                    };
                    as_text(value, "--rand-seed")?
                }
            };
            seed = Some(
                value
                    .parse::<u64>()
                    .map_err(|_| "--rand-seed must be an unsigned integer".to_owned())?,
            );
        } else if git_option_takes_value(arg)
            || COMPOSE_VALUE_OPTIONS
                .iter()
                .any(|name| arg == OsStr::new(name))
            || is_dynamic_footer_option(arg)
        {
            i += 1;
        }
        i += 1;
    }
    Ok(seed)
}

/// Everything [`parse_args`] extracts from the command line.
struct ParsedArgs {
    subject: Option<String>,
    sections: Vec<Section>,
    footers: BTreeMap<usize, String>,
    wrap_width: usize,
    charset: Charset,
    git_args: Vec<OsString>,
}

fn parse_args(args: Vec<OsString>, config: &ComposeConfig) -> Result<ParsedArgs, String> {
    let mut bubble_rng = BubbleRng::new(extract_rand_seed(&args)?.or(config.rand_seed));
    let mut subject = None;
    let mut sections = Vec::new();
    let mut footers = BTreeMap::new();
    let mut wrap_width = config.wrap_width.unwrap_or(WRAP_WIDTH);
    let mut saw_wrap_width = false;
    let mut charset = Charset::Ascii;
    let mut saw_charset = false;
    let mut git_args = Vec::new();
    let mut active_table = None;
    let mut active_flow_diagram = None;
    let mut list_stack: Vec<ListKind> = Vec::new();
    let mut active_list_section = None;
    let mut i = 0;
    let mut passthrough = false;

    while i < args.len() {
        let arg = &args[i];
        if passthrough {
            git_args.push(arg.clone());
            i += 1;
            continue;
        }
        if arg == OsStr::new("--") {
            active_flow_diagram = None;
            git_args.push(arg.clone());
            passthrough = true;
            i += 1;
            continue;
        }
        if arg == OsStr::new("-h") || arg == OsStr::new("--help=short") {
            println!("{}", synopsis());
            std::process::exit(0);
        }
        if arg == OsStr::new("--help")
            || arg == OsStr::new("--help=long")
            || arg == OsStr::new("--man")
        {
            println!("{}", usage());
            std::process::exit(0);
        }
        if let Some(topic) = arg.to_str().and_then(|value| value.strip_prefix("--help=")) {
            println!("{}", topic_help(topic)?);
            std::process::exit(0);
        }
        if arg == OsStr::new("--version") {
            println!("git-compose-commit {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }

        if let Some(inline) = rand_seed_option(arg) {
            active_flow_diagram = None;
            if inline.is_none() {
                i += 1;
            }
            i += 1;
            continue;
        }

        let recognized = COMPOSE_VALUE_OPTIONS
            .iter()
            .find_map(|name| split_equals(arg, name).map(|inline| (*name, inline)));
        let dynamic_footer = arg.to_str().and_then(|value| {
            let (suffix, option_prefix) = value
                .strip_prefix("--compose-footer-")
                .map(|suffix| (suffix, "--compose-footer"))
                .or_else(|| {
                    value
                        .strip_prefix("--footer-")
                        .map(|suffix| (suffix, "--footer"))
                })?;
            {
                let (number, inline) = match suffix.split_once('=') {
                    Some((number, value)) => (number, Some(value.to_owned())),
                    None => (suffix, None),
                };
                if number.is_empty() || !number.chars().all(|ch| ch.is_ascii_digit()) {
                    None
                } else {
                    number
                        .parse::<usize>()
                        .ok()
                        .map(|number| (number, inline, option_prefix))
                }
            }
        });
        if let Some((number, inline, option_prefix)) = dynamic_footer {
            active_flow_diagram = None;
            let value = match inline {
                Some(value) => value,
                None => {
                    i += 1;
                    let Some(value) = args.get(i) else {
                        return Err(format!("{option_prefix}-{number} requires a value"));
                    };
                    as_text(value, option_prefix)?
                }
            };
            if value.is_empty() {
                return Err(format!("{option_prefix}-{number} value cannot be empty"));
            }
            if footers.insert(number, value).is_some() {
                return Err(format!(
                    "{option_prefix}-{number} may only be specified once"
                ));
            }
            i += 1;
            continue;
        }
        if arg == OsStr::new("--compose-list-end") || arg == OsStr::new("--list-end") {
            active_flow_diagram = None;
            if list_stack.pop().is_none() {
                return Err("--list-end has no open list".into());
            }
            active_table = None;
            i += 1;
            continue;
        }
        let list_shorthand = match arg.to_str() {
            Some("--numlist" | "--compose-numlist") => Some(ListKind::Numbered),
            Some("--bullets" | "--compose-bullets") => Some(ListKind::Bullets),
            _ => None,
        };
        if let Some(kind) = list_shorthand {
            active_flow_diagram = None;
            list_stack.push(kind);
            if active_list_section.is_none() {
                sections.push(Section::List(Vec::new()));
                active_list_section = Some(sections.len() - 1);
            }
            active_table = None;
            i += 1;
            continue;
        }
        if let Some((name, inline)) = recognized {
            let value = match inline {
                Some(value) => value,
                None => {
                    i += 1;
                    let Some(value) = args.get(i) else {
                        return Err(format!("{name} requires a value"));
                    };
                    as_text(value, name)?
                }
            };
            if value.is_empty() {
                return Err(format!("{name} value cannot be empty"));
            }
            if matches!(name, "--wrap-width" | "--compose-wrap-width") {
                active_flow_diagram = None;
                if saw_wrap_width {
                    return Err("--wrap-width may only be specified once".into());
                }
                saw_wrap_width = true;
                wrap_width = value
                    .parse::<usize>()
                    .map_err(|_| "--wrap-width must be an integer from 20 to 256".to_owned())?;
                if !(20..=256).contains(&wrap_width) {
                    return Err("--wrap-width must be an integer from 20 to 256".into());
                }
                i += 1;
                continue;
            }
            if matches!(name, "--charset" | "--compose-charset") {
                active_flow_diagram = None;
                if saw_charset {
                    return Err("--charset may only be specified once".into());
                }
                saw_charset = true;
                charset = match value.as_str() {
                    "ascii" => Charset::Ascii,
                    "unicode" => Charset::Unicode,
                    _ => return Err("--charset must be ascii or unicode".into()),
                };
                i += 1;
                continue;
            }
            let canonical = match name {
                "--subject" => "--compose-subject",
                "--text" => "--compose-text",
                "--detail" => "--compose-detail",
                "--line" => "--compose-line",
                "--field" => "--compose-field",
                "--note" => "--compose-note",
                "--warning" => "--compose-warning",
                "--table-head" | "--th" | "--compose-th" => "--compose-table-head",
                "--table-row" | "--tr" | "--compose-tr" => "--compose-table-row",
                "--table-rows" => "--compose-table-rows",
                "--hbar-chart" => "--compose-hbar-chart",
                "--sparkline" => "--compose-sparkline",
                "--progress" => "--compose-progress",
                "--checklist-item" => "--compose-checklist-item",
                "--banner" => "--compose-banner",
                "--badges" => "--compose-badges",
                "--fortune" => "--compose-fortune",
                "--stamp" => "--compose-stamp",
                "--vbar-chart" => "--compose-vbar-chart",
                "--hflow-chart" => "--compose-hflow-chart",
                "--vflow-chart" => "--compose-vflow-chart",
                "--flow-chart" => "--compose-flow-chart",
                "--flow-node" => "--compose-flow-node",
                "--flow-decision" => "--compose-flow-decision",
                "--flow-terminal" => "--compose-flow-terminal",
                "--flow-edge" => "--compose-flow-edge",
                "--bubble" => "--compose-bubble",
                "--list" => "--compose-list",
                "--list-item" => "--compose-list-item",
                _ => name,
            };
            if !matches!(
                canonical,
                "--compose-flow-chart"
                    | "--compose-flow-node"
                    | "--compose-flow-decision"
                    | "--compose-flow-terminal"
                    | "--compose-flow-edge"
            ) {
                active_flow_diagram = None;
            }
            match canonical {
                "--compose-subject" => {
                    if subject.replace(value).is_some() {
                        return Err("--compose-subject may only be specified once".into());
                    }
                }
                "--compose-text" => sections.push(Section::Text(value)),
                "--compose-detail" => sections.push(Section::Detail(value)),
                "--compose-line" => {
                    if value.contains('\n') || value.contains('\r') {
                        return Err("--compose-line value must be a single line".into());
                    }
                    sections.push(Section::Line(value));
                }
                "--compose-field" => {
                    let Some((key, value)) = value.split_once(':') else {
                        return Err("--field requires KEY:VALUE".into());
                    };
                    let key = key.trim();
                    let value = value.trim();
                    if key.is_empty() || value.is_empty() {
                        return Err("--field requires a non-empty KEY and VALUE".into());
                    }
                    match sections.last_mut() {
                        Some(Section::Fields(fields)) => {
                            fields.push((key.to_owned(), value.to_owned()));
                        }
                        _ => {
                            sections.push(Section::Fields(vec![(key.to_owned(), value.to_owned())]))
                        }
                    }
                }
                "--compose-note" => sections.push(Section::Note(value)),
                "--compose-warning" => sections.push(Section::Warning(value)),
                "--compose-sparkline" => {
                    sections.push(Section::Sparkline(parse_sparkline(&value)?))
                }
                "--compose-progress" => sections.push(Section::Progress(parse_progress(&value)?)),
                "--compose-banner" => sections.push(Section::Banner(parse_styled_text(
                    &value, "--banner", "text", "box",
                )?)),
                "--compose-badges" => sections.push(Section::Badges(parse_styled_text(
                    &value, "--badges", "labels", "plain",
                )?)),
                "--compose-stamp" => sections.push(Section::Stamp(parse_styled_text(
                    &value, "--stamp", "name", "plain",
                )?)),
                "--compose-checklist-item" => {
                    let (state, text) = value
                        .split_once(':')
                        .ok_or("--checklist-item requires done:TEXT or todo:TEXT")?;
                    let checked = match state {
                        "done" => true,
                        "todo" => false,
                        _ => return Err("--checklist-item requires done:TEXT or todo:TEXT".into()),
                    };
                    if text.trim().is_empty() {
                        return Err("--checklist-item text cannot be empty".into());
                    }
                    let entry = ListEntry {
                        kind: ListKind::Check(checked),
                        depth: 0,
                        text: text.trim().to_owned(),
                    };
                    match sections.last_mut() {
                        Some(Section::List(items))
                            if items
                                .last()
                                .is_some_and(|item| matches!(item.kind, ListKind::Check(_))) =>
                        {
                            items.push(entry)
                        }
                        _ => sections.push(Section::List(vec![entry])),
                    }
                }
                "--compose-table-head" | "--compose-th" => {
                    let head = parse_table_cells(&value);
                    if head.is_empty() {
                        return Err("--compose-table-head requires at least one column".into());
                    }
                    sections.push(Section::Table {
                        head,
                        rows: Vec::new(),
                    });
                    active_table = Some(sections.len() - 1);
                    active_list_section = None;
                    list_stack.clear();
                }
                "--compose-table-row" | "--compose-tr" => {
                    let row = parse_table_cells(&value);
                    let Some(index) = active_table else {
                        return Err(
                            "--compose-table-row requires a preceding --compose-table-head".into(),
                        );
                    };
                    let Section::Table { head, rows } = &mut sections[index] else {
                        unreachable!()
                    };
                    if row.len() != head.len() {
                        return Err(format!(
                            "table row has {} cells, but its header has {}",
                            row.len(),
                            head.len()
                        ));
                    }
                    rows.push(row);
                    active_list_section = None;
                    list_stack.clear();
                }
                "--compose-table-rows" => {
                    let Some(index) = active_table else {
                        return Err(
                            "--compose-table-rows requires a preceding --compose-table-head".into(),
                        );
                    };
                    let Section::Table { head, rows } = &mut sections[index] else {
                        unreachable!()
                    };
                    for cells in value.split(';') {
                        let row = parse_table_cells(cells);
                        if row.len() != head.len() {
                            return Err(format!(
                                "table row has {} cells, but its header has {}",
                                row.len(),
                                head.len()
                            ));
                        }
                        rows.push(row);
                    }
                    active_list_section = None;
                    list_stack.clear();
                }
                "--compose-hbar-chart" | "--compose-vbar-chart" => {
                    let orientation = if canonical == "--compose-hbar-chart" {
                        ChartOrientation::Horizontal
                    } else {
                        ChartOrientation::Vertical
                    };
                    let chart = parse_bar_chart(&value, orientation)?;
                    sections.push(Section::BarChart(chart));
                }
                "--compose-hflow-chart" | "--compose-vflow-chart" => {
                    let orientation = if canonical == "--compose-hflow-chart" {
                        ChartOrientation::Horizontal
                    } else {
                        ChartOrientation::Vertical
                    };
                    sections.push(Section::FlowChart(parse_flow_chart(&value, orientation)?));
                }
                "--compose-flow-chart" => {
                    let diagram = parse_flow_diagram_options(&value)?;
                    sections.push(Section::FlowDiagram(diagram));
                    active_flow_diagram = Some(sections.len() - 1);
                    active_table = None;
                    active_list_section = None;
                    list_stack.clear();
                }
                "--compose-flow-node" | "--compose-flow-decision" | "--compose-flow-terminal" => {
                    let Some(index) = active_flow_diagram else {
                        return Err(format!("{name} requires a preceding --flow-chart"));
                    };
                    let shape = match canonical {
                        "--compose-flow-node" => FlowNodeShape::Box,
                        "--compose-flow-decision" => FlowNodeShape::Decision,
                        _ => FlowNodeShape::Terminal,
                    };
                    let (id, label) = parse_flow_node(&value, shape)?;
                    let Section::FlowDiagram(diagram) = &mut sections[index] else {
                        unreachable!()
                    };
                    if !diagram.node_ids.insert(id.clone()) {
                        return Err(format!(
                            "--flow-chart node '{id}' is defined more than once"
                        ));
                    }
                    diagram.nodes.push(FlowNode { id, label, shape });
                }
                "--compose-flow-edge" => {
                    let Some(index) = active_flow_diagram else {
                        return Err("--flow-edge requires a preceding --flow-chart".into());
                    };
                    let edges = parse_flow_edges(&value)?;
                    let Section::FlowDiagram(diagram) = &mut sections[index] else {
                        unreachable!()
                    };
                    diagram.edges.extend(edges);
                }
                "--compose-bubble" => {
                    sections.push(Section::Bubble(parse_bubble(
                        &value,
                        &mut bubble_rng,
                        config,
                    )?));
                }
                "--compose-fortune" => {
                    let index = if value == "random" {
                        bubble_rng.index(FORTUNES.len())
                    } else {
                        value
                            .parse::<usize>()
                            .ok()
                            .and_then(|n| n.checked_sub(1))
                            .filter(|n| *n < FORTUNES.len())
                            .ok_or("--fortune must be random or a number from 1 to 6")?
                    };
                    sections.push(Section::Bubble(parse_bubble(
                        FORTUNES[index],
                        &mut bubble_rng,
                        config,
                    )?));
                }
                "--compose-list" => {
                    let kind = match value.as_str() {
                        "numbered" => ListKind::Numbered,
                        "bullets" => ListKind::Bullets,
                        _ => return Err("--compose-list must be 'numbered' or 'bullets'".into()),
                    };
                    list_stack.push(kind);
                    if active_list_section.is_none() {
                        sections.push(Section::List(Vec::new()));
                        active_list_section = Some(sections.len() - 1);
                    }
                    active_table = None;
                }
                "--compose-list-item" => {
                    if list_stack.is_empty() {
                        list_stack.push(ListKind::Numbered);
                    }
                    let kind = list_stack.last().copied().unwrap_or(ListKind::Numbered);
                    let depth = list_stack.len().saturating_sub(1);
                    let index = match active_list_section {
                        Some(index) => index,
                        None => {
                            sections.push(Section::List(Vec::new()));
                            let index = sections.len() - 1;
                            active_list_section = Some(index);
                            index
                        }
                    };
                    let Section::List(items) = &mut sections[index] else {
                        unreachable!()
                    };
                    items.push(ListEntry {
                        kind,
                        depth,
                        text: value,
                    });
                    active_table = None;
                }
                _ => unreachable!(),
            }
            if !matches!(canonical, "--compose-list" | "--compose-list-item")
                && !matches!(canonical, "--compose-table-row")
                && !matches!(canonical, "--compose-table-rows")
                && !matches!(canonical, "--compose-table-head")
            {
                active_list_section = None;
                active_table = None;
                list_stack.clear();
            }
            i += 1;
            continue;
        }

        if git_option_takes_value(arg) && i + 1 < args.len() {
            active_flow_diagram = None;
            git_args.push(arg.clone());
            git_args.push(args[i + 1].clone());
            i += 2;
        } else {
            active_flow_diagram = None;
            git_args.push(arg.clone());
            i += 1;
        }
    }
    Ok(ParsedArgs {
        subject,
        sections,
        footers,
        wrap_width,
        charset,
        git_args,
    })
}

#[cfg(test)]
fn wrap_paragraph(text: &str) -> String {
    wrap_paragraph_at(text, WRAP_WIDTH)
}

fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

fn pad_left_aligned(text: &str, width: usize) -> String {
    format!(
        "{text}{}",
        " ".repeat(width.saturating_sub(display_width(text)))
    )
}

fn pad_centered(text: &str, width: usize) -> String {
    let padding = width.saturating_sub(display_width(text));
    format!(
        "{}{text}{}",
        " ".repeat(padding / 2),
        " ".repeat(padding - padding / 2)
    )
}

fn wrap_paragraph_at(text: &str, width: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in words {
        let extra = display_width(word) + usize::from(!line.is_empty());
        if !line.is_empty() && display_width(&line) + extra > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines.join("\n")
}

fn parse_table_cells(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|cell| cell.trim().to_owned())
        .collect()
}

fn format_table(head: &[String], rows: &[Vec<String>], charset: Charset) -> String {
    let palette = charset.palette();
    let mut widths: Vec<usize> = head.iter().map(|cell| display_width(cell)).collect();
    for row in rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(display_width(cell));
        }
    }
    let border = |(left, join, right): (&str, &str, &str)| {
        format!(
            "{left}{}{right}",
            widths
                .iter()
                .map(|width| palette.horizontal.repeat(width + 2))
                .collect::<Vec<_>>()
                .join(join)
        )
    };
    let render_row = |row: &[String]| {
        format!(
            "{} {} {}",
            palette.vertical,
            row.iter()
                .zip(&widths)
                .map(|(cell, width)| pad_left_aligned(cell, *width))
                .collect::<Vec<_>>()
                .join(&format!(" {} ", palette.vertical)),
            palette.vertical
        )
    };
    let mut lines = vec![
        border(palette.top),
        render_row(head),
        border(palette.middle),
    ];
    lines.extend(rows.iter().map(|row| render_row(row)));
    lines.push(border(palette.bottom));
    lines.join("\n")
}

fn parse_bar_chart(value: &str, orientation: ChartOrientation) -> Result<BarChart, String> {
    let option_name = match orientation {
        ChartOrientation::Horizontal => "--hbar-chart",
        ChartOrientation::Vertical => "--vbar-chart",
    };
    let clauses = value.split(';').collect::<Vec<_>>();
    let has_settings = clauses.iter().any(|clause| clause.contains('='));
    let mut alignment = ChartAlignment::Center;
    let mut size = match orientation {
        ChartOrientation::Horizontal => 24,
        ChartOrientation::Vertical => 6,
    };
    let mut data = None;

    if has_settings {
        for clause in clauses {
            let Some((key, setting)) = clause.split_once('=') else {
                return Err(format!(
                    "{option_name} requires data= when chart settings are specified"
                ));
            };
            match key {
                "float" => {
                    alignment = match setting {
                        "left" => ChartAlignment::Left,
                        "center" => ChartAlignment::Center,
                        "right" => ChartAlignment::Right,
                        _ => {
                            return Err(format!(
                                "{option_name} float must be 'left', 'center', or 'right'"
                            ))
                        }
                    };
                }
                "height" if orientation == ChartOrientation::Vertical => {
                    size = parse_chart_size(setting, option_name, "height")?;
                }
                "width" if orientation == ChartOrientation::Horizontal => {
                    size = parse_chart_size(setting, option_name, "width")?;
                }
                "data" => data = Some(setting),
                "height" | "width" => {
                    let expected = match orientation {
                        ChartOrientation::Horizontal => "width",
                        ChartOrientation::Vertical => "height",
                    };
                    return Err(format!("{option_name} uses {expected}= for its bar scale"));
                }
                _ => return Err(format!("{option_name} has unknown setting '{key}'")),
            }
        }
        let Some(data) = data else {
            return Err(format!(
                "{option_name} requires data= when chart settings are specified"
            ));
        };
        parse_chart_data(data, option_name, alignment, orientation, size)
    } else {
        parse_chart_data(value, option_name, alignment, orientation, size)
    }
}

fn parse_visual_alignment(value: &str, option: &str) -> Result<ChartAlignment, String> {
    match value {
        "left" => Ok(ChartAlignment::Left),
        "center" => Ok(ChartAlignment::Center),
        "right" => Ok(ChartAlignment::Right),
        _ => Err(format!("{option} float must be left, center, or right")),
    }
}

fn parse_sparkline(value: &str) -> Result<Sparkline, String> {
    let mut chart = Sparkline {
        values: Vec::new(),
        label: None,
        min: None,
        max: None,
        endpoints: false,
        alignment: ChartAlignment::Center,
    };
    let mut data = None;
    let mut seen = BTreeSet::new();
    if !value.contains('=') {
        data = Some(value);
    } else {
        for clause in value.split(';') {
            let (key, setting) = clause
                .split_once('=')
                .ok_or("--sparkline settings require name=value")?;
            if !seen.insert(key) {
                return Err(format!("--sparkline duplicate setting '{key}'"));
            }
            let number = |text: &str| -> Result<f64, String> {
                let n = text.parse::<f64>().map_err(|_| {
                    "--sparkline values and limits must be finite numbers".to_owned()
                })?;
                if !n.is_finite() {
                    return Err("--sparkline values and limits must be finite numbers".into());
                }
                Ok(n)
            };
            match key {
                "data" => data = Some(setting),
                "label" if !setting.trim().is_empty() && !setting.contains(['\n', '\r']) => {
                    chart.label = Some(setting.trim().to_owned())
                }
                "min" => chart.min = Some(number(setting)?),
                "max" => chart.max = Some(number(setting)?),
                "endpoints" => {
                    chart.endpoints = match setting {
                        "true" => true,
                        "false" => false,
                        _ => return Err("--sparkline endpoints must be true or false".into()),
                    }
                }
                "float" => chart.alignment = parse_visual_alignment(setting, "--sparkline")?,
                _ => return Err(format!("--sparkline has invalid setting '{key}'")),
            }
        }
    }
    let data = data.ok_or("--sparkline requires data=...")?;
    for item in data.split(',') {
        let n = item
            .trim()
            .parse::<f64>()
            .map_err(|_| "--sparkline values must be finite numbers".to_owned())?;
        if !n.is_finite() {
            return Err("--sparkline values must be finite numbers".into());
        }
        chart.values.push(n);
    }
    if chart.values.is_empty() {
        return Err("--sparkline requires data".into());
    }
    let min = chart
        .min
        .unwrap_or_else(|| chart.values.iter().copied().fold(f64::INFINITY, f64::min));
    let max = chart.max.unwrap_or_else(|| {
        chart
            .values
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
    });
    if min > max || chart.values.iter().any(|n| *n < min || *n > max) {
        return Err("--sparkline data must lie within its min and max".into());
    }
    Ok(chart)
}

fn format_sparkline(
    chart: &Sparkline,
    wrap_width: usize,
    charset: Charset,
) -> Result<String, String> {
    let min = chart
        .min
        .unwrap_or_else(|| chart.values.iter().copied().fold(f64::INFINITY, f64::min));
    let max = chart.max.unwrap_or_else(|| {
        chart
            .values
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
    });
    let levels: Vec<char> = match charset {
        Charset::Ascii => ".:-=+*#@",
        Charset::Unicode => "▁▂▃▄▅▆▇█",
    }
    .chars()
    .collect();
    let rendered: String = chart
        .values
        .iter()
        .map(|value| {
            let fraction = if min == max {
                0.5
            } else if (max - min).is_finite() {
                (value - min) / (max - min)
            } else {
                (value / 2.0 - min / 2.0) / (max / 2.0 - min / 2.0)
            };
            levels[((fraction.clamp(0.0, 1.0) * 7.0).round() as usize).min(7)]
        })
        .collect();
    let prefix = chart
        .label
        .as_ref()
        .map(|label| format!("{label}: "))
        .unwrap_or_default();
    let suffix = if chart.endpoints {
        format!(
            " ({} -> {})",
            chart.values[0],
            chart.values[chart.values.len() - 1]
        )
    } else {
        String::new()
    };
    let line = format!("{prefix}{rendered}{suffix}");
    if display_width(&line) > wrap_width {
        return Err(
            "--sparkline exceeds the message margin; use fewer values or a wider --wrap-width"
                .into(),
        );
    }
    Ok(align_block(&line, chart.alignment, wrap_width))
}

fn parse_progress(value: &str) -> Result<Progress, String> {
    let mut progress = Progress {
        fraction: 0.0,
        width: 20,
        label: None,
        alignment: ChartAlignment::Center,
    };
    let mut selected = None;
    let mut seen = BTreeSet::new();
    if !value.contains('=') {
        selected = Some(value);
    } else {
        for clause in value.split(';') {
            let (key, setting) = clause
                .split_once('=')
                .ok_or("--progress settings require name=value")?;
            if !seen.insert(key) {
                return Err(format!("--progress duplicate setting '{key}'"));
            }
            match key {
                "value" => selected = Some(setting),
                "width" => progress.width = parse_chart_size(setting, "--progress", "width")?,
                "label" if !setting.trim().is_empty() && !setting.contains(['\n', '\r']) => {
                    progress.label = Some(setting.trim().to_owned())
                }
                "float" => progress.alignment = parse_visual_alignment(setting, "--progress")?,
                _ => return Err(format!("--progress has invalid setting '{key}'")),
            }
        }
    }
    let selected = selected.ok_or("--progress requires value=...")?;
    let number = |text: &str| -> Result<f64, String> {
        let number = text
            .trim()
            .parse::<f64>()
            .map_err(|_| "--progress needs a percentage or fraction".to_owned())?;
        if !number.is_finite() {
            return Err("--progress needs finite numbers".into());
        }
        Ok(number)
    };
    progress.fraction = if let Some(percent) = selected.strip_suffix('%') {
        number(percent)? / 100.0
    } else if let Some((numerator, denominator)) = selected.split_once('/') {
        let denominator = number(denominator)?;
        if denominator <= 0.0 {
            return Err("--progress denominator must be positive".into());
        }
        number(numerator)? / denominator
    } else {
        return Err("--progress needs a percentage or fraction".into());
    };
    if !progress.fraction.is_finite() || !(0.0..=1.0).contains(&progress.fraction) {
        return Err("--progress must be between zero and 100%".into());
    }
    Ok(progress)
}

fn parse_styled_text(
    value: &str,
    option: &str,
    text_key: &str,
    default_style: &str,
) -> Result<StyledText, String> {
    let mut result = StyledText {
        text: String::new(),
        style: default_style.to_owned(),
        label: None,
        width: None,
        alignment: ChartAlignment::Center,
    };
    let mut seen = BTreeSet::new();
    if !value.contains('=') {
        result.text = value.trim().to_owned();
    } else {
        for clause in value.split(';') {
            let (key, setting) = clause
                .split_once('=')
                .ok_or_else(|| format!("{option} settings require name=value"))?;
            if !seen.insert(key) {
                return Err(format!("{option} duplicate setting '{key}'"));
            }
            match key {
                key if key == text_key => result.text = setting.trim().to_owned(),
                "style" => result.style = setting.to_owned(),
                "label"
                    if option == "--stamp"
                        && !setting.trim().is_empty()
                        && !setting.contains(['\n', '\r']) =>
                {
                    result.label = Some(setting.trim().to_owned())
                }
                "width" => result.width = Some(parse_chart_size(setting, option, "width")?),
                "float" => result.alignment = parse_visual_alignment(setting, option)?,
                _ => return Err(format!("{option} has unknown setting '{key}'")),
            }
        }
    }
    if result.text.is_empty() || result.text.contains(['\n', '\r']) {
        return Err(format!("{option} needs non-empty single-line {text_key}"));
    }
    Ok(result)
}

fn format_banner(
    banner: &StyledText,
    wrap_width: usize,
    charset: Charset,
) -> Result<String, String> {
    let text_width = display_width(&banner.text);
    let width = banner
        .width
        .unwrap_or((text_width + 4).max(20).min(wrap_width));
    if width > wrap_width || width < text_width + if banner.style == "box" { 4 } else { 0 } {
        return Err("--banner width must fit the text and message margin".into());
    }
    let palette = charset.palette();
    let centered = pad_centered(
        &banner.text,
        if banner.style == "box" {
            width - 2
        } else {
            width
        },
    );
    let rendered = match banner.style.as_str() {
        "plain" => centered,
        "box" => format!(
            "{}{}{}\n{}{}{}\n{}{}{}",
            palette.top.0,
            palette.horizontal.repeat(width - 2),
            palette.top.2,
            palette.vertical,
            centered,
            palette.vertical,
            palette.bottom.0,
            palette.horizontal.repeat(width - 2),
            palette.bottom.2
        ),
        "double" | "heavy" | "retro" => {
            let mark = match (banner.style.as_str(), charset) {
                ("double", Charset::Unicode) => "═",
                ("heavy", Charset::Unicode) => "━",
                ("double", _) => "=",
                ("heavy", _) => "#",
                _ => "*",
            };
            format!(
                "{}\n{}\n{}",
                mark.repeat(width),
                centered,
                mark.repeat(width)
            )
        }
        _ => return Err("--banner style must be plain, box, double, heavy, or retro".into()),
    };
    Ok(align_block(&rendered, banner.alignment, wrap_width))
}

fn format_badges(
    badges: &StyledText,
    wrap_width: usize,
    charset: Charset,
) -> Result<String, String> {
    if badges.width.is_some() {
        return Err("--badges does not accept width".into());
    }
    let labels = badges.text.split(',').map(str::trim).collect::<Vec<_>>();
    if labels.iter().any(|label| label.is_empty()) {
        return Err("--badges labels cannot be empty".into());
    }
    let palette = charset.palette();
    let rendered = match badges.style.as_str() {
        "plain" => labels
            .iter()
            .map(|label| format!("[ {label} ]"))
            .collect::<Vec<_>>()
            .join(" "),
        "boxed" => {
            let top = labels
                .iter()
                .map(|label| {
                    format!(
                        "{}{}{}",
                        palette.top.0,
                        palette.horizontal.repeat(display_width(label) + 2),
                        palette.top.2
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            let middle = labels
                .iter()
                .map(|label| format!("{} {label} {}", palette.vertical, palette.vertical))
                .collect::<Vec<_>>()
                .join(" ");
            let bottom = labels
                .iter()
                .map(|label| {
                    format!(
                        "{}{}{}",
                        palette.bottom.0,
                        palette.horizontal.repeat(display_width(label) + 2),
                        palette.bottom.2
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!("{top}\n{middle}\n{bottom}")
        }
        _ => return Err("--badges style must be plain or boxed".into()),
    };
    if rendered
        .lines()
        .any(|line| display_width(line) > wrap_width)
    {
        return Err("--badges exceeds the message margin".into());
    }
    Ok(align_block(&rendered, badges.alignment, wrap_width))
}

fn format_stamp(stamp: &StyledText, wrap_width: usize) -> Result<String, String> {
    if stamp.width.is_some() || stamp.style != "plain" {
        return Err("--stamp accepts name, label, and float settings".into());
    }
    let (art, default_label) = match stamp.text.as_str() {
        "approved" | "ok" => ("( OK )", "APPROVED"),
        "warning" | "danger" => ("/ ! \\", "WARNING"),
        "tests" | "tests-passed" => ("[x x]", "TESTS PASSED"),
        "achievement" => ("< * >", "ACHIEVEMENT UNLOCKED"),
        "bug" => ("x(#)x", "BUG ELIMINATED"),
        "rip" => ("  _|_\n |RIP|\n |___|", "RIP"),
        "deprecated" => ("[ - ]", "DEPRECATED"),
        "performance" => ("->>>", "PERFORMANCE"),
        _ => return Err("--stamp name must be approved, warning, tests, achievement, bug, rip, deprecated, or performance".into()),
    };
    let label = stamp.label.as_deref().unwrap_or(default_label);
    let width = art
        .lines()
        .map(display_width)
        .chain(std::iter::once(display_width(label)))
        .max()
        .unwrap_or(0);
    if width > wrap_width {
        return Err("--stamp exceeds the message margin".into());
    }
    let rendered = art
        .lines()
        .chain(std::iter::once(label))
        .map(|line| pad_centered(line, width))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(align_block(&rendered, stamp.alignment, wrap_width))
}

fn format_progress(
    progress: &Progress,
    wrap_width: usize,
    charset: Charset,
) -> Result<String, String> {
    let filled = (progress.fraction * progress.width as f64).round() as usize;
    let palette = charset.palette();
    let empty = if charset == Charset::Ascii {
        "."
    } else {
        "░"
    };
    let prefix = progress
        .label
        .as_ref()
        .map(|label| format!("{label}: "))
        .unwrap_or_default();
    let line = format!(
        "{prefix}[{}{}] {:.0}%",
        palette.bar.repeat(filled),
        empty.repeat(progress.width - filled),
        progress.fraction * 100.0
    );
    if display_width(&line) > wrap_width {
        return Err(
            "--progress exceeds the message margin; choose a smaller width or wider --wrap-width"
                .into(),
        );
    }
    Ok(align_block(&line, progress.alignment, wrap_width))
}

fn parse_chart_size(value: &str, option: &str, setting: &str) -> Result<usize, String> {
    let size = value
        .parse::<usize>()
        .map_err(|_| format!("{option} {setting} must be a positive whole number"))?;
    if size == 0 {
        return Err(format!(
            "{option} {setting} must be a positive whole number"
        ));
    }
    if size > MAX_CHART_SIZE {
        return Err(format!("{option} {setting} cannot exceed {MAX_CHART_SIZE}"));
    }
    Ok(size)
}

fn parse_chart_data(
    value: &str,
    option: &str,
    alignment: ChartAlignment,
    orientation: ChartOrientation,
    size: usize,
) -> Result<BarChart, String> {
    let mut entries = Vec::new();
    for item in value.split(',') {
        let Some((label, number)) = item.split_once(':') else {
            return Err(format!("{option} data items must use label:value"));
        };
        let label = label.trim();
        if label.is_empty() {
            return Err(format!("{option} data labels cannot be empty"));
        }
        let value = number
            .trim()
            .parse::<u64>()
            .map_err(|_| format!("{option} data values must be non-negative whole numbers"))?;
        entries.push((label.to_owned(), value));
    }
    if entries.len() < 2 {
        return Err(format!("{option} requires at least two data items"));
    }
    Ok(BarChart {
        orientation,
        alignment,
        size,
        entries,
    })
}

fn parse_flow_chart(value: &str, orientation: ChartOrientation) -> Result<FlowChart, String> {
    let option_name = match orientation {
        ChartOrientation::Horizontal => "--hflow-chart",
        ChartOrientation::Vertical => "--vflow-chart",
    };
    let clauses = value.split(';').collect::<Vec<_>>();
    let has_settings = clauses.iter().any(|clause| clause.contains('='));
    let mut alignment = ChartAlignment::Center;
    let data = if has_settings {
        let mut data = None;
        for clause in clauses {
            let Some((key, setting)) = clause.split_once('=') else {
                return Err(format!(
                    "{option_name} requires data= when chart settings are specified"
                ));
            };
            match key {
                "float" => {
                    alignment = match setting {
                        "left" => ChartAlignment::Left,
                        "center" => ChartAlignment::Center,
                        "right" => ChartAlignment::Right,
                        _ => {
                            return Err(format!(
                                "{option_name} float must be 'left', 'center', or 'right'"
                            ))
                        }
                    };
                }
                "data" => data = Some(setting),
                _ => return Err(format!("{option_name} has unknown setting '{key}'")),
            }
        }
        let Some(data) = data else {
            return Err(format!(
                "{option_name} requires data= when chart settings are specified"
            ));
        };
        data
    } else {
        value
    };
    let steps = data
        .split(',')
        .map(str::trim)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if steps.iter().any(String::is_empty) {
        return Err(format!("{option_name} box labels cannot be empty"));
    }
    if steps.len() < 2 {
        return Err(format!("{option_name} requires at least two boxes"));
    }
    Ok(FlowChart {
        orientation,
        alignment,
        steps,
    })
}

fn parse_flow_diagram_options(value: &str) -> Result<FlowDiagram, String> {
    let mut direction = "TD".to_owned();
    let mut alignment = ChartAlignment::Center;
    let mut node_sep = 0.8;
    let mut rank_sep = 1.0;
    let mut seen = BTreeSet::new();
    for clause in value.split(';') {
        let Some((key, setting)) = clause.split_once('=') else {
            return Err("--flow-chart options must use name=value".into());
        };
        if !seen.insert(key) {
            return Err(format!("--flow-chart {key} may only be specified once"));
        }
        match key {
            "direction" => {
                direction = match setting {
                    "vertical" | "TB" => "TB".to_owned(),
                    "horizontal" | "LR" => "LR".to_owned(),
                    _ => return Err("--flow-chart direction must be vertical or horizontal".into()),
                };
            }
            "float" => {
                alignment = match setting {
                    "left" => ChartAlignment::Left,
                    "center" => ChartAlignment::Center,
                    "right" => ChartAlignment::Right,
                    _ => {
                        return Err("--flow-chart float must be 'left', 'center', or 'right'".into())
                    }
                };
            }
            "nodesep" => node_sep = parse_graphviz_spacing("nodesep", setting)?,
            "ranksep" => rank_sep = parse_graphviz_spacing("ranksep", setting)?,
            _ => return Err(format!("--flow-chart has unknown option '{key}'")),
        }
    }
    Ok(FlowDiagram {
        direction,
        alignment,
        node_sep,
        rank_sep,
        nodes: Vec::new(),
        edges: Vec::new(),
        node_ids: BTreeSet::new(),
    })
}

fn parse_graphviz_spacing(key: &str, value: &str) -> Result<f64, String> {
    let spacing = value
        .parse::<f64>()
        .map_err(|_| format!("--flow-chart {key} must be a number greater than zero"))?;
    if !spacing.is_finite() || spacing <= 0.0 || spacing > 10.0 {
        return Err(format!(
            "--flow-chart {key} must be greater than zero and at most 10"
        ));
    }
    Ok(spacing)
}

fn parse_flow_node(value: &str, _shape: FlowNodeShape) -> Result<(String, String), String> {
    let Some((id, label)) = value.split_once(':') else {
        return Err("flow nodes must use ID:LABEL".into());
    };
    let id = id.trim();
    let label = label.trim();
    if !valid_flow_node_id(id) {
        return Err(format!(
            "flow node ID '{id}' must start with a letter or underscore and contain only letters, digits, and underscores"
        ));
    }
    if label.is_empty() || label.contains('\n') || label.contains('\r') {
        return Err("flow node labels must be non-empty and on one line".into());
    }
    Ok((id.to_owned(), label.to_owned()))
}

fn parse_flow_edges(value: &str) -> Result<Vec<FlowEdge>, String> {
    if value.contains('\n') || value.contains('\r') {
        return Err("--flow-edge must be a single line".into());
    }
    let (chain, label) = if let Some((chain, label_clause)) = value.split_once("[label=") {
        let label_clause = label_clause.trim();
        let Some(label) = label_clause
            .strip_suffix(']')
            .and_then(|label| label.trim().strip_prefix('"'))
            .and_then(|label| label.strip_suffix('"'))
        else {
            return Err("--flow-edge labels must use [label=\"TEXT\"]".into());
        };
        if chain.contains("->") {
            let segments = chain.split("->").collect::<Vec<_>>();
            if segments.len() != 2 {
                return Err("labeled --flow-edge must contain exactly one '->'".into());
            }
        }
        if label.is_empty() {
            return Err("--flow-edge labels must not be empty".into());
        }
        (chain.trim(), Some(label.trim().to_owned()))
    } else {
        (value.trim(), None)
    };
    let ids = chain.split("->").map(str::trim).collect::<Vec<_>>();
    if ids.len() < 2 || ids.iter().any(|id| !valid_flow_node_id(id)) {
        return Err("--flow-edge must use DOT syntax such as 'FROM -> TO'".into());
    }
    if label.is_some() && ids.len() != 2 {
        return Err("labeled --flow-edge must contain exactly one '->'".into());
    }
    Ok(ids
        .windows(2)
        .enumerate()
        .map(|(index, pair)| FlowEdge {
            from: pair[0].to_owned(),
            to: pair[1].to_owned(),
            label: (index == 0).then(|| label.clone()).flatten(),
        })
        .collect())
}

fn valid_flow_node_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn escape_dot_label(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn to_ascii_diagram(value: &str) -> String {
    let ascii = value
        .chars()
        .map(|ch| match ch {
            '┌' | '┐' | '└' | '┘' | '├' | '┤' | '┬' | '┴' | '┼' => '+',
            '─' => '-',
            '│' => '|',
            '▼' => 'v',
            '▲' => '^',
            '▶' => '>',
            '◀' => '<',
            other => other,
        })
        .collect::<String>();
    ascii
        .lines()
        .map(str::trim_end)
        .skip_while(|line| line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn style_flow_nodes(rendered: &str, nodes: &[FlowNode]) -> String {
    let mut rows = rendered
        .lines()
        .map(|line| line.chars().collect::<Vec<_>>())
        .collect::<Vec<_>>();

    for node in nodes.iter().filter(|node| node.shape != FlowNodeShape::Box) {
        let label = node.label.chars().collect::<Vec<_>>();
        let Some((row_index, left, right)) =
            rows.iter().enumerate().find_map(|(row_index, row)| {
                let label_start = row.windows(label.len()).position(|part| part == label)?;
                let left = row[..label_start].iter().rposition(|ch| *ch == '|')?;
                let right = row[label_start + label.len()..]
                    .iter()
                    .position(|ch| *ch == '|')?
                    + label_start
                    + label.len();
                if row_index == 0 || row_index + 1 >= rows.len() {
                    return None;
                }
                let top = &rows[row_index - 1];
                let bottom = &rows[row_index + 1];
                (top.get(left) == Some(&'+')
                    && top.get(right) == Some(&'+')
                    && bottom.get(left) == Some(&'+')
                    && bottom.get(right) == Some(&'+'))
                .then_some((row_index, left, right))
            })
        else {
            continue;
        };

        match node.shape {
            FlowNodeShape::Box => {}
            FlowNodeShape::Decision => {
                rows[row_index - 1][left] = '/';
                rows[row_index - 1][right] = '\\';
                rows[row_index][left] = '|';
                rows[row_index][right] = '|';
                rows[row_index + 1][left] = '\\';
                rows[row_index + 1][right] = '/';
            }
            FlowNodeShape::Terminal => {
                for row in [row_index - 1, row_index, row_index + 1] {
                    rows[row][left] = '(';
                    rows[row][right] = ')';
                }
            }
        }
    }
    rows.into_iter()
        .map(|row| row.into_iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Draw a left-to-right flowchart. Nodes come from [`place_flow_nodes`] so the
/// drawing does not depend on graphviz coordinates; graphs it cannot place
/// fall back to graphviz's scaled positions.
fn render_horizontal_layout(
    graph: &hascii::model::LayoutGraph,
    node_sep: f64,
    rank_sep: f64,
) -> String {
    let labels = graph
        .nodes
        .iter()
        .map(|node| node.label.clone())
        .collect::<Vec<_>>();
    let scale = hascii::scale::ScaleParams::compute(&graph.bb, &graph.nodes, &labels);
    let placement = place_flow_nodes(graph, true, node_sep, rank_sep);
    let (rects, mut grid, placed) = match placement {
        Some(placement) => {
            let mut grid = hascii::grid::CharGrid::new(placement.width, placement.height);
            draw_flow_boxes(&mut grid, &placement.rects, &graph.nodes);
            (placement.rects, grid, true)
        }
        None => {
            let rects = graph
                .nodes
                .iter()
                .zip(&labels)
                .map(|(node, label)| scale.node_rect(node, label))
                .collect::<Vec<_>>();
            let mut grid = hascii::grid::CharGrid::new(scale.grid_w, scale.grid_h);
            hascii::render_nodes::render_nodes(&mut grid, &graph.nodes, &labels, &scale, true);
            (rects, grid, false)
        }
    };

    let mut outgoing = vec![Vec::new(); graph.nodes.len()];
    let mut incoming = vec![Vec::new(); graph.nodes.len()];
    for (edge_index, edge) in graph.edges.iter().enumerate() {
        outgoing[edge.tail_idx].push(edge_index);
        incoming[edge.head_idx].push(edge_index);
    }
    let mut turn_columns = vec![None; graph.edges.len()];
    let mut shared_fanouts = vec![false; graph.nodes.len()];
    for edge_indices in outgoing.iter().filter(|edges| edges.len() > 1) {
        let first = &graph.edges[edge_indices[0]];
        let same_label = edge_indices
            .iter()
            .all(|&index| graph.edges[index].label == first.label);
        shared_fanouts[first.tail_idx] = same_label;
        let source_col = rects[first.tail_idx].right() + 1;
        let nearest_target_col = edge_indices
            .iter()
            .map(|&index| rects[graph.edges[index].head_idx].left.saturating_sub(1))
            .min_by_key(|&col| source_col.abs_diff(col))
            .unwrap_or(source_col);
        let shared_col =
            source_col.min(nearest_target_col) + source_col.abs_diff(nearest_target_col) / 2;
        for &index in edge_indices {
            turn_columns[index] = Some(shared_col);
        }
    }
    for edge_indices in incoming.iter().filter(|edges| edges.len() > 1) {
        let first = &graph.edges[edge_indices[0]];
        let target_col = rects[first.head_idx].left.saturating_sub(1);
        let nearest_source_col = edge_indices
            .iter()
            .map(|&index| rects[graph.edges[index].tail_idx].right() + 1)
            .min_by_key(|&col| target_col.abs_diff(col))
            .unwrap_or(target_col);
        let shared_col =
            nearest_source_col.min(target_col) + nearest_source_col.abs_diff(target_col) / 2;
        for &index in edge_indices {
            turn_columns[index].get_or_insert(shared_col);
        }
    }

    for (edge_index, edge) in graph.edges.iter().enumerate() {
        let source = &rects[edge.tail_idx];
        let target = &rects[edge.head_idx];
        let (start_col, end_col) = if source.center_col() <= target.center_col() {
            (source.right() + 1, target.left.saturating_sub(1))
        } else {
            (source.left.saturating_sub(1), target.right() + 1)
        };
        let separate_labeled_branch = outgoing[edge.tail_idx].len() > 1
            && !shared_fanouts[edge.tail_idx]
            && edge.label.is_some();
        let start_row = if separate_labeled_branch && target.center_row() < source.center_row() {
            source.center_row().saturating_sub(1)
        } else if separate_labeled_branch && target.center_row() > source.center_row() {
            source.center_row() + 1
        } else {
            source.center_row()
        };
        let start = (start_row, start_col);
        let end = (target.center_row(), end_col);
        let turn_col = turn_columns[edge_index]
            .unwrap_or_else(|| start_col.min(end_col) + start_col.abs_diff(end_col) / 2);
        let path = if start.0 == end.0 {
            vec![start, end]
        } else {
            vec![start, (start.0, turn_col), (end.0, turn_col), end]
        };
        draw_flow_edge_path(&mut grid, &path);
        let previous = path[path.len() - 2];
        let arrow = if end.1 > previous.1 {
            '>'
        } else if end.1 < previous.1 {
            '<'
        } else if end.0 > previous.0 {
            'v'
        } else {
            '^'
        };
        grid.set(end.0, end.1, hascii::grid::Cell::Arrow(arrow));

        if let Some(label) = &edge.label {
            if separate_labeled_branch && start.0 != end.0 {
                let label_start = start_col + 1;
                draw_edge_label(&mut grid, start.0, label_start, label.trim());
            } else if outgoing[edge.tail_idx].len() > 1 && start.0 != end.0 {
                let label_row = if end.0 > start.0 {
                    start.0 + 1
                } else {
                    start.0.saturating_sub(1)
                };
                draw_edge_label(&mut grid, label_row, turn_col + 1, label.trim());
            } else if placed {
                // graphviz label positions do not match placed nodes; write
                // the label into the edge where it leaves its source.
                draw_edge_label(&mut grid, start.0, start_col + 1, label.trim());
            } else if let Some(label_pos) = &edge.label_pos {
                let (row, center_col) = scale.to_grid(label_pos);
                let chars = label.chars().collect::<Vec<_>>();
                let label_start = center_col.saturating_sub(chars.len() / 2);
                for (offset, ch) in chars.into_iter().enumerate() {
                    grid.set(row, label_start + offset, hascii::grid::Cell::EdgeLabel(ch));
                }
            }
        }
    }
    grid.render()
}

/// Grid rectangles for every flowchart node plus the grid size they need.
struct FlowPlacement {
    rects: Vec<hascii::scale::GridRect>,
    width: usize,
    height: usize,
}

/// Place flowchart nodes on a character grid using only the rank and in-rank
/// order graphviz assigns, never its coordinates, so drawings stay identical
/// across graphviz releases whose spacing and spline routing differ. Each node
/// is centred on its parents and nodes sharing a rank are pushed apart in
/// graphviz order.
///
/// Returns `None` when an edge does not connect adjacent ranks (rank-skipping
/// or back edges), leaving those graphs to graphviz coordinates.
fn place_flow_nodes(
    graph: &hascii::model::LayoutGraph,
    horizontal: bool,
    node_sep: f64,
    rank_sep: f64,
) -> Option<FlowPlacement> {
    const NODE_HEIGHT: i64 = 3;
    let count = graph.nodes.len();
    if count == 0 {
        return None;
    }
    // graphviz puts every node of a rank on one line: the same y top-to-bottom
    // (larger y nearer the top), the same x left-to-right.
    let rank_key = |index: usize| {
        let center = graph.nodes[index].center;
        if horizontal {
            center.x.round() as i64
        } else {
            -(center.y.round() as i64)
        }
    };
    let order_key = |index: usize| {
        let center = graph.nodes[index].center;
        if horizontal {
            -center.y
        } else {
            center.x
        }
    };
    let mut rank_keys = (0..count).map(rank_key).collect::<Vec<_>>();
    rank_keys.sort_unstable();
    rank_keys.dedup();
    let rank_of = (0..count)
        .map(|index| rank_keys.binary_search(&rank_key(index)).ok())
        .collect::<Option<Vec<_>>>()?;
    if graph
        .edges
        .iter()
        .any(|edge| rank_of[edge.head_idx] != rank_of[edge.tail_idx] + 1)
    {
        return None;
    }
    let mut ranks = vec![Vec::new(); rank_keys.len()];
    for index in 0..count {
        ranks[rank_of[index]].push(index);
    }
    for rank in &mut ranks {
        rank.sort_by(|&a, &b| order_key(a).total_cmp(&order_key(b)));
    }

    let widths = graph
        .nodes
        .iter()
        .map(|node| node.label.trim().chars().count() as i64 + 4)
        .collect::<Vec<_>>();
    let cross_size = |index: usize| {
        if horizontal {
            NODE_HEIGHT
        } else {
            widths[index]
        }
    };
    let cross_gap = if horizontal {
        ((node_sep * 4.0).round() as i64).max(1)
    } else {
        ((node_sep * 10.0).round() as i64).max(2)
    };
    let mut parents = vec![Vec::new(); count];
    for edge in &graph.edges {
        parents[edge.head_idx].push(edge.tail_idx);
    }

    // centre each node on its parents, push apart in graphviz order, then
    // shift the rank back so it sits where its nodes asked to be.
    let mut centers = vec![0i64; count];
    for rank in &ranks {
        let wanted = rank
            .iter()
            .map(|&index| {
                (!parents[index].is_empty()).then(|| {
                    parents[index]
                        .iter()
                        .map(|&parent| centers[parent])
                        .sum::<i64>()
                        / parents[index].len() as i64
                })
            })
            .collect::<Vec<_>>();
        let mut placed = Vec::<i64>::with_capacity(rank.len());
        for (position, &index) in rank.iter().enumerate() {
            let mut center = wanted[position].unwrap_or(0);
            if position > 0 {
                let previous = rank[position - 1];
                let previous_end =
                    placed[position - 1] - cross_size(previous) / 2 + cross_size(previous) - 1;
                let mut nearest = previous_end + 1 + cross_gap + cross_size(index) / 2;
                // keep sibling centres an even distance apart so a pair can
                // sit exactly either side of the parent they share.
                nearest += (nearest - placed[position - 1]).rem_euclid(2);
                center = center.max(nearest);
            }
            placed.push(center);
        }
        let asked = wanted
            .iter()
            .zip(&placed)
            .filter_map(|(wanted, placed)| wanted.map(|wanted| (wanted, *placed)))
            .collect::<Vec<_>>();
        let shift = if asked.is_empty() {
            0
        } else {
            asked
                .iter()
                .map(|(wanted, placed)| wanted - placed)
                .sum::<i64>()
                / asked.len() as i64
        };
        for (&index, center) in rank.iter().zip(placed) {
            centers[index] = center + shift;
        }
    }
    let cross_starts = (0..count)
        .map(|index| centers[index] - cross_size(index) / 2)
        .collect::<Vec<_>>();
    let cross_offset = -cross_starts.iter().copied().min()?;

    // ranks follow one another with room for the longest edge label between.
    let rank_sizes = ranks
        .iter()
        .map(|rank| {
            if horizontal {
                rank.iter().map(|&index| widths[index]).max().unwrap_or(0)
            } else {
                NODE_HEIGHT
            }
        })
        .collect::<Vec<_>>();
    let mut rank_starts = vec![0i64; ranks.len()];
    for rank in 1..ranks.len() {
        let gap = if horizontal {
            let longest_label = graph
                .edges
                .iter()
                .filter(|edge| rank_of[edge.tail_idx] == rank - 1)
                .filter_map(|edge| edge.label.as_deref())
                .map(|label| label.trim().chars().count() as i64)
                .max()
                .unwrap_or(0);
            // a labelled fork turns halfway across the gap, so the first half
            // must hold "- label -" before the corner.
            ((rank_sep * 16.0).round() as i64)
                .max(8)
                .max(2 * longest_label + 9)
        } else {
            ((rank_sep * 5.0).round() as i64).max(5)
        };
        rank_starts[rank] = rank_starts[rank - 1] + rank_sizes[rank - 1] + gap;
    }

    let rects = (0..count)
        .map(|index| {
            let rank = rank_of[index];
            let cross = (cross_starts[index] + cross_offset) as usize;
            if horizontal {
                hascii::scale::GridRect {
                    left: (rank_starts[rank] + (rank_sizes[rank] - widths[index]) / 2) as usize,
                    top: cross,
                    width: widths[index] as usize,
                    height: NODE_HEIGHT as usize,
                }
            } else {
                hascii::scale::GridRect {
                    left: cross,
                    top: rank_starts[rank] as usize,
                    width: widths[index] as usize,
                    height: NODE_HEIGHT as usize,
                }
            }
        })
        .collect::<Vec<_>>();
    let width = rects.iter().map(|rect| rect.right() + 1).max()?;
    let height = rects.iter().map(|rect| rect.bottom() + 1).max()?;
    Some(FlowPlacement {
        rects,
        width,
        height,
    })
}

fn draw_flow_boxes(
    grid: &mut hascii::grid::CharGrid,
    rects: &[hascii::scale::GridRect],
    nodes: &[hascii::model::Node],
) {
    use hascii::grid::Cell;
    for (rect, node) in rects.iter().zip(nodes) {
        for row in [rect.top, rect.bottom()] {
            grid.draw_hline(row, rect.left, rect.right(), '-');
            grid.set(row, rect.left, Cell::NodeContent('+'));
            grid.set(row, rect.right(), Cell::NodeContent('+'));
        }
        grid.draw_text(
            rect.top + 1,
            rect.left,
            &format!("| {} |", node.label.trim()),
        );
    }
}

/// Draw a top-to-bottom flowchart with square forks and merges between ranks.
/// Returns `None` when [`place_flow_nodes`] cannot place the graph.
fn render_vertical_layout(
    graph: &hascii::model::LayoutGraph,
    node_sep: f64,
    rank_sep: f64,
) -> Option<String> {
    let placement = place_flow_nodes(graph, false, node_sep, rank_sep)?;
    let rects = &placement.rects;
    let mut grid = hascii::grid::CharGrid::new(placement.width, placement.height);
    draw_flow_boxes(&mut grid, rects, &graph.nodes);

    let mut outgoing = vec![0usize; graph.nodes.len()];
    let mut incoming = vec![0usize; graph.nodes.len()];
    for edge in &graph.edges {
        outgoing[edge.tail_idx] += 1;
        incoming[edge.head_idx] += 1;
    }
    for edge in &graph.edges {
        let source = &rects[edge.tail_idx];
        let target = &rects[edge.head_idx];
        let below_source = source.bottom() + 1;
        let arrow_row = target.top - 1;
        let fork_row = below_source + 1;
        let merge_row = arrow_row - 1;
        let (source_col, target_col) = (source.center_col(), target.center_col());
        let (path, label_col) = if source_col == target_col {
            (
                vec![(below_source, source_col), (arrow_row, target_col)],
                target_col,
            )
        } else if outgoing[edge.tail_idx] > 1 || incoming[edge.head_idx] < 2 {
            (
                vec![
                    (below_source, source_col),
                    (fork_row, source_col),
                    (fork_row, target_col),
                    (arrow_row, target_col),
                ],
                target_col,
            )
        } else {
            (
                vec![
                    (below_source, source_col),
                    (merge_row, source_col),
                    (merge_row, target_col),
                    (arrow_row, target_col),
                ],
                source_col,
            )
        };
        draw_flow_edge_path(&mut grid, &path);
        grid.set(arrow_row, target_col, hascii::grid::Cell::Arrow('v'));
        if let Some(label) = edge
            .label
            .as_deref()
            .map(str::trim)
            .filter(|label| !label.is_empty())
        {
            draw_edge_label(&mut grid, fork_row + 1, label_col + 1, label);
        }
    }
    Some(grid.render())
}

fn draw_edge_label(grid: &mut hascii::grid::CharGrid, row: usize, col: usize, label: &str) {
    grid.set(row, col, hascii::grid::Cell::EdgeLabel(' '));
    for (offset, ch) in label.chars().enumerate() {
        grid.set(row, col + offset + 1, hascii::grid::Cell::EdgeLabel(ch));
    }
    grid.set(
        row,
        col + label.chars().count() + 1,
        hascii::grid::Cell::EdgeLabel(' '),
    );
}

fn draw_flow_edge_path(grid: &mut hascii::grid::CharGrid, path: &[(usize, usize)]) {
    for segment in path.windows(2) {
        let (from_row, from_col) = segment[0];
        let (to_row, to_col) = segment[1];
        let distance = from_row.abs_diff(to_row) + from_col.abs_diff(to_col);
        for step in 0..=distance {
            let row = if to_row >= from_row {
                from_row + step.min(to_row - from_row)
            } else {
                from_row - step.min(from_row - to_row)
            };
            let col = if to_col >= from_col {
                from_col + step.min(to_col - from_col)
            } else {
                from_col - step.min(from_col - to_col)
            };
            let mut links = hascii::chars::Connectivity::default();
            if step > 0 {
                if row < from_row || col < from_col {
                    if row < from_row {
                        links.down = true;
                    } else {
                        links.right = true;
                    }
                } else if row > from_row || col > from_col {
                    if row > from_row {
                        links.up = true;
                    } else {
                        links.left = true;
                    }
                }
            }
            if step < distance {
                if to_row < from_row || to_col < from_col {
                    if to_row < from_row {
                        links.up = true;
                    } else {
                        links.left = true;
                    }
                } else if to_row > from_row || to_col > from_col {
                    if to_row > from_row {
                        links.down = true;
                    } else {
                        links.right = true;
                    }
                }
            }
            grid.draw_edge_segment(row, col, links);
        }
    }
}

fn parse_bubble(
    value: &str,
    rng: &mut BubbleRng,
    config: &ComposeConfig,
) -> Result<Bubble, String> {
    let mut speaker = None;
    let mut style = None;
    let mut speaker_side = None;
    let mut bubble_side = None;
    let mut alignment = None;
    let first_key = value.split_once('=').map(|(key, _)| key);
    let structured = matches!(
        first_key,
        Some("speaker" | "style" | "speaker-side" | "bubble-side" | "float" | "phrase")
    );
    let mut phrase = if structured { None } else { Some(value) };
    let mut seen = BTreeSet::new();

    for clause in value.split(';').filter(|_| structured) {
        let Some((key, setting)) = clause.split_once('=') else {
            return Err("--bubble clauses must use name=value".into());
        };
        if !seen.insert(key) {
            return Err(format!("--bubble {key} may only be specified once"));
        }
        match key {
            "speaker" => {
                speaker = Some(bubble_speaker(setting).ok_or_else(|| {
                    "--bubble speaker must be cat, dog, stick-figure, cow, robot, owl, dragon, penguin, duck, or ghost".to_owned()
                })?);
            }
            "style" => {
                style = Some(match setting {
                    "speech" | "voice" => BubbleStyle::Speech,
                    "thought" => BubbleStyle::Thought,
                    _ => {
                        return Err("--bubble style must be 'voice', 'speech', or 'thought'".into())
                    }
                });
            }
            "speaker-side" => {
                speaker_side = Some(match setting {
                    "below" => SpeakerSide::Below,
                    "left" => SpeakerSide::Left,
                    "right" => SpeakerSide::Right,
                    _ => {
                        return Err(
                            "--bubble speaker-side must be 'left', 'right', or 'below'".into()
                        )
                    }
                });
            }
            "bubble-side" => {
                bubble_side = Some(match setting {
                    "left" => BubbleSide::Left,
                    "center" => BubbleSide::Center,
                    "right" => BubbleSide::Right,
                    _ => {
                        return Err(
                            "--bubble bubble-side must be 'left', 'center', or 'right'".into()
                        )
                    }
                });
            }
            "float" => {
                alignment = Some(match setting {
                    "left" => ChartAlignment::Left,
                    "center" => ChartAlignment::Center,
                    "right" => ChartAlignment::Right,
                    _ => return Err("--bubble float must be 'left', 'center', or 'right'".into()),
                });
            }
            "phrase" => phrase = Some(setting),
            _ => return Err(format!("--bubble has unknown option '{key}'")),
        }
    }

    let phrase = phrase.ok_or_else(|| "--bubble requires phrase=...".to_owned())?;
    if phrase.trim().is_empty() {
        return Err("--bubble phrase cannot be empty".into());
    }
    Ok(Bubble {
        speaker: speaker
            .or(config.bubble_speaker)
            .unwrap_or_else(|| random_bubble_speaker(rng)),
        speaker_side: speaker_side
            .or(config.bubble_speaker_side)
            .unwrap_or(SpeakerSide::Below),
        bubble_side: bubble_side
            .or(config.bubble_side)
            .unwrap_or_else(|| random_bubble_side(rng)),
        style: style
            .or(config.bubble_style)
            .unwrap_or_else(|| random_bubble_style(rng)),
        alignment: alignment
            .or(config.bubble_float)
            .unwrap_or_else(|| random_chart_alignment(rng)),
        phrase: phrase.to_owned(),
    })
}

fn random_bubble_speaker(rng: &mut BubbleRng) -> BubbleSpeaker {
    match rng.index(10) {
        0 => BubbleSpeaker::Cat,
        1 => BubbleSpeaker::Dog,
        2 => BubbleSpeaker::StickFigure,
        3 => BubbleSpeaker::Cow,
        4 => BubbleSpeaker::Robot,
        5 => BubbleSpeaker::Owl,
        6 => BubbleSpeaker::Dragon,
        7 => BubbleSpeaker::Penguin,
        8 => BubbleSpeaker::Duck,
        _ => BubbleSpeaker::Ghost,
    }
}

fn random_bubble_side(rng: &mut BubbleRng) -> BubbleSide {
    match rng.index(3) {
        0 => BubbleSide::Left,
        1 => BubbleSide::Center,
        _ => BubbleSide::Right,
    }
}

fn random_bubble_style(rng: &mut BubbleRng) -> BubbleStyle {
    if rng.index(2) == 0 {
        BubbleStyle::Speech
    } else {
        BubbleStyle::Thought
    }
}

fn random_chart_alignment(rng: &mut BubbleRng) -> ChartAlignment {
    match rng.index(3) {
        0 => ChartAlignment::Left,
        1 => ChartAlignment::Center,
        _ => ChartAlignment::Right,
    }
}

fn format_bar_chart(chart: &BarChart, wrap_width: usize, charset: Charset) -> String {
    let rendered = match chart.orientation {
        ChartOrientation::Horizontal => format_horizontal_chart(chart, charset),
        ChartOrientation::Vertical => format_vertical_chart(chart, charset),
    };
    align_block(&rendered, chart.alignment, wrap_width)
}

fn format_flow_chart(chart: &FlowChart, wrap_width: usize, charset: Charset) -> String {
    let palette = charset.palette();
    let box_width = chart
        .steps
        .iter()
        .map(|step| display_width(step))
        .max()
        .unwrap_or(0);
    let top = format!(
        "{}{}{}",
        palette.top.0,
        palette.horizontal.repeat(box_width + 2),
        palette.top.2
    );
    let bottom = format!(
        "{}{}{}",
        palette.bottom.0,
        palette.horizontal.repeat(box_width + 2),
        palette.bottom.2
    );
    let render_box = |step: &str| {
        (
            top.clone(),
            format!(
                "{} {} {}",
                palette.vertical,
                pad_centered(step, box_width),
                palette.vertical
            ),
            bottom.clone(),
        )
    };
    let rendered = match chart.orientation {
        ChartOrientation::Horizontal => {
            let boxes = chart.steps.iter().map(|step| render_box(step));
            let boxes = boxes.collect::<Vec<_>>();
            let arrow = palette.arrow_right;
            let spacer = " ".repeat(display_width(arrow));
            [
                boxes
                    .iter()
                    .map(|(top, _, _)| top.as_str())
                    .collect::<Vec<_>>()
                    .join(&spacer),
                boxes
                    .iter()
                    .map(|(_, middle, _)| middle.as_str())
                    .collect::<Vec<_>>()
                    .join(arrow),
                boxes
                    .iter()
                    .map(|(_, _, bottom)| bottom.as_str())
                    .collect::<Vec<_>>()
                    .join(&spacer),
            ]
            .join("\n")
        }
        ChartOrientation::Vertical => {
            let mut lines = Vec::new();
            for (index, step) in chart.steps.iter().enumerate() {
                let (top, middle, bottom) = render_box(step);
                lines.extend([top, middle, bottom]);
                if index + 1 < chart.steps.len() {
                    let connector_indent = " ".repeat(box_width / 2 + 2);
                    lines.push(format!("{connector_indent}{}", palette.vertical));
                    lines.push(format!("{connector_indent}{}", palette.arrow_down));
                }
            }
            lines.join("\n")
        }
    };
    align_block(&rendered, chart.alignment, wrap_width)
}

fn format_bubble(bubble: &Bubble, wrap_width: usize) -> String {
    let avatar = bubble_avatar(bubble.speaker);
    let avatar_width = avatar
        .iter()
        .map(|line| display_width(line))
        .max()
        .unwrap_or(0);
    let head_anchor = match bubble.speaker {
        BubbleSpeaker::Cat => 3,
        BubbleSpeaker::Dog => 4,
        BubbleSpeaker::StickFigure => 2,
        BubbleSpeaker::Cow => 3,
        BubbleSpeaker::Robot => 3,
        BubbleSpeaker::Owl => 3,
        BubbleSpeaker::Dragon => 4,
        BubbleSpeaker::Penguin => 3,
        BubbleSpeaker::Duck => 3,
        BubbleSpeaker::Ghost => 3,
    };
    // Leave room for the frame and either the horizontal connector/avatar or
    // the part of an avatar that extends beyond a bubble above it.
    let reserved_width = match bubble.speaker_side {
        SpeakerSide::Below => match bubble.bubble_side {
            BubbleSide::Left => 4 + avatar_width.saturating_sub(head_anchor + 1),
            BubbleSide::Center => 4,
            BubbleSide::Right => 4 + head_anchor.saturating_sub(1),
        },
        SpeakerSide::Left | SpeakerSide::Right => avatar_width + 11,
    };
    let phrase_lines = wrap_paragraph_at(&bubble.phrase, wrap_width.saturating_sub(reserved_width));
    let phrase_lines = phrase_lines.lines().collect::<Vec<_>>();
    let phrase_width = phrase_lines
        .iter()
        .map(|line| display_width(line))
        .max()
        .unwrap_or(1)
        .max(1);
    let lines = match bubble.style {
        BubbleStyle::Speech => {
            let border = "-".repeat(phrase_width);
            let mut lines = vec![format!(" .{border}.")];
            lines.extend(
                phrase_lines
                    .iter()
                    .map(|line| format!("( {} )", pad_left_aligned(line, phrase_width))),
            );
            lines.push(format!(" '{border}'"));
            lines
        }
        BubbleStyle::Thought => phrase_lines
            .iter()
            .map(|line| format!("( {} )", pad_left_aligned(line, phrase_width)))
            .collect(),
    };
    let bubble_width = phrase_width + 4;
    let rendered = match bubble.speaker_side {
        SpeakerSide::Below => {
            let bubble_anchor = match bubble.bubble_side {
                BubbleSide::Left => bubble_width - 1,
                BubbleSide::Center => bubble_width / 2,
                BubbleSide::Right => 1,
            };
            let (bubble_x, avatar_x) = if head_anchor >= bubble_anchor {
                (head_anchor - bubble_anchor, 0)
            } else {
                (0, bubble_anchor - head_anchor)
            };
            let canvas_width = (bubble_x + bubble_width).max(avatar_x + avatar_width);
            let mut output = lines
                .iter()
                .map(|line| format!("{}{line}", " ".repeat(bubble_x)))
                .collect::<Vec<_>>();
            let head_x = avatar_x + head_anchor;
            match bubble.style {
                BubbleStyle::Speech => {
                    let mark = match bubble.bubble_side {
                        BubbleSide::Right => '/',
                        BubbleSide::Left | BubbleSide::Center => '\\',
                    };
                    output.push(place_bubble_mark(canvas_width, head_x, mark));
                }
                BubbleStyle::Thought => {
                    for mark in ['O', 'o', '.'] {
                        output.push(place_bubble_mark(canvas_width, head_x, mark));
                    }
                }
            }
            output.extend(
                avatar
                    .iter()
                    .map(|line| format!("{}{line}", " ".repeat(avatar_x))),
            );
            output.join("\n")
        }
        SpeakerSide::Left | SpeakerSide::Right => {
            let height = lines.len().max(avatar.len());
            let bubble_top_padding = (height - lines.len()) / 2;
            let top_padding = (height - avatar.len()) / 2;
            let bubble_row = bubble_top_padding + lines.len() / 2;
            let compact_thought = bubble.style == BubbleStyle::Thought
                && !matches!(
                    bubble.speaker,
                    BubbleSpeaker::Cat | BubbleSpeaker::Dog | BubbleSpeaker::StickFigure
                )
                && avatar_width + bubble_width + 7 > wrap_width;
            let connector = match (bubble.style, bubble.speaker_side) {
                (BubbleStyle::Speech, SpeakerSide::Left) => " <-- ",
                (BubbleStyle::Speech, SpeakerSide::Right) => " --> ",
                (BubbleStyle::Speech, SpeakerSide::Below) => unreachable!(),
                (BubbleStyle::Thought, SpeakerSide::Left | SpeakerSide::Right)
                    if compact_thought =>
                {
                    " o "
                }
                (BubbleStyle::Thought, SpeakerSide::Left) => " . o O ",
                (BubbleStyle::Thought, SpeakerSide::Right) => " O o . ",
                (BubbleStyle::Thought, SpeakerSide::Below) => unreachable!(),
            };
            let blank_spacer = " ".repeat(connector.len());
            (0..height)
                .map(|row| {
                    let bubble_line = row
                        .checked_sub(bubble_top_padding)
                        .and_then(|index| lines.get(index))
                        .map(String::as_str)
                        .unwrap_or("");
                    let avatar_line = row
                        .checked_sub(top_padding)
                        .and_then(|index| avatar.get(index))
                        .copied()
                        .unwrap_or("");
                    let spacer = if row == bubble_row {
                        connector
                    } else {
                        &blank_spacer
                    };
                    match bubble.speaker_side {
                        SpeakerSide::Left => format!(
                            "{avatar_line:<avatar_width$}{spacer}{bubble_line}",
                            avatar_width = avatar_width
                        ),
                        SpeakerSide::Right => format!(
                            "{}{spacer}{avatar_line:<avatar_width$}",
                            pad_left_aligned(bubble_line, bubble_width)
                        ),
                        SpeakerSide::Below => unreachable!(),
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
    };
    align_block(&rendered, bubble.alignment, wrap_width)
}

fn place_bubble_mark(width: usize, position: usize, mark: char) -> String {
    let mut line = vec![' '; width.max(position + 1)];
    line[position] = mark;
    line.into_iter().collect()
}

fn bubble_avatar(speaker: BubbleSpeaker) -> &'static [&'static str] {
    match speaker {
        BubbleSpeaker::Cat => &[" /\\_/\\", "( o.o )", " > ^ <"],
        BubbleSpeaker::Dog => &[
            " / \\__",
            "(    @\\___",
            " /         O",
            "/   (_____/",
            "/_____/   U",
        ],
        BubbleSpeaker::StickFigure => &["  O", " /|\\", " / \\"],
        BubbleSpeaker::Cow => &[" ^__^", " (oo)\\__", " (__)\\  )", "     ||||"],
        BubbleSpeaker::Robot => &[" .---.", " |o o|", " |_-_|", " /| |\\"],
        BubbleSpeaker::Owl => &[" ,_,", " (O,O)", " (   )", "  \" \""],
        BubbleSpeaker::Dragon => &[r"  /\ /\", r" / o o \", "(   ^   )", r" \__V__/"],
        BubbleSpeaker::Penguin => &["  .--.", " (o  o)", " /|  |\\", "  _||_"],
        BubbleSpeaker::Duck => &["  __", " (o )>", " /  )", " `--'"],
        BubbleSpeaker::Ghost => &[" .-.", "(o o)", "| O |", "|/\\/|"],
    }
}

fn align_block(block: &str, alignment: ChartAlignment, wrap_width: usize) -> String {
    let width = block.lines().map(display_width).max().unwrap_or(0);
    let padding = match alignment {
        ChartAlignment::Left => 0,
        ChartAlignment::Center => wrap_width.saturating_sub(width) / 2,
        ChartAlignment::Right => wrap_width.saturating_sub(width),
    };
    let prefix = " ".repeat(padding);
    block
        .lines()
        .map(|line| format!("{prefix}{}", line.trim_end()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_horizontal_chart(chart: &BarChart, charset: Charset) -> String {
    let palette = charset.palette();
    let label_width = chart
        .entries
        .iter()
        .map(|(label, _)| display_width(label))
        .max()
        .unwrap_or(0);
    let max_value = chart
        .entries
        .iter()
        .map(|(_, value)| *value)
        .max()
        .unwrap_or(0);
    chart
        .entries
        .iter()
        .map(|(label, value)| {
            let bar_length = if max_value == 0 || *value == 0 {
                0
            } else {
                (((*value as u128 * chart.size as u128) / max_value as u128) as usize).max(1)
            };
            format!(
                "{} {} {} {value}",
                pad_left_aligned(label, label_width),
                palette.vertical,
                palette.bar.repeat(bar_length)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_vertical_chart(chart: &BarChart, charset: Charset) -> String {
    let palette = charset.palette();
    let max_value = chart
        .entries
        .iter()
        .map(|(_, value)| *value)
        .max()
        .unwrap_or(0);
    let axis_width = max_value.to_string().chars().count().max(1);
    let cell_width = chart
        .entries
        .iter()
        .map(|(label, _)| display_width(label))
        .max()
        .unwrap_or(1)
        .max(1);
    let bar_row = |level: usize| {
        let bars = chart
            .entries
            .iter()
            .map(|(_, value)| {
                let bar_height = if max_value == 0 {
                    0
                } else {
                    ((*value as u128 * chart.size as u128 + max_value as u128 / 2)
                        / max_value as u128) as usize
                };
                let bar_height = if *value > 0 {
                    bar_height.max(1).min(chart.size)
                } else {
                    0
                };
                let center = cell_width / 2;
                let mut column = vec![' '; cell_width];
                if level <= bar_height {
                    column[center] = palette.bar.chars().next().unwrap_or('#');
                }
                column.into_iter().collect::<String>()
            })
            .collect::<Vec<_>>()
            .join(" ");
        bars
    };
    let mut lines = Vec::new();
    for level in (1..=chart.size).rev() {
        let axis = if level == chart.size {
            format!("{:>axis_width$} {} ", max_value, palette.vertical)
        } else {
            format!("{:>axis_width$} {} ", "", palette.vertical)
        };
        lines.push(format!("{axis}{}", bar_row(level)));
    }
    let baseline = palette
        .horizontal
        .repeat(chart.entries.len() * (cell_width + 1) - 1);
    lines.push(format!(
        "{:>axis_width$} {}{}{baseline}",
        "0",
        if charset == Charset::Ascii {
            "+"
        } else {
            "└"
        },
        palette.horizontal
    ));
    let labels = chart
        .entries
        .iter()
        .map(|(label, _)| pad_centered(label, cell_width))
        .collect::<Vec<_>>()
        .join(" ");
    lines.push(format!("{}   {labels}", " ".repeat(axis_width)));
    lines.join("\n")
}

fn format_list(items: &[ListEntry], wrap_width: usize) -> String {
    let mut counters = Vec::<usize>::new();
    let mut kinds = Vec::<Option<ListKind>>::new();
    let mut lines = Vec::new();
    for item in items {
        while counters.len() <= item.depth {
            counters.push(0);
            kinds.push(None);
        }
        if kinds[item.depth] != Some(item.kind) {
            counters[item.depth] = 0;
            kinds[item.depth] = Some(item.kind);
        }
        let marker = match item.kind {
            ListKind::Numbered => {
                counters[item.depth] += 1;
                format!("{}. ", counters[item.depth])
            }
            ListKind::Bullets => "* ".to_owned(),
            ListKind::Check(true) => "[x] ".to_owned(),
            ListKind::Check(false) => "[ ] ".to_owned(),
        };
        let indent = "  ".repeat(item.depth);
        let first_prefix = format!("{indent}{marker}");
        let continuation = " ".repeat(first_prefix.chars().count());
        let wrapped = wrap_paragraph_at(
            &item.text,
            wrap_width.saturating_sub(first_prefix.chars().count()),
        );
        let mut wrapped_lines = wrapped.lines();
        lines.push(format!(
            "{first_prefix}{}",
            wrapped_lines.next().unwrap_or("")
        ));
        lines.extend(wrapped_lines.map(|line| format!("{continuation}{line}")));
    }
    lines.join("\n")
}

fn find_footer_references(text: &str) -> BTreeSet<usize> {
    let bytes = text.as_bytes();
    let mut references = BTreeSet::new();
    let mut active_ticks = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let start = i;
            while i < bytes.len() && bytes[i] == b'`' {
                i += 1;
            }
            let run = i - start;
            if active_ticks == 0 {
                active_ticks = run;
            } else if run == active_ticks {
                active_ticks = 0;
            }
            continue;
        }
        if active_ticks == 0 && bytes[i] == b'[' && bytes.get(i + 1) == Some(&b'#') {
            let mut end = i + 2;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end > i + 2 && bytes.get(end) == Some(&b']') {
                if let Ok(number) = text[i + 2..end].parse::<usize>() {
                    references.insert(number);
                }
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }
    references
}

fn format_fields(fields: &[(String, String)], wrap_width: usize) -> String {
    let key_width = fields
        .iter()
        .map(|(key, _)| display_width(key))
        .max()
        .unwrap_or(0);
    let mut lines = Vec::new();
    for (key, value) in fields {
        let prefix = format!("{}: ", pad_left_aligned(key, key_width));
        let continuation = " ".repeat(key_width + 2);
        let wrapped = wrap_paragraph_at(value, wrap_width.saturating_sub(key_width + 2));
        let mut wrapped_lines = wrapped.lines();
        lines.push(format!("{prefix}{}", wrapped_lines.next().unwrap_or("")));
        lines.extend(wrapped_lines.map(|line| format!("{continuation}{line}")));
    }
    lines.join("\n")
}

fn format_callout(label: &str, text: &str, wrap_width: usize) -> String {
    let prefix = format!("{label}: ");
    let continuation = " ".repeat(prefix.chars().count());
    let wrapped = wrap_paragraph_at(text, wrap_width.saturating_sub(prefix.chars().count()));
    let mut lines = wrapped.lines();
    std::iter::once(format!("{prefix}{}", lines.next().unwrap_or("")))
        .chain(lines.map(|line| format!("{continuation}{line}")))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_message(
    subject: Option<String>,
    sections: Vec<Section>,
    footers: BTreeMap<usize, String>,
    wrap_width: usize,
    charset: Charset,
) -> Result<String, String> {
    let mut parts = Vec::new();
    if let Some(subject) = subject {
        parts.push(subject);
    }
    let mut references = BTreeSet::new();
    for section in sections {
        let formatted = match section {
            Section::Text(text) => {
                references.extend(find_footer_references(&text));
                wrap_paragraph_at(&text, wrap_width)
            }
            Section::Detail(text) => text
                .split('\n')
                .map(|line| format!("    {line}"))
                .collect::<Vec<_>>()
                .join("\n"),
            Section::Line(line) => {
                references.extend(find_footer_references(&line));
                line
            }
            Section::Fields(fields) => format_fields(&fields, wrap_width),
            Section::Note(text) => format_callout("Note", &text, wrap_width),
            Section::Warning(text) => format_callout("Warning", &text, wrap_width),
            Section::Table { head, rows } => format_table(&head, &rows, charset),
            Section::BarChart(chart) => format_bar_chart(&chart, wrap_width, charset),
            Section::Sparkline(chart) => format_sparkline(&chart, wrap_width, charset)?,
            Section::Progress(progress) => format_progress(&progress, wrap_width, charset)?,
            Section::Banner(banner) => format_banner(&banner, wrap_width, charset)?,
            Section::Badges(badges) => format_badges(&badges, wrap_width, charset)?,
            Section::Stamp(stamp) => format_stamp(&stamp, wrap_width)?,
            Section::FlowChart(chart) => format_flow_chart(&chart, wrap_width, charset),
            Section::FlowDiagram(diagram) => format_flow_diagram(&diagram, wrap_width, charset)?,
            Section::Bubble(bubble) => format_bubble(&bubble, wrap_width),
            Section::List(items) => format_list(&items, wrap_width),
        };
        parts.push(formatted);
    }
    if let Some(number) = references
        .iter()
        .find(|number| !footers.contains_key(number))
    {
        return Err(format!(
            "reference [#{number}] has no matching --compose-footer-{number}"
        ));
    }
    if let Some(number) = footers.keys().find(|number| !references.contains(number)) {
        return Err(format!(
            "--compose-footer-{number} has no matching [#{number}] reference"
        ));
    }
    if !footers.is_empty() {
        let number_width = footers
            .keys()
            .map(|number| number.to_string().len())
            .max()
            .unwrap_or(1);
        let label_width = number_width + 3;
        let lines = footers.iter().map(|(number, text)| {
            let label = format!("[#{number}]");
            let wrapped = wrap_paragraph_at(text, wrap_width.saturating_sub(label_width + 1));
            let mut wrapped_lines = wrapped.lines();
            let first = wrapped_lines.next().unwrap_or("");
            let continuation = " ".repeat(label_width + 1);
            std::iter::once(format!(
                "{label:<label_width$} {first}",
                label_width = label_width
            ))
            .chain(wrapped_lines.map(|line| format!("{continuation}{line}")))
            .collect::<Vec<_>>()
            .join("\n")
        });
        parts.push(lines.collect::<Vec<_>>().join("\n"));
    }
    Ok(parts.join("\n\n") + "\n")
}

fn format_flow_diagram(
    diagram: &FlowDiagram,
    wrap_width: usize,
    charset: Charset,
) -> Result<String, String> {
    if diagram.node_ids.len() < 2 {
        return Err("--flow-chart requires at least two declared nodes".into());
    }
    if diagram.edges.is_empty() {
        return Err("--flow-chart requires at least one --flow-edge".into());
    }
    for edge in &diagram.edges {
        for id in [&edge.from, &edge.to] {
            if !diagram.node_ids.contains(id) {
                return Err(format!("--flow-edge references undefined node '{id}'"));
            }
        }
    }

    // The flowchart grid stores one character per cell. Give each non-ASCII
    // grapheme a placeholder and reserve its remaining display cells; restore
    // the original text after routing and styling have finished.
    const FILLER: char = '\u{f0000}';
    let mut replacements = BTreeMap::new();
    let mut encode = |label: &str| -> Result<String, String> {
        let mut encoded = String::new();
        for grapheme in label.graphemes(true) {
            if grapheme.is_ascii() && (charset == Charset::Ascii || grapheme == " ") {
                encoded.push_str(grapheme);
                continue;
            }
            let index = u32::try_from(replacements.len())
                .map_err(|_| "too many distinct flowchart characters".to_owned())?;
            let placeholder = char::from_u32(0xf0001 + index)
                .filter(|ch| *ch <= '\u{ffffd}')
                .ok_or_else(|| "too many flowchart characters".to_owned())?;
            // A standalone combining mark has no terminal width, but the
            // grid still needs a cell for it. Keep that cell as a space.
            let restored = if display_width(grapheme) == 0 {
                format!("{grapheme} ")
            } else {
                grapheme.to_owned()
            };
            replacements.insert(placeholder, restored);
            encoded.push(placeholder);
            for _ in 1..display_width(grapheme) {
                encoded.push(FILLER);
            }
        }
        Ok(encoded)
    };
    let mut encoded_diagram = diagram.clone();
    for node in &mut encoded_diagram.nodes {
        node.label = encode(&node.label)?;
    }
    for edge in &mut encoded_diagram.edges {
        if let Some(label) = &mut edge.label {
            *label = encode(label)?;
        }
    }
    let diagram = &encoded_diagram;

    let mut dot = format!(
        "digraph {{\n    graph [rankdir={}, nodesep={}, ranksep={}];\n",
        diagram.direction, diagram.node_sep, diagram.rank_sep
    );
    for node in &diagram.nodes {
        let shape = match node.shape {
            FlowNodeShape::Box => "box",
            FlowNodeShape::Decision => "diamond",
            FlowNodeShape::Terminal => "oval",
        };
        dot.push_str(&format!(
            "    {} [label=\"{}\", shape={}];\n",
            node.id,
            escape_dot_label(&node.label),
            shape
        ));
    }
    for edge in &diagram.edges {
        match &edge.label {
            Some(label) => dot.push_str(&format!(
                "    {} -> {} [label=\"{}\"];\n",
                edge.from,
                edge.to,
                escape_dot_label(&format!(" {label} "))
            )),
            None => dot.push_str(&format!("    {} -> {};\n", edge.from, edge.to)),
        }
    }
    dot.push_str("}\n");

    let layout = || {
        hascii::graphviz::layout(&dot)
            .map_err(|error| format!("could not render --flow-chart: {error}"))
    };
    let rendered = if diagram.direction == "LR" {
        render_horizontal_layout(&layout()?, diagram.node_sep, diagram.rank_sep)
    } else if let Some(rendered) =
        render_vertical_layout(&layout()?, diagram.node_sep, diagram.rank_sep)
    {
        rendered
    } else {
        let options = hascii::RenderOptions {
            border_mode: hascii::BorderMode::Plain,
            ..Default::default()
        };
        hascii::render(&dot, &options)
            .map_err(|error| format!("could not render --flow-chart: {error}"))?
    };
    let rendered = to_ascii_diagram(rendered.trim_end());
    let rendered = style_flow_nodes(&rendered, &diagram.nodes);
    let rendered = if charset == Charset::Unicode {
        unicode_flow_diagram(&rendered)
    } else {
        rendered
    };
    let mut decoded = String::new();
    for ch in rendered.chars() {
        if ch == FILLER {
            continue;
        }
        if let Some(grapheme) = replacements.get(&ch) {
            decoded.push_str(grapheme);
        } else {
            decoded.push(ch);
        }
    }
    Ok(align_block(&decoded, diagram.alignment, wrap_width))
}

fn unicode_flow_diagram(rendered: &str) -> String {
    let rows: Vec<Vec<char>> = rendered
        .lines()
        .map(|line| line.chars().collect())
        .collect();
    let at = |row: isize, col: isize| -> Option<char> {
        if row < 0 || col < 0 {
            return None;
        }
        rows.get(row as usize)
            .and_then(|line| line.get(col as usize))
            .copied()
    };
    rows.iter()
        .enumerate()
        .map(|(row, line)| {
            line.iter()
                .enumerate()
                .map(|(col, ch)| match ch {
                    '+' => {
                        let (row, col) = (row as isize, col as isize);
                        let left = matches!(at(row, col - 1), Some('-' | '+' | '<' | '>'));
                        let right = matches!(at(row, col + 1), Some('-' | '+' | '<' | '>'));
                        let up = matches!(at(row - 1, col), Some('|' | '+' | '^' | 'v'));
                        let down = matches!(at(row + 1, col), Some('|' | '+' | '^' | 'v'));
                        match (left, right, up, down) {
                            (false, true, false, true) => '┌',
                            (true, false, false, true) => '┐',
                            (false, true, true, false) => '└',
                            (true, false, true, false) => '┘',
                            (true, true, false, true) => '┬',
                            (true, true, true, false) => '┴',
                            (false, true, true, true) => '├',
                            (true, false, true, true) => '┤',
                            _ => '┼',
                        }
                    }
                    '-' => '─',
                    '|' => '│',
                    '>' => '▶',
                    '<' => '◀',
                    '^' => '▲',
                    'v' => '▼',
                    other => *other,
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn create_message_file(message: &str) -> io::Result<PathBuf> {
    let dir = env::temp_dir();
    for _ in 0..100 {
        let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = dir.join(format!(
            "git-compose-commit-{}-{id}.msg",
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                if let Err(error) = file.write_all(message.as_bytes()) {
                    drop(file);
                    let _ = fs::remove_file(&path);
                    return Err(error);
                }
                return Ok(path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a temporary commit message file",
    ))
}

fn run_inner() -> Result<ExitCode, String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if let Some(result) = agent_usage::handle(&args) {
        result?;
        return Ok(ExitCode::SUCCESS);
    }
    let (output_target, args) = extract_output_option(args)?;
    let (message, git_args) = compose(args)?;
    if let Some(output_target) = output_target {
        if !git_args.is_empty() {
            let option = match &output_target {
                OutputTarget::File(_) => "--output",
                OutputTarget::Stdout => "--stdout",
            };
            return Err(format!(
                "{option} cannot be combined with Git commit options"
            ));
        }
        match output_target {
            OutputTarget::File(output_path) => {
                fs::write(&output_path, message).map_err(|error| {
                    format!(
                        "could not write {}: {error}",
                        PathBuf::from(&output_path).display()
                    )
                })?;
            }
            OutputTarget::Stdout => io::stdout()
                .write_all(message.as_bytes())
                .map_err(|error| format!("could not write standard output: {error}"))?,
        }
        return Ok(ExitCode::SUCCESS);
    }
    let path = create_message_file(&message)
        .map_err(|e| format!("could not write temporary commit message: {e}"))?;

    let result = Command::new("git")
        .arg("commit")
        .arg("-F")
        .arg(&path)
        .args(git_args)
        .status();
    let cleanup = fs::remove_file(&path);
    match result {
        Ok(status) => {
            if let Err(error) = cleanup {
                eprintln!("git-compose-commit: could not remove temporary message file: {error}");
            }
            Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
        }
        Err(error) => {
            let _ = cleanup;
            Err(format!("could not run git commit: {error}"))
        }
    }
}

fn run_compose_text_inner() -> Result<ExitCode, String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if let Some(result) = agent_usage::handle(&args) {
        result?;
        return Ok(ExitCode::SUCCESS);
    }
    if args.len() == 1 {
        match args[0].to_str() {
            Some("-h" | "--help" | "--help=short" | "--help=long" | "--man") => {
                println!("{}", include_str!("help/compose-text.txt").trim_end());
                return Ok(ExitCode::SUCCESS);
            }
            Some("--version") => {
                println!("compose-text {}", env!("CARGO_PKG_VERSION"));
                return Ok(ExitCode::SUCCESS);
            }
            Some(value) if value.starts_with("--help=") => {
                let topic = &value["--help=".len()..];
                let help = topic_help(topic)?.replace(
                    "Use --help=long for the complete guide.",
                    "Use git-compose-commit --help for the complete guide.",
                );
                println!("Some examples use git-compose-commit; omit Git options for compose-text.\n\n{help}");
                return Ok(ExitCode::SUCCESS);
            }
            _ => {}
        }
    }
    let (target, args) = extract_output_option(args)?;
    let (message, git_args) = compose(args)?;
    if !git_args.is_empty() {
        return Err("compose-text accepts message options only, not Git options or paths".into());
    }
    match target.unwrap_or(OutputTarget::Stdout) {
        OutputTarget::File(path) => fs::write(&path, message).map_err(|error| {
            format!("could not write {}: {error}", PathBuf::from(path).display())
        })?,
        OutputTarget::Stdout => io::stdout()
            .write_all(message.as_bytes())
            .map_err(|error| format!("could not write standard output: {error}"))?,
    }
    Ok(ExitCode::SUCCESS)
}

/// Compose text to stdout or a file without invoking Git.
pub fn run_compose_text() -> ExitCode {
    match run_compose_text_inner() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("compose-text: {error}");
            eprintln!("Try 'compose-text --help' for details.");
            ExitCode::from(2)
        }
    }
}

/// Assemble a commit message and return the arguments to forward to `git commit`.
pub fn compose(args: Vec<OsString>) -> Result<(String, Vec<OsString>), String> {
    let help_requested = args
        .iter()
        .take_while(|arg| arg.as_os_str() != OsStr::new("--"))
        .any(|arg| match arg.to_str() {
            Some("-h" | "--help" | "--man" | "--version") => true,
            Some(value) => value.starts_with("--help="),
            None => false,
        });
    let config = if help_requested {
        ComposeConfig::default()
    } else {
        load_user_config()?
    };
    let ParsedArgs {
        subject,
        sections,
        footers,
        wrap_width,
        charset,
        git_args,
    } = parse_args(args, &config)?;
    let message = format_message(subject, sections, footers, wrap_width, charset)?;
    Ok((message, git_args))
}

/// Run `git commit` using the composed message and forwarded arguments.
pub fn run() -> ExitCode {
    match run_inner() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("git-compose-commit: {error}");
            eprintln!("Try 'git-compose-commit --help' for details.");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
