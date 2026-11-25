//! The text guard (`design/ENGINE_TEXT.md` › The guard test): no new
//! sentence for players or authors may be written in the source. Every
//! one comes from a text table in `src/builtins/text/`.
//!
//! A heuristic scan: string literals in the files below, outside
//! `#[cfg(test)]` code, that read as text, two or more words of letters
//! once `{placeholders}` are set aside. File templates (a literal starting
//! `---` and a newline, the frontmatter of a card the console writes) and
//! a literal on the line after a `// text-guard: not text` comment are
//! not text, nor is a command line (`/set holds in`, a recipe's step). Author notes, validation messages, errors and command-line
//! output stay in code for now (`PLAN.md` › Phase 11), so a literal passed
//! to one of their call sites is exempt. Anything else goes in
//! `tests/text_allowlist.txt` with why, and a line there that matches no
//! literal any more fails too, so the list doesn't rot.

use std::path::{Path, PathBuf};

/// The folders and files scanned. `src/console/` joins once its last
/// sentences have moved to `[console]`.
const SCANNED: &[&str] = &["src/engines", "src/render.rs", "src/console"];

/// Call sites whose messages are tooling, not story: author notes,
/// validation and load messages, errors, and assertions. A literal in
/// the same statement as one of these is exempt.
const TOOLING: &[&str] = &[
    "with_note(",
    "notes.push(",
    "note: Some(",
    "warnings.push(",
    "problems.push(",
    "errors.push(",
    "warning = Some(",
    "bail!(",
    "anyhow!(",
    "Err(",
    "context(",
    "with_context(",
    "map_err(",
    "ok_or_else(",
    "ok_or(",
    "expect(",
    "panic!(",
    "unreachable!(",
    "unimplemented!(",
    "todo!(",
    "eprintln!(",
    "assert",
];

/// A string literal: its text (escapes read), file and line.
#[derive(Debug)]
struct Literal {
    text: String,
    file: PathBuf,
    line: usize,
    /// The code before it, back to the start of its statement.
    before: String,
    /// Marked `// text-guard: not text` on the line before.
    marked: bool,
}

/// The comment that marks the next literal as not text.
const MARKER: &str = "text-guard: not text";

