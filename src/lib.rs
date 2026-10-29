use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::sync::atomic::{AtomicU64, Ordering};

const WRAP_WIDTH: usize = 72;
const MAX_CHART_SIZE: usize = 256;
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
struct FlowChart {
    orientation: ChartOrientation,
    alignment: ChartAlignment,
    steps: Vec<String>,
}

#[derive(Debug)]
struct FlowDiagram {
    direction: String,
    alignment: ChartAlignment,
    node_sep: f64,
    rank_sep: f64,
    nodes: Vec<FlowNode>,
    edges: Vec<FlowEdge>,
    node_ids: BTreeSet<String>,
}

#[derive(Debug)]
struct FlowNode {
    id: String,
    label: String,
    shape: FlowNodeShape,
}

#[derive(Debug, PartialEq, Eq)]
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
enum ListKind {
    Numbered,
    Bullets,
}

#[derive(Debug)]
struct ListEntry {
    kind: ListKind,
    depth: usize,
    text: String,
}

fn synopsis() -> &'static str {
    "git-compose-commit - compose a Git commit message

Usage: git-compose-commit [COMPOSE OPTIONS] [GIT OPTIONS] [-- PATHSPEC...]

Common options: --subject, --text, --field, --note, --line, --th, --tr, --flow-chart
Other: --version
Use --help for the full option guide."
}

fn usage() -> &'static str {
    "git-compose-commit - compose a Git commit message

Usage: git-compose-commit [COMPOSE OPTIONS] [GIT OPTIONS] [-- PATHSPEC...]

