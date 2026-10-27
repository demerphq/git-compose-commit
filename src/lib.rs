use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::sync::atomic::{AtomicU64, Ordering};

const WRAP_WIDTH: usize = 72;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
enum Section {
    Text(String),
    Detail(String),
    Line(String),
    Table {
        head: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    List(Vec<ListEntry>),
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

fn usage() -> &'static str {
    "Usage: git-compose-commit [wrapper options] [git commit options] [-- pathspec...]

Message options:
  --subject TEXT       Set the subject (also --compose-subject; once only)
  --text TEXT          Add a wrapped prose paragraph (repeatable)
  --detail TEXT        Add literal text, indented by four spaces
  --line TEXT          Add one unwrapped line
  --th CSV             Table header (also --table-head, --compose-th, --compose-table-head)
  --tr CSV             Add a table row (also --table-row, --compose-tr, --compose-table-row)
  --table-rows CSV     Add semicolon-separated rows
  --footer-N TEXT      Define footer N for a [#N] reference
  --numlist            Start a numbered list (also --list=numbered)
  --bullets            Start a bullet list (also --list=bullets)
  --list KIND          Start a numbered or bullets list
  --list-item TEXT     Add an item; defaults to a numbered list
  --list-end           End the current explicit list

Namespaced equivalents use the --compose- prefix. Unprefixed options may
conflict with future git commit options; use --compose-* in scripts.

Other arguments are passed to git commit. Use -- to stop parsing wrapper options."
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

fn parse_args(
    args: Vec<OsString>,
) -> Result<
    (
        Option<String>,
        Vec<Section>,
        BTreeMap<usize, String>,
        Vec<OsString>,
    ),
    String,
> {
    let mut subject = None;
    let mut sections = Vec::new();
    let mut footers = BTreeMap::new();
    let mut git_args = Vec::new();
    let mut active_table = None;
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
            git_args.push(arg.clone());
            passthrough = true;
            i += 1;
            continue;
        }
        if arg == OsStr::new("--help") || arg == OsStr::new("-h") {
            println!("{}", usage());
            std::process::exit(0);
        }
        if arg == OsStr::new("--version") {
            println!("git-compose-commit {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }

        let recognized = [
            "--compose-subject",
            "--compose-text",
            "--compose-detail",
            "--compose-line",
            "--compose-table-head",
            "--compose-table-row",
            "--compose-table-rows",
            "--compose-th",
            "--compose-tr",
            "--compose-list",
            "--compose-list-item",
            "--subject",
            "--text",
            "--detail",
            "--line",
            "--table-head",
            "--table-row",
            "--table-rows",
            "--th",
            "--tr",
            "--list",
            "--list-item",
        ]
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
            let canonical = match name {
                "--subject" => "--compose-subject",
                "--text" => "--compose-text",
                "--detail" => "--compose-detail",
                "--line" => "--compose-line",
                "--table-head" | "--th" | "--compose-th" => "--compose-table-head",
                "--table-row" | "--tr" | "--compose-tr" => "--compose-table-row",
                "--table-rows" => "--compose-table-rows",
                "--list" => "--compose-list",
                "--list-item" => "--compose-list-item",
                _ => name,
            };
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
            git_args.push(arg.clone());
            git_args.push(args[i + 1].clone());
            i += 2;
        } else {
            git_args.push(arg.clone());
            i += 1;
        }
    }
    Ok((subject, sections, footers, git_args))
}

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

fn format_list(items: &[ListEntry]) -> String {
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
            WRAP_WIDTH.saturating_sub(first_prefix.chars().count()),
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

fn format_message(
    subject: Option<String>,
    sections: Vec<Section>,
    footers: BTreeMap<usize, String>,
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
                wrap_paragraph(&text)
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
            Section::Table { head, rows } => format_table(&head, &rows),
            Section::List(items) => format_list(&items),
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
            let wrapped = wrap_paragraph_at(text, WRAP_WIDTH.saturating_sub(label_width + 1));
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
    let (message, git_args) = compose(env::args_os().skip(1).collect())?;
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
    let (subject, sections, footers, git_args) = parse_args(args)?;
    let message = format_message(subject, sections, footers)?;
    Ok((message, git_args))
}

/// Run `git commit` using the composed message and forwarded arguments.
pub fn run() -> ExitCode {
    match run_inner() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("git-compose-commit: {error}\n\n{}", usage());
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