/// Every string literal in `source` outside `#[cfg(test)]` items and
/// comments.
fn literals(file: &Path, source: &str) -> Vec<Literal> {
    let chars: Vec<char> = source.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;
    // Code since the last statement boundary, for the call-site check.
    let mut statement = String::new();
    // Inside a `#[cfg(test)]` item: the brace depth, once its `{` is seen.
    let mut skipping: Option<Option<usize>> = None;
    // The line a marker comment covers.
    let mut marked_line: Option<usize> = None;
    let starts = |i: usize, s: &str| s.chars().enumerate().all(|(k, c)| chars.get(i + k) == Some(&c));
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            line += 1;
        }
        // Comments.
        if starts(i, "//") {
            let mut comment = String::new();
            while i < chars.len() && chars[i] != '\n' {
                comment.push(chars[i]);
                i += 1;
            }
            if comment.contains(MARKER) {
                marked_line = Some(line + 1);
            }
            continue;
        }
        if starts(i, "/*") {
            let mut depth = 0;
            while i < chars.len() {
                if starts(i, "/*") {
                    depth += 1;
                    i += 2;
                } else if starts(i, "*/") {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    if chars[i] == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
            continue;
        }
        if skipping.is_none() && starts(i, "#[cfg(test)]") {
            skipping = Some(None);
            i += "#[cfg(test)]".len();
            continue;
        }
        // Character literals and lifetimes.
        if c == '\'' {
            if chars.get(i + 1) == Some(&'\\') {
                i += 2;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                i += 1;
                continue;
            }
            if chars.get(i + 2) == Some(&'\'') {
                i += 3;
                continue;
            }
            i += 1;
            continue;
        }
        // String literals: "...", r"...", r#"..."#, b"...".
        let raw_hashes = if c == 'r' && (chars.get(i + 1) == Some(&'"') || chars.get(i + 1) == Some(&'#')) && (i == 0 || !is_ident(chars[i - 1])) {
            let mut n = 0;
            while chars.get(i + 1 + n) == Some(&'#') {
                n += 1;
            }
            (chars.get(i + 1 + n) == Some(&'"')).then_some(n)
        } else {
            None
        };
        if c == '"' || raw_hashes.is_some() {
            let start_line = line;
            let mut text = String::new();
            match raw_hashes {
                Some(n) => {
                    i += 2 + n;
                    let close: String = std::iter::once('"').chain(std::iter::repeat('#').take(n)).collect();
                    while i < chars.len() && !starts(i, &close) {
                        if chars[i] == '\n' {
                            line += 1;
                        }
                        text.push(chars[i]);
                        i += 1;
                    }
                    i += close.chars().count();
                }
                None => {
                    i += 1;
                    while i < chars.len() && chars[i] != '"' {
                        if chars[i] == '\\' {
                            match chars.get(i + 1) {
                                Some('n') => text.push('\n'),
                                Some('t') => text.push('\t'),
                                Some('\n') => {
                                    // A line continuation skips the
                                    // newline and the next line's indent.
                                    line += 1;
                                    i += 2;
                                    while i < chars.len() && chars[i].is_whitespace() {
                                        i += 1;
                                    }
                                    continue;
                                }
                                Some(other) => text.push(*other),
                                None => {}
                            }
                            i += 2;
                            continue;
                        }
                        if chars[i] == '\n' {
                            line += 1;
                        }
                        text.push(chars[i]);
                        i += 1;
                    }
                    i += 1;
                }
            }
            if skipping.is_none() {
                let marked = marked_line == Some(start_line);
                out.push(Literal { text, file: file.to_path_buf(), line: start_line, before: statement.clone(), marked });
            }
            statement.push_str("\"\"");
            continue;
        }
        match c {
            '{' => {
                if let Some(depth) = skipping.as_mut() {
                    *depth = Some(depth.unwrap_or(0) + 1);
                }
                statement.clear();
            }
            '}' => {
                if let Some(Some(depth)) = skipping.as_mut() {
                    *depth -= 1;
                    if *depth == 0 {
                        skipping = None;
                    }
                }
                statement.clear();
            }
            // A test item with no body (`#[cfg(test)] use ...;`) ends here.
            ';' => {
                if skipping == Some(None) {
                    skipping = None;
                }
                statement.clear();
            }
            _ => statement.push(c),
        }
        i += 1;
    }
    out
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether a literal reads as text: two or more words of letters, with
/// `{placeholders}` set aside. A file template isn't text.
fn reads_as_text(text: &str) -> bool {
    if text.starts_with("---\n") {
        return false;
    }
    // A command line the person types, like a recipe's steps.
    // Prose that opens with a command ("/play here needs a room.") ends
    // like a sentence; a command line doesn't.
    if text.starts_with('/') && text[1..].starts_with(|c: char| c.is_ascii_lowercase()) && !text.trim_end().ends_with(['.', '?', '!', ':']) {
        return false;
    }
    let mut plain = String::new();
    let mut depth = 0;
    for c in text.chars() {
        match c {
            '{' => depth += 1,
            '}' if depth > 0 => depth -= 1,
            _ if depth == 0 => plain.push(c),
            _ => plain.push(' '),
        }
    }
    // A word is letters (an apostrophe or hyphen inside is fine) with any
    // punctuation around it; `no_exit`, `door/open.md` and `#not-here`
    // are ids and paths, not words.
    let word = |w: &str| {
        let w = w.trim_matches(|c: char| ",.;:!?()[]`\"'«»…".contains(c));
        w.chars().filter(|c| c.is_alphabetic()).count() >= 2 && w.chars().all(|c| c.is_alphabetic() || c == '\'' || c == '-')
    };
    plain.split_whitespace().filter(|w| word(w)).count() >= 2
}

fn rust_files(root: &Path, rel: &str) -> Vec<PathBuf> {
    let path = root.join(rel);
    if path.is_file() {
        return vec![path];
    }
    let mut files: Vec<PathBuf> = walk(&path).into_iter().filter(|p| p.extension().map_or(false, |e| e == "rs")).collect();
    files.sort();
    files
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}

/// One allowlist line: the literal (before the tab), the file its reason
/// names first (`src/render.rs: ...`), and its line number.
struct Allowed {
    literal: String,
    file: String,
    line: usize,
}

fn allowlist(root: &Path) -> Vec<Allowed> {
    let text = std::fs::read_to_string(root.join("tests/text_allowlist.txt")).unwrap_or_default();
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|(n, l)| {
            let (literal, why) = l.split_once('\t').unwrap_or((l, ""));
            let file = why.split(':').next().unwrap_or("").trim().to_string();
            Allowed { literal: literal.to_string(), file, line: n + 1 }
        })
        .collect()
}