Compose options:
  -h, --help=short     Show a concise synopsis
  --help, --help=long  Show this detailed guide
  --version            Show the program version
  --rand-seed N        Seed random formatting choices
  --subject TEXT       Set the subject; may be given once
  --text TEXT          Add a paragraph, wrapped at the message margin
  --field KEY:VALUE    Add an aligned key/value row
  --note TEXT          Add a wrapped, labeled note
  --warning TEXT       Add a wrapped, labeled warning
  --wrap-width N       Set the message margin (default 72; range 20-256)
  --output FILE        Write the message to FILE without running git commit
  --detail TEXT        Add an unwrapped block, indented by four spaces
  --line TEXT          Add one unwrapped line
  --th CELLS           Start a table with comma-separated headings
  --tr CELLS           Add one comma-separated row
  --table-rows ROWS    Add rows separated by semicolons
  --hbar-chart DATA    Add an ASCII horizontal bar chart
  --vbar-chart DATA    Add an ASCII vertical bar chart
  --hflow-chart DATA   Add an ASCII horizontal flow chart
  --vflow-chart DATA   Add an ASCII vertical flow chart
  --flow-chart OPTIONS Start a branching flowchart
  --flow-node ID:TEXT  Add a box to the active flowchart
  --flow-decision ID:TEXT Add a decision diamond
  --flow-terminal ID:TEXT Add a rounded start/end node
  --flow-edge EDGE     Add a DOT-style edge to the active flowchart
  --bubble OPTIONS     Add an ASCII speech or thought bubble
  --footer-N TEXT      Define the footer referenced by [#N]
  --list-item TEXT     Add an item; starts a numbered list if needed
  --numlist            Start a numbered list (also --list=numbered)
  --bullets            Start a bullet list (also --list=bullets)
  --list KIND          Start a list: numbered or bullets
  --list-end           Close the current list and return to its parent

Options may be written as --option=value or --option value. The subject is
always first. Body sections appear in argument order, separated by blank
lines. Footers are collected at the end in number order.

--wrap-width sets the global wrapping and alignment margin. It defaults to 72
columns and accepts values from 20 to 256.

The optional ~/.compose-commit.toml file sets user defaults. Command-line
options override configured defaults. See the README for supported settings.

--output=FILE (also --compose-output=FILE) writes the composed message to a
file and skips git commit. It cannot be combined with Git commit options.

--field=KEY:VALUE adds an aligned key/value row. Adjacent fields form one
block. Split at the first colon; keys and values must both be non-empty.
--note and --warning add labeled prose blocks with wrapped continuation lines.

Footer references are recognized in --text and --line. Every reference needs
a footer, and every footer must be referenced. References inside backticks
are ignored.

Tables split cells on commas and rows on semicolons. CSV quoting and escaping
are not supported. Starting a list inside an open list nests it; a new
message section closes open lists automatically.

Chart data items use label:value pairs separated by commas. With no chart
settings, the data list may be given directly. When settings are present,
include data=LABEL:VALUE,... as a clause. Clauses are separated by semicolons;
supported settings are float=left|center|right, height=N for vertical charts,
and width=N for horizontal charts. Charts are centered by default. Data needs
at least two items.

Flow chart items are comma-separated box labels, with the same optional
float=left|center|right and data= clauses. Use --hflow-chart for a horizontal
chain or --vflow-chart for a vertical chain. Flow charts need at least two
boxes and do not wrap labels.

For branching diagrams, --flow-chart=direction=vertical;float=center starts a
top-to-bottom directed graph. Use direction=horizontal for a left-to-right
branching graph. Add nodes with --flow-node=ID:TEXT,
--flow-decision=ID:TEXT, or --flow-terminal=ID:TEXT. Add links with --flow-edge='FROM -> TO'. A link
may include a DOT label, for example --flow-edge='FROM -> TO [label=\"yes\"]';
unlabeled chains such as A -> B -> C are also allowed. Optional nodesep=N and
ranksep=N settings control horizontal and vertical spacing. The graph closes
at the next non-flowchart option or end of arguments. DOT is generated
internally; arbitrary DOT input is not accepted.

Bubbles use speaker=cat|dog|stick-figure and phrase=TEXT, with optional
style=speech|thought, speaker-side=left|right|below,
bubble-side=left|center|right, and float=left|center|right. Any omitted
settings use the configured default or are chosen randomly. --rand-seed
controls all random choices made by this tool; the same arguments and seed
produce the same output.

Each compose option also accepts a --compose- prefix, including
--compose-numlist and --compose-bullets. The short table options also accept
--table-head/--table-row and --compose-th/--compose-tr. Unprefixed options
may conflict with future Git options; use the namespaced forms in scripts.

Other arguments are passed to git commit. The -- separator is passed through
to Git and means that following arguments are pathspecs."
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
                config.bubble_speaker =
                    Some(match parse_config_string(value, key, path)?.as_str() {
                        "cat" => BubbleSpeaker::Cat,
                        "dog" => BubbleSpeaker::Dog,
                        "stick-figure" => BubbleSpeaker::StickFigure,
                        _ => return Err(invalid("must be cat, dog, or stick-figure")),
                    });
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

fn extract_output_option(args: Vec<OsString>) -> Result<(Option<OsString>, Vec<OsString>), String> {
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
        let parsed =
            split_equals(arg, "--output").or_else(|| split_equals(arg, "--compose-output"));
        if let Some(inline) = parsed {
            if output.is_some() {
                return Err("--output may only be specified once".into());
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
            output = Some(path);
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

fn parse_args(
    args: Vec<OsString>,
    config: &ComposeConfig,
) -> Result<
    (
        Option<String>,
        Vec<Section>,
        BTreeMap<usize, String>,
        usize,
        Vec<OsString>,
    ),
    String,
> {
    let mut bubble_rng = BubbleRng::new(extract_rand_seed(&args)?.or(config.rand_seed));
    let mut subject = None;
    let mut sections = Vec::new();
    let mut footers = BTreeMap::new();
    let mut wrap_width = config.wrap_width.unwrap_or(WRAP_WIDTH);
    let mut saw_wrap_width = false;
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
        if arg == OsStr::new("--help") || arg == OsStr::new("--help=long") {
            println!("{}", usage());
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
            (|| {
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
            })()
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
    Ok((subject, sections, footers, wrap_width, git_args))
}

#[cfg(test)]
fn wrap_paragraph(text: &str) -> String {
    wrap_paragraph_at(text, WRAP_WIDTH)
}

fn wrap_paragraph_at(text: &str, width: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in words {
        let extra = word.chars().count() + usize::from(!line.is_empty());
        if !line.is_empty() && line.chars().count() + extra > width {
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

fn format_table(head: &[String], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = head.iter().map(|cell| cell.chars().count()).collect();
    for row in rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }
    let border = format!(
        "+{}+",
        widths
            .iter()
            .map(|width| "-".repeat(width + 2))
            .collect::<Vec<_>>()
            .join("+")
    );
    let render_row = |row: &[String]| {
        format!(
            "| {} |",
            row.iter()
                .zip(&widths)
                .map(|(cell, width)| format!("{cell:<width$}", width = *width))
                .collect::<Vec<_>>()
                .join(" | ")
        )
    };
    let mut lines = vec![border.clone(), render_row(head), border.clone()];
    lines.extend(rows.iter().map(|row| render_row(row)));
    lines.push(border);
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

fn render_horizontal_dot(dot: &str) -> Result<String, String> {
    let graph = hascii::graphviz::layout(dot)
        .map_err(|error| format!("could not lay out --flow-chart: {error}"))?;
    let labels = graph
        .nodes
        .iter()
        .map(|node| node.label.clone())
        .collect::<Vec<_>>();
    let scale = hascii::scale::ScaleParams::compute(&graph.bb, &graph.nodes, &labels);
    let rects = graph
        .nodes
        .iter()
        .zip(&labels)
        .map(|(node, label)| scale.node_rect(node, label))
        .collect::<Vec<_>>();
    let mut grid = hascii::grid::CharGrid::new(scale.grid_w, scale.grid_h);
    hascii::render_nodes::render_nodes(&mut grid, &graph.nodes, &labels, &scale, true);

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
    Ok(grid.render())
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
    let mut phrase = None;
    let mut seen = BTreeSet::new();

    for clause in value.split(';') {
        let Some((key, setting)) = clause.split_once('=') else {
            return Err("--bubble clauses must use name=value".into());
        };
        if !seen.insert(key) {
            return Err(format!("--bubble {key} may only be specified once"));
        }
        match key {
            "speaker" => {
                speaker = Some(match setting {
                    "cat" => BubbleSpeaker::Cat,
                    "dog" => BubbleSpeaker::Dog,
                    "stick-figure" => BubbleSpeaker::StickFigure,
                    _ => {
                        return Err(
                            "--bubble speaker must be 'cat', 'dog', or 'stick-figure'".into()
                        )
                    }
                });
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
            .unwrap_or_else(|| random_speaker_side(rng)),
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
    match rng.index(3) {
        0 => BubbleSpeaker::Cat,
        1 => BubbleSpeaker::Dog,
        _ => BubbleSpeaker::StickFigure,
    }
}

fn random_speaker_side(rng: &mut BubbleRng) -> SpeakerSide {
    match rng.index(3) {
        0 => SpeakerSide::Below,
        1 => SpeakerSide::Left,
        _ => SpeakerSide::Right,
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

fn format_bar_chart(chart: &BarChart, wrap_width: usize) -> String {
    let rendered = match chart.orientation {
        ChartOrientation::Horizontal => format_horizontal_chart(chart),
        ChartOrientation::Vertical => format_vertical_chart(chart),
    };
    align_block(&rendered, chart.alignment, wrap_width)
}

fn format_flow_chart(chart: &FlowChart, wrap_width: usize) -> String {
    let box_width = chart
        .steps
        .iter()
        .map(|step| step.chars().count())
        .max()
        .unwrap_or(0);
    let border = format!("+{}+", "-".repeat(box_width + 2));
    let render_box = |step: &str| {
        (
            border.clone(),
            format!("| {step:^box_width$} |"),
            border.clone(),
        )
    };
    let rendered = match chart.orientation {
        ChartOrientation::Horizontal => {
            let boxes = chart.steps.iter().map(|step| render_box(step));
            let boxes = boxes.collect::<Vec<_>>();
            vec![
                boxes
                    .iter()
                    .map(|(top, _, _)| top.as_str())
                    .collect::<Vec<_>>()
                    .join("      "),
                boxes
                    .iter()
                    .map(|(_, middle, _)| middle.as_str())
                    .collect::<Vec<_>>()
                    .join(" --> "),
                boxes
                    .iter()
                    .map(|(_, _, bottom)| bottom.as_str())
                    .collect::<Vec<_>>()
                    .join("      "),
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
                    lines.push(format!("{connector_indent}|"));
                    lines.push(format!("{connector_indent}v"));
                }
            }
            lines.join("\n")
        }
    };
    align_block(&rendered, chart.alignment, wrap_width)
}

fn format_bubble(bubble: &Bubble, wrap_width: usize) -> String {
    let phrase_lines = wrap_paragraph_at(&bubble.phrase, wrap_width.saturating_sub(18));
    let phrase_lines = phrase_lines.lines().collect::<Vec<_>>();
    let phrase_width = phrase_lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(1)
        .max(1);
    let lines = match bubble.style {
        BubbleStyle::Speech => {
            let border = "-".repeat(phrase_width + 1);
            let mut lines = vec![format!(" .{border}.")];
            lines.extend(
                phrase_lines
                    .iter()
                    .map(|line| format!("( {line:<phrase_width$} )")),
            );
            lines.push(format!(" '{border}'"));
            lines
        }
        BubbleStyle::Thought => phrase_lines
            .iter()
            .map(|line| format!("( {line:<phrase_width$} )"))
            .collect(),
    };
    let bubble_width = phrase_width + 4;
    let avatar = bubble_avatar(bubble.speaker);
    let rendered = match bubble.speaker_side {
        SpeakerSide::Below => {
            let avatar_width = avatar
                .iter()
                .map(|line| line.chars().count())
                .max()
                .unwrap_or(0);
            let offset = if bubble.bubble_side == BubbleSide::Center {
                0
            } else {
                4
            };
            let canvas_width = bubble_width.max(avatar_width) + offset;
            let (bubble_x, avatar_x) = match bubble.bubble_side {
                BubbleSide::Center => (
                    (canvas_width - bubble_width) / 2,
                    (canvas_width - avatar_width) / 2,
                ),
                BubbleSide::Left => (0, 4),
                BubbleSide::Right => (4, 0),
            };
            let mut output = lines
                .iter()
                .map(|line| format!("{}{line}", " ".repeat(bubble_x)))
                .collect::<Vec<_>>();
            let bubble_center = bubble_x + bubble_width / 2;
            let speaker_center = avatar_x + avatar_width / 2;
            let direction = if speaker_center > bubble_center {
                1isize
            } else if speaker_center < bubble_center {
                -1isize
            } else {
                0isize
            };
            match bubble.style {
                BubbleStyle::Speech => {
                    for step in 1..=2 {
                        let x = (bubble_center as isize + direction * step).max(0) as usize;
                        output.push(place_bubble_mark(
                            canvas_width,
                            x,
                            if direction < 0 { '/' } else { '\\' },
                        ));
                    }
                }
                BubbleStyle::Thought => {
                    for (step, mark) in ['O', 'o', '.'].into_iter().enumerate() {
                        let x = (bubble_center as isize + direction * (step as isize + 1)).max(0)
                            as usize;
                        output.push(place_bubble_mark(canvas_width, x, mark));
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
            let avatar_width = avatar
                .iter()
                .map(|line| line.chars().count())
                .max()
                .unwrap_or(0);
            let height = lines.len().max(avatar.len());
            let bubble_top_padding = (height - lines.len()) / 2;
            let top_padding = (height - avatar.len()) / 2;
            let bubble_row = bubble_top_padding + lines.len() / 2;
            let connector = match (bubble.style, bubble.speaker_side) {
                (BubbleStyle::Speech, SpeakerSide::Left) => " <-- ",
                (BubbleStyle::Speech, SpeakerSide::Right) => " --> ",
                (BubbleStyle::Speech, SpeakerSide::Below) => unreachable!(),
                (BubbleStyle::Thought, SpeakerSide::Left) => " . o O ",
                (BubbleStyle::Thought, SpeakerSide::Right) => " O o . ",
                (BubbleStyle::Thought, SpeakerSide::Below) => unreachable!(),
            };
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
                        "     "
                    };
                    match bubble.speaker_side {
                        SpeakerSide::Left => format!(
                            "{avatar_line:<avatar_width$}{spacer}{bubble_line}",
                            avatar_width = avatar_width
                        ),
                        SpeakerSide::Right => format!(
                            "{bubble_line}{spacer}{avatar_line:<avatar_width$}",
                            avatar_width = avatar_width
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
    }
}

fn align_block(block: &str, alignment: ChartAlignment, wrap_width: usize) -> String {
    let width = block
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
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

fn format_horizontal_chart(chart: &BarChart) -> String {
    let label_width = chart
        .entries
        .iter()
        .map(|(label, _)| label.chars().count())
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
                "{label:<label_width$} | {} {value}",
                "#".repeat(bar_length),
                label_width = label_width
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_vertical_chart(chart: &BarChart) -> String {
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
        .map(|(label, _)| label.chars().count())
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
                    column[center] = '#';
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
            format!("{:>axis_width$} | ", max_value)
        } else {
            format!("{:>axis_width$} | ", "")
        };
        lines.push(format!("{axis}{}", bar_row(level)));
    }
    let baseline = "-".repeat(chart.entries.len() * (cell_width + 1) - 1);
    lines.push(format!("{:>axis_width$} +-{baseline}", "0"));
    let labels = chart
        .entries
        .iter()
        .map(|(label, _)| format!("{label:^cell_width$}"))
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
        .map(|(key, _)| key.chars().count())
        .max()
        .unwrap_or(0);
    let mut lines = Vec::new();
    for (key, value) in fields {
        let prefix = format!("{key:<key_width$}: ");
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
            Section::Table { head, rows } => format_table(&head, &rows),
            Section::BarChart(chart) => format_bar_chart(&chart, wrap_width),
            Section::FlowChart(chart) => format_flow_chart(&chart, wrap_width),
            Section::FlowDiagram(diagram) => format_flow_diagram(&diagram, wrap_width)?,
            Section::Bubble(bubble) => format_bubble(&bubble, wrap_width),
            Section::List(items) => format_list(&items, wrap_width),
        };
        parts.push(formatted);
    }
    for number in references.difference(&footers.keys().copied().collect()) {
        return Err(format!(
            "reference [#{number}] has no matching --compose-footer-{number}"
        ));
    }
    for number in footers.keys().filter(|number| !references.contains(number)) {
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

fn format_flow_diagram(diagram: &FlowDiagram, wrap_width: usize) -> Result<String, String> {
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

    let rendered = if diagram.direction == "LR" {
        render_horizontal_dot(&dot)?
    } else {
        let mut options = hascii::RenderOptions::default();
        options.border_mode = hascii::BorderMode::Plain;
        hascii::render(&dot, &options)
            .map_err(|error| format!("could not render --flow-chart: {error}"))?
    };
    let rendered = to_ascii_diagram(rendered.trim_end());
    let rendered = style_flow_nodes(&rendered, &diagram.nodes);
    Ok(align_block(&rendered, diagram.alignment, wrap_width))
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
    let (output_path, args) = extract_output_option(env::args_os().skip(1).collect())?;
    let (message, git_args) = compose(args)?;
    if let Some(output_path) = output_path {
        if !git_args.is_empty() {
            return Err("--output cannot be combined with Git commit options".into());
        }
        fs::write(&output_path, message).map_err(|error| {
            format!(
                "could not write {}: {error}",
                PathBuf::from(&output_path).display()
            )
        })?;
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

/// Assemble a commit message and return the arguments to forward to `git commit`.
pub fn compose(args: Vec<OsString>) -> Result<(String, Vec<OsString>), String> {
    let help_requested = args
        .iter()
        .take_while(|arg| arg.as_os_str() != OsStr::new("--"))
        .any(|arg| {
            matches!(
                arg.to_str(),
                Some("-h" | "--help" | "--help=short" | "--help=long" | "--version")
            )
        });
    let config = if help_requested {
        ComposeConfig::default()
    } else {
        load_user_config()?
    };
    let (subject, sections, footers, wrap_width, git_args) = parse_args(args, &config)?;
    let message = format_message(subject, sections, footers, wrap_width)?;
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