#[test]
fn no_sentence_is_written_in_the_source() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let allowed = allowlist(root);
    let mut flagged = Vec::new();
    let mut used = vec![false; allowed.len()];
    for rel in SCANNED {
        for file in rust_files(root, rel) {
            let source = std::fs::read_to_string(&file).unwrap();
            let shown = file.strip_prefix(root).unwrap_or(&file).to_path_buf();
            for lit in literals(&shown, &source) {
                if lit.marked || !reads_as_text(&lit.text) {
                    continue;
                }
                if let Some(k) = allowed.iter().position(|a| a.literal == lit.text) {
                    used[k] = true;
                    continue;
                }
                if TOOLING.iter().any(|t| lit.before.contains(t)) {
                    continue;
                }
                flagged.push(format!("{}:{}: {:?}", lit.file.display(), lit.line, lit.text));
            }
        }
    }
    // A line for a file the scan doesn't cover yet isn't checked.
    let scanned = |file: &str| SCANNED.iter().any(|s| file.starts_with(s));
    let stale: Vec<String> = allowed
        .iter()
        .zip(&used)
        .filter(|(a, u)| !**u && scanned(&a.file))
        .map(|(a, _)| format!("tests/text_allowlist.txt:{}: {:?} matches no literal in {}", a.line, a.literal, a.file))
        .collect();
    assert!(
        flagged.is_empty() && stale.is_empty(),
        "Sentences belong in a text table (src/builtins/text/), or in tests/text_allowlist.txt with why:\n{}\n{}",
        flagged.join("\n"),
        stale.join("\n")
    );
}

#[test]
fn the_scan_reads_literals_and_skips_tests_and_comments() {
    let source = "fn a() { say(\"You see it.\"); let c = '\"'; } // \"Not this.\"\n/* \"Nor this.\" */\nfn b<'a>() { x(r#\"A raw \"one\".\"#); bail!(\"Tooling here.\") }\n#[cfg(test)]\nmod tests { fn t() { s(\"Skipped too.\"); } }\nfn c() { s(\"After {n} tests.\"); }\n";
    let found: Vec<(String, usize, bool)> = literals(Path::new("x.rs"), source)
        .into_iter()
        .map(|l| (l.text.clone(), l.line, TOOLING.iter().any(|t| l.before.contains(t))))
        .collect();
    assert_eq!(
        found,
        [
            ("You see it.".to_string(), 1, false),
            ("A raw \"one\".".to_string(), 3, false),
            ("Tooling here.".to_string(), 3, true),
            ("After {n} tests.".to_string(), 6, false),
        ]
    );
    assert!(reads_as_text("You see {list}."));
    assert!(reads_as_text("{} can't hold itself"));
    assert!(reads_as_text("no game yet"));
    assert!(!reads_as_text("{}: {}"));
    assert!(!reads_as_text("door"));
    assert!(!reads_as_text("  {}  {}"));
    assert!(!reads_as_text("no_exit"));
    assert!(!reads_as_text("/set holds in"));
    assert!(reads_as_text("/play here needs a room."), "a command line is code; this one's prose isn't");
    assert!(!reads_as_text("../builtins/door/actions.rhai"));
    assert!(!reads_as_text("---\nid: {}\ntitle: {}\n---\n"));
    let marked = literals(Path::new("x.rs"), "fn a() {\n    // text-guard: not text\n    k(\"two words\");\n    k(\"two more\");\n}\n");
    assert_eq!(marked.iter().map(|l| l.marked).collect::<Vec<_>>(), [true, false], "the marker covers the next line only");
}
