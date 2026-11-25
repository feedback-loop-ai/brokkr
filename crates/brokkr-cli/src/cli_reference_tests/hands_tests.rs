//! `docs/reference/hands.md`, held to `Cli::command()` (#1277, decision 0071
//! ruling 5).
//!
//! `brokkr hands` is hidden from `docs/reference/cli.md`, so its synopses and
//! option tables live in `docs/reference/hands.md`. This module reads the page
//! only when it can read it whole — refusing constructs it does not need to
//! follow — then derives the expected `Usage:` synopsis and option table for
//! every subcommand of `brokkr hands` directly from `Cli::command()`.

use std::fmt;

use clap::{Arg, Command};

use super::{built, default_values, description, listed, spelled, undescribed_in, workspace};

const HANDS_REFERENCE: &str = "docs/reference/hands.md";

#[derive(Debug, PartialEq, Eq)]
struct ParsedSection {
    verb: String,
    usage: String,
    table: String,
}

/// Page-wide constructs `docs/reference/hands.md` cannot hold (#1277 F1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageRefusal {
    BareCarriageReturn,
    HtmlComment,
    RawHtml,
    UnclosedCodeSpan,
    IndentedFence,
    UnclosedFence,
    NonCanonicalHeading,
    SetextOrThematicBreak,
}

impl fmt::Display for PageRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::BareCarriageReturn => "bare carriage return (`\r` not followed by `\n`)",
            Self::HtmlComment => "HTML comment opener (`<!--`) outside a fenced block",
            Self::RawHtml => "raw HTML tag outside a fenced block or code span",
            Self::UnclosedCodeSpan => "unclosed backtick code span outside a fenced block",
            Self::IndentedFence => "fence marker not at column 0",
            Self::UnclosedFence => "unclosed fenced block",
            Self::NonCanonicalHeading => "heading not written `#`..`# ` from column 0",
            Self::SetextOrThematicBreak => "setext underline or thematic break",
        })
    }
}

/// Section-header and table constructs refused between `## ` and table end (#1277 F1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeaderRefusal {
    ListItem,
    BlockQuote,
    IndentedLine,
    SecondFenceBeforeTable,
    TableBeforeSynopsis,
    TableRowWithoutLeadingPipe,
    TableNotFollowedByBlankLine,
}

impl fmt::Display for HeaderRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ListItem => "list item between `## ` and the end of the option table",
            Self::BlockQuote => "block quote between `## ` and the end of the option table",
            Self::IndentedLine => "line indented by four spaces or a tab before the table",
            Self::SecondFenceBeforeTable => "second fenced block before the option table",
            Self::TableBeforeSynopsis => "table row before the synopsis fenced block",
            Self::TableRowWithoutLeadingPipe => "table row not beginning with `|` at column 0",
            Self::TableNotFollowedByBlankLine => {
                "option table not followed by a blank line or end of page"
            }
        })
    }
}

/// Why `docs/reference/hands.md` cannot be read whole (#1277 F1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefusalKind {
    Page(PageRefusal),
    Header(HeaderRefusal),
}

impl From<PageRefusal> for RefusalKind {
    fn from(kind: PageRefusal) -> Self {
        Self::Page(kind)
    }
}

impl From<HeaderRefusal> for RefusalKind {
    fn from(kind: HeaderRefusal) -> Self {
        Self::Header(kind)
    }
}

impl fmt::Display for RefusalKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Page(kind) => kind.fmt(f),
            Self::Header(kind) => kind.fmt(f),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum HandsRefDrift {
    Refused {
        line: usize,
        kind: RefusalKind,
    },
    MissingUsage {
        verb: String,
    },
    MissingTable {
        verb: String,
    },
    SectionHeadings {
        expected: Vec<String>,
        actual: Vec<String>,
    },
    Synopsis {
        verb: String,
        expected: String,
        actual: String,
    },
    OptionTable {
        verb: String,
        expected: String,
        actual: String,
    },
}

impl fmt::Display for HandsRefDrift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused { line, kind } => write!(f, "line {line}: {kind}"),
            Self::MissingUsage { verb } => write!(f, "{verb}: missing usage fenced block"),
            Self::MissingTable { verb } => write!(f, "{verb}: missing option table"),
            Self::SectionHeadings { expected, actual } => {
                write!(
                    f,
                    "section headings differ: expected {expected:?}, found {actual:?}"
                )
            }
            Self::Synopsis {
                verb,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "{verb}: usage synopsis drifted:\n  expected: {expected}\n  actual:   {actual}"
                )
            }
            Self::OptionTable {
                verb,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "{verb}: option table drifted:\nexpected:\n{expected}actual:\n{actual}"
                )
            }
        }
    }
}

fn refused(line: usize, kind: impl Into<RefusalKind>) -> HandsRefDrift {
    HandsRefDrift::Refused {
        line,
        kind: kind.into(),
    }
}

fn hands_verbs(cli: &Command) -> Vec<&Command> {
    cli.find_subcommand("hands")
        .expect("brokkr hands subcommand")
        .get_subcommands()
        .filter(|verb| verb.get_name() != "help")
        .collect()
}

fn hands_argument_row(arg: &Arg) -> String {
    format!(
        "| `{}` | {} | {} |\n",
        spelled(arg),
        default_values(arg),
        description(arg)
    )
}

fn expected_section(verb: &Command) -> ParsedSection {
    let path = verb.get_bin_name().expect("a built command names its path");
    let usage = verb.clone().render_usage().to_string();
    let mut table = String::from("| Argument | Default | Description |\n| --- | --- | --- |\n");
    for arg in listed(verb) {
        table.push_str(&hands_argument_row(arg));
    }
    ParsedSection {
        verb: path.to_string(),
        usage: usage.trim_end().to_string(),
        table,
    }
}

fn is_gfm_blank(line: &str) -> bool {
    line.bytes().all(|b| matches!(b, b' ' | b'\t'))
}

fn strip_up_to_three_spaces(line: &str) -> &str {
    &line[line.bytes().take(3).take_while(|&b| b == b' ').count()..]
}

fn is_indented_four_or_tab(line: &str) -> bool {
    let lead = line.bytes().take_while(|&b| b == b' ').count();
    lead >= 4 || line.as_bytes().get(lead) == Some(&b'\t')
}

fn fence_opener(s: &str) -> Option<(u8, usize)> {
    let first = *s.as_bytes().first()?;
    if first != b'`' && first != b'~' {
        return None;
    }
    let count = s.bytes().take_while(|&b| b == first).count();
    (count >= 3 && (first != b'`' || !s[count..].contains('`'))).then_some((first, count))
}

fn is_closing_fence_rest(s: &str, marker: u8, width: usize) -> bool {
    let count = s.bytes().take_while(|&b| b == marker).count();
    count >= width && s[count..].trim_matches([' ', '\t']).is_empty()
}

fn strip_list_marker(rest: &str) -> Option<&str> {
    let bytes = rest.as_bytes();
    let marker = if matches!(bytes.first(), Some(b'-' | b'*' | b'+')) {
        1
    } else {
        let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
        if !(1..=9).contains(&digits) || !matches!(bytes.get(digits), Some(b'.' | b')')) {
            return None;
        }
        digits + 1
    };
    match bytes.get(marker) {
        None => Some(&rest[marker..]),
        Some(b' ' | b'\t') => Some(&rest[marker + 1..]),
        Some(_) => None,
    }
}

fn strip_leading_containers(mut line: &str) -> &str {
    loop {
        line = line.trim_start_matches([' ', '\t']);
        if let Some(rest) = line.strip_prefix('>') {
            line = rest;
        } else if let Some(rest) = strip_list_marker(line) {
            line = rest;
        } else {
            return line;
        }
    }
}

fn is_atx_heading(s: &str) -> bool {
    let hashes = s.bytes().take_while(|&b| b == b'#').count();
    (1..=6).contains(&hashes) && matches!(s.as_bytes().get(hashes), None | Some(b' ' | b'\t'))
}

fn is_canonical_heading(line: &str) -> bool {
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    (1..=6).contains(&hashes) && line.as_bytes().get(hashes) == Some(&b' ')
}

fn is_only_char(s: &str, ch: u8) -> bool {
    !s.is_empty() && s.bytes().all(|b| b == ch)
}

fn is_thematic_break(s: &str) -> bool {
    let Some(&m) = s
        .as_bytes()
        .first()
        .filter(|b| matches!(b, b'-' | b'*' | b'_'))
    else {
        return false;
    };
    s.bytes().all(|b| b == m || matches!(b, b' ' | b'\t'))
        && s.bytes().filter(|&b| b == m).count() >= 3
}

fn is_setext_or_thematic(line: &str, inner: &str) -> bool {
    let a = line.trim_matches([' ', '\t']);
    let b = inner.trim_matches([' ', '\t']);
    [a, b]
        .into_iter()
        .any(|s| is_only_char(s, b'=') || is_only_char(s, b'-') || is_thematic_break(s))
}

fn strip_quotes(mut line: &str) -> &str {
    loop {
        let rest = strip_up_to_three_spaces(line);
        let Some(after) = rest.strip_prefix('>') else {
            return rest;
        };
        line = after.strip_prefix(' ').unwrap_or(after);
    }
}

fn strip_heading_containers(mut line: &str, in_list: bool) -> &str {
    if in_list {
        return strip_leading_containers(line);
    }
    loop {
        let rest = strip_quotes(line);
        let Some(after) = strip_list_marker(rest) else {
            return rest;
        };
        line = after;
    }
}

fn line_opens_list(line: &str) -> bool {
    strip_list_marker(strip_quotes(line)).is_some()
}

fn is_raw_html_start(s: &str) -> bool {
    let Some(rest) = s.strip_prefix('<') else {
        return false;
    };
    let b = rest.as_bytes();
    if matches!(b.first(), Some(b'!' | b'?')) {
        return true;
    }
    let (closing, tail) = match b.strip_prefix(b"/") {
        Some(t) => (true, t),
        None => (false, b),
    };
    if !tail.first().is_some_and(u8::is_ascii_alphabetic) {
        return false;
    }
    let n = tail
        .iter()
        .take_while(|&&c| c.is_ascii_alphanumeric() || c == b'-')
        .count();
    matches!(tail.get(n), None | Some(b' ' | b'\t' | b'\x0c' | b'>'))
        || (!closing && tail.get(n) == Some(&b'/'))
}

fn find_closing_backticks(bytes: &[u8], mut j: usize, width: usize) -> Option<usize> {
    while j < bytes.len() {
        if bytes[j] == b'`' {
            let run = bytes[j..].iter().take_while(|&&b| b == b'`').count();
            if run == width {
                return Some(j + run);
            }
            j += run;
        } else {
            j += 1;
        }
    }
    None
}

fn check_inline_html_and_code_spans(line_no: usize, line: &str) -> Result<(), HandsRefDrift> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if bytes.get(i + 1).is_some_and(u8::is_ascii_punctuation) => i += 2,
            b'`' => {
                let width = bytes[i..].iter().take_while(|&&b| b == b'`').count();
                let Some(close_end) = find_closing_backticks(bytes, i + width, width) else {
                    return Err(refused(line_no, PageRefusal::UnclosedCodeSpan));
                };
                i = close_end;
            }
            b'<' if is_raw_html_start(&line[i..]) => {
                return Err(refused(line_no, PageRefusal::RawHtml));
            }
            _ => i += 1,
        }
    }
    Ok(())
}

fn parse_heading_verb(heading: &str) -> String {
    let trimmed = heading.trim_matches([' ', '\t']);
    let n = trimmed.bytes().take_while(|&b| b == b'`').count();
    let inner = (n > 0 && trimmed.len() >= 2 * n)
        .then(|| trimmed.strip_suffix(&trimmed[..n]).map(|s| &s[n..]))
        .flatten()
        .filter(|s| !s.contains('`'));
    inner.unwrap_or(trimmed).to_string()
}

fn is_delimiter_cell(cell: &str) -> bool {
    let trimmed = cell.trim_matches([' ', '\t']);
    let inner = trimmed.strip_prefix(':').unwrap_or(trimmed);
    is_only_char(inner.strip_suffix(':').unwrap_or(inner), b'-')
}

fn is_gfm_delimiter_row(line: &str) -> bool {
    if !line.contains('|') {
        return false;
    }
    let trimmed = line.trim_matches([' ', '\t']);
    let trimmed = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix('|').unwrap_or(trimmed);
    !trimmed.is_empty() && trimmed.split('|').all(is_delimiter_cell)
}

fn check_page_wide(line_no: usize, line: &str, in_list: bool) -> Result<(), HandsRefDrift> {
    if line.contains("<!--") {
        return Err(refused(line_no, PageRefusal::HtmlComment));
    }
    let inner = strip_leading_containers(line);
    if fence_opener(inner).is_some() {
        return Err(refused(line_no, PageRefusal::IndentedFence));
    }
    if is_atx_heading(strip_heading_containers(line, in_list)) && !is_canonical_heading(line) {
        return Err(refused(line_no, PageRefusal::NonCanonicalHeading));
    }
    if is_setext_or_thematic(line, inner) {
        return Err(refused(line_no, PageRefusal::SetextOrThematicBreak));
    }
    check_inline_html_and_code_spans(line_no, line)
}

fn check_section_header(line_no: usize, line: &str) -> Result<(), HandsRefDrift> {
    if is_indented_four_or_tab(line) {
        return Err(refused(line_no, HeaderRefusal::IndentedLine));
    }
    let rest = strip_up_to_three_spaces(line);
    if rest.starts_with('>') {
        return Err(refused(line_no, HeaderRefusal::BlockQuote));
    }
    if strip_list_marker(rest).is_some() {
        return Err(refused(line_no, HeaderRefusal::ListItem));
    }
    if rest.starts_with('|') || is_gfm_delimiter_row(line) {
        return Err(refused(line_no, HeaderRefusal::TableRowWithoutLeadingPipe));
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct OpenFence {
    marker: u8,
    width: usize,
    open_line: usize,
    captures_synopsis: bool,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum SectionPhase {
    #[default]
    Prose,
    BeforeSynopsis,
    BeforeTable,
    InTable,
}

struct SectionAcc {
    verb: String,
    usage: Option<Vec<String>>,
    table: Option<String>,
}

#[derive(Default)]
struct PageReader {
    in_fence: Option<OpenFence>,
    phase: SectionPhase,
    prose_list_open: bool,
    prev_blank: bool,
    sections: Vec<SectionAcc>,
}

impl PageReader {
    fn step_fence(&mut self, line_no: usize, line: &str) -> Result<bool, HandsRefDrift> {
        if let Some(fence) = self.in_fence {
            if is_closing_fence_rest(line, fence.marker, fence.width) {
                self.in_fence = None;
                self.prev_blank = false;
                if fence.captures_synopsis {
                    self.phase = SectionPhase::BeforeTable;
                }
            } else if is_closing_fence_rest(
                strip_up_to_three_spaces(line),
                fence.marker,
                fence.width,
            ) {
                return Err(refused(line_no, PageRefusal::IndentedFence));
            } else if fence.captures_synopsis {
                let usage = self
                    .sections
                    .last_mut()
                    .and_then(|s| s.usage.as_mut())
                    .unwrap();
                usage.push(line.to_string());
            }
            return Ok(true);
        }

        let Some((marker, width)) = fence_opener(line) else {
            return Ok(false);
        };
        let captures_synopsis = match self.phase {
            SectionPhase::InTable => {
                return Err(refused(line_no, HeaderRefusal::TableNotFollowedByBlankLine));
            }
            SectionPhase::BeforeTable => {
                return Err(refused(line_no, HeaderRefusal::SecondFenceBeforeTable));
            }
            SectionPhase::BeforeSynopsis => {
                self.sections.last_mut().unwrap().usage = Some(Vec::new());
                true
            }
            SectionPhase::Prose => {
                self.prose_list_open = false;
                false
            }
        };
        self.prev_blank = false;
        self.in_fence = Some(OpenFence {
            marker,
            width,
            open_line: line_no,
            captures_synopsis,
        });
        Ok(true)
    }

    fn step_in_table(&mut self, line_no: usize, line: &str) -> Result<(), HandsRefDrift> {
        if line.starts_with('|') {
            let table = self
                .sections
                .last_mut()
                .and_then(|s| s.table.as_mut())
                .unwrap();
            table.push_str(line);
            table.push('\n');
            self.prev_blank = false;
        } else if is_gfm_blank(line) {
            self.phase = SectionPhase::Prose;
            self.prose_list_open = false;
            self.prev_blank = true;
        } else {
            return Err(refused(line_no, HeaderRefusal::TableNotFollowedByBlankLine));
        }
        Ok(())
    }

    fn step_section_header(&mut self, line_no: usize, line: &str) -> Result<(), HandsRefDrift> {
        if is_gfm_blank(line) {
            self.prev_blank = true;
            return Ok(());
        }
        self.prev_blank = false;
        if line.starts_with('|') {
            if self.phase == SectionPhase::BeforeSynopsis {
                return Err(refused(line_no, HeaderRefusal::TableBeforeSynopsis));
            }
            self.sections.last_mut().unwrap().table = Some(format!("{line}\n"));
            self.phase = SectionPhase::InTable;
            return Ok(());
        }
        check_section_header(line_no, line)
    }

    fn step_prose_list(&mut self, line: &str) {
        if is_gfm_blank(line) {
            self.prev_blank = true;
            return;
        }
        if line_opens_list(line) {
            self.prose_list_open = true;
        } else if is_canonical_heading(line) || (self.prev_blank && !line.starts_with([' ', '\t']))
        {
            self.prose_list_open = false;
        }
        self.prev_blank = false;
    }

    fn step_line(&mut self, line_no: usize, line: &str) -> Result<(), HandsRefDrift> {
        if self.step_fence(line_no, line)? {
            return Ok(());
        }
        let in_list = self.phase == SectionPhase::Prose && self.prose_list_open;
        check_page_wide(line_no, line, in_list)?;
        if self.phase == SectionPhase::InTable {
            return self.step_in_table(line_no, line);
        }
        if let Some(heading) = line.strip_prefix("## ") {
            self.sections.push(SectionAcc {
                verb: parse_heading_verb(heading),
                usage: None,
                table: None,
            });
            self.phase = SectionPhase::BeforeSynopsis;
            self.prose_list_open = false;
            self.prev_blank = false;
            return Ok(());
        }
        if matches!(
            self.phase,
            SectionPhase::BeforeSynopsis | SectionPhase::BeforeTable
        ) {
            return self.step_section_header(line_no, line);
        }
        self.step_prose_list(line);
        Ok(())
    }

    fn finish(self) -> Result<Vec<ParsedSection>, HandsRefDrift> {
        if let Some(fence) = self.in_fence {
            return Err(refused(fence.open_line, PageRefusal::UnclosedFence));
        }
        self.sections
            .into_iter()
            .map(|s| {
                let usage = s.usage.ok_or_else(|| HandsRefDrift::MissingUsage {
                    verb: s.verb.clone(),
                })?;
                let table = s.table.ok_or_else(|| HandsRefDrift::MissingTable {
                    verb: s.verb.clone(),
                })?;
                Ok(ParsedSection {
                    verb: s.verb,
                    usage: usage.join("\n"),
                    table,
                })
            })
            .collect()
    }
}

fn parse_sections(page: &str) -> Result<Vec<ParsedSection>, HandsRefDrift> {
    let mut reader = PageReader {
        prev_blank: true,
        ..PageReader::default()
    };
    for (idx, raw) in page.split_inclusive('\n').enumerate() {
        let line = raw
            .strip_suffix("\r\n")
            .or_else(|| raw.strip_suffix('\n'))
            .unwrap_or(raw);

        if line.contains('\r') {
            return Err(refused(idx + 1, PageRefusal::BareCarriageReturn));
        }
        reader.step_line(idx + 1, line)?;
    }
    reader.finish()
}

fn check_hands_reference(cli: &Command, page: &str) -> Vec<HandsRefDrift> {
    let expected: Vec<ParsedSection> = hands_verbs(cli).into_iter().map(expected_section).collect();
    let actual = match parse_sections(page) {
        Ok(sections) => sections,
        Err(err) => return vec![err],
    };

    let expected_verbs: Vec<String> = expected.iter().map(|s| s.verb.clone()).collect();
    let actual_verbs: Vec<String> = actual.iter().map(|s| s.verb.clone()).collect();
    if actual_verbs != expected_verbs {
        return vec![HandsRefDrift::SectionHeadings {
            expected: expected_verbs,
            actual: actual_verbs,
        }];
    }

    let mut drifts = Vec::new();
    for (exp, act) in expected.iter().zip(actual.iter()) {
        if act.usage != exp.usage {
            drifts.push(HandsRefDrift::Synopsis {
                verb: exp.verb.clone(),
                expected: exp.usage.clone(),
                actual: act.usage.clone(),
            });
        }
        if act.table != exp.table {
            drifts.push(HandsRefDrift::OptionTable {
                verb: exp.verb.clone(),
                expected: exp.table.clone(),
                actual: act.table.clone(),
            });
        }
    }
    drifts
}

fn serve_reference_fixture() -> (Command, String, ParsedSection) {
    let cli = built();
    let committed = std::fs::read_to_string(workspace().join(HANDS_REFERENCE))
        .expect("docs/reference/hands.md reads");
    let serve = hands_verbs(&cli)
        .into_iter()
        .find(|verb| verb.get_name() == "serve")
        .expect("hands serve subcommand");
    let serve_expected = expected_section(serve);
    (cli, committed, serve_expected)
}

fn line_of(page: &str, needle: &str) -> usize {
    let offset = page
        .find(needle)
        .unwrap_or_else(|| panic!("expected needle in page: {needle:?}"));
    page[..offset].bytes().filter(|&b| b == b'\n').count() + 1
}

fn insert_before(page: &str, anchor: &str, block: &str) -> (String, usize) {
    let line_no = line_of(page, anchor);
    let mutated = page.replacen(anchor, &format!("{block}\n{anchor}"), 1);
    assert_ne!(mutated, page, "anchor not found: {anchor:?}");
    (mutated, line_no)
}

type RefusalCase = (&'static str, String, usize, RefusalKind);

fn push_inserted<K: Into<RefusalKind> + Copy>(
    out: &mut Vec<RefusalCase>,
    page: &str,
    anchor: &str,
    rows: &[(&'static str, &str, K)],
) {
    for &(label, block, kind) in rows {
        let (mutated, line_no) = insert_before(page, anchor, block);
        out.push((label, mutated, line_no, kind.into()));
    }
}

fn replace_case(
    label: &'static str,
    page: &str,
    edit: (&str, &str, &str),
    kind: impl Into<RefusalKind>,
) -> RefusalCase {
    let (from, to, needle) = edit;
    let mutated = page.replacen(from, to, 1);
    assert_ne!(mutated, page, "replacement target not found for {label}");
    let line_no = line_of(&mutated, needle);
    (label, mutated, line_no, kind.into())
}

fn inserted_refusal_cases(committed: &str, serve: &ParsedSection) -> Vec<RefusalCase> {
    use HeaderRefusal as H;
    use PageRefusal as P;

    const PROSE: &[(&str, &str, PageRefusal)] = &[
        ("bare cr", "Prose.\r## `hands x`", P::BareCarriageReturn),
        ("comment in list", "- <!-- x -->", P::HtmlComment),
        ("raw html block", "<table><tr></tr></table>", P::RawHtml),
        ("inline html heading", "Note. <h2>extra</h2>", P::RawHtml),
        ("unclosed code span", "Prose `unclosed", P::UnclosedCodeSpan),
        ("fence in list", "- ```text", P::IndentedFence),
        ("fence in quote", "> ```text", P::IndentedFence),
        ("indented fence", "    ```text", P::IndentedFence),
        ("spaced heading", " ## `hands x`", P::NonCanonicalHeading),
        ("tab heading", "##\t`hands x`", P::NonCanonicalHeading),
        ("empty heading", "##", P::NonCanonicalHeading),
        ("list heading", "- ## `hands x`", P::NonCanonicalHeading),
        ("setext equals", "===", P::SetextOrThematicBreak),
        ("setext hyphens", "---", P::SetextOrThematicBreak),
        ("thematic break", "- - -", P::SetextOrThematicBreak),
    ];
    const HEADER: &[(&str, &str, HeaderRefusal)] = &[
        ("dash list", "- item", H::ListItem),
        ("star list", "  * item", H::ListItem),
        ("plus list", "+ item", H::ListItem),
        ("dot list", "1. item", H::ListItem),
        ("paren list", "2) item", H::ListItem),
        ("block quote", "> note", H::BlockQuote),
        ("four-space indent", "    code", H::IndentedLine),
        ("tab indent", "\tcode", H::IndentedLine),
        ("indented nbsp in header", "    \u{00A0}", H::IndentedLine),
        (
            "second fence",
            "```text\nsecond\n```\n",
            H::SecondFenceBeforeTable,
        ),
    ];

    let usage_open = format!("```text\n{}", serve.usage);
    let table_head = "| Argument | Default | Description |";
    let prose = "### Spelling, mutual exclusion, and defaults";
    let mut cases = Vec::new();
    push_inserted(&mut cases, committed, prose, PROSE);
    push_inserted(
        &mut cases,
        committed,
        table_head,
        &[("comment in header", "<!-- x -->", P::HtmlComment)],
    );
    push_inserted(&mut cases, committed, table_head, HEADER);
    push_inserted(
        &mut cases,
        committed,
        &usage_open,
        &[("table before synopsis", "| a | b |", H::TableBeforeSynopsis)],
    );
    cases
}

fn rewritten_page_refusal_cases(committed: &str, serve: &ParsedSection) -> Vec<RefusalCase> {
    use PageRefusal as P;

    let usage = &serve.usage;
    let usage_open = format!("```text\n{usage}");
    let usage_block = format!("```text\n{usage}\n```\n");
    let ind_open = format!(" {usage_open}");
    let ind_close = format!("```text\n{usage}\n ```\n");
    let unclosed = format!("{committed}\n```text\nunclosed\n");
    let unclosed_line = line_of(&unclosed, "```text\nunclosed");
    let serve_h2 = format!("## `{}`", serve.verb);
    let unmatched_h2 = format!("## `{}", serve.verb);
    let nested_h2 = "    ## `hands x`\n  - spec:";
    let case =
        |l, from, to, needle, k: PageRefusal| replace_case(l, committed, (from, to, needle), k);

    vec![
        case(
            "open fence col>0",
            &usage_open,
            &ind_open,
            &ind_open,
            P::IndentedFence,
        ),
        case(
            "close fence col>0",
            &usage_block,
            &ind_close,
            " ```\n",
            P::IndentedFence,
        ),
        (
            "unclosed fence",
            unclosed,
            unclosed_line,
            P::UnclosedFence.into(),
        ),
        case(
            "unmatched heading backtick",
            &serve_h2,
            &unmatched_h2,
            &unmatched_h2,
            P::UnclosedCodeSpan,
        ),
        case(
            "nested list heading",
            "  - spec:",
            nested_h2,
            "    ## `hands x`",
            P::NonCanonicalHeading,
        ),
    ]
}

fn rewritten_table_refusal_cases(committed: &str, serve: &ParsedSection) -> Vec<RefusalCase> {
    use HeaderRefusal as H;

    let table = &serve.table;
    let head = "| Argument | Default | Description |";
    let pipeless: String = table
        .lines()
        .map(|l| {
            format!(
                "{}\n",
                l.trim_start_matches('|').trim_end_matches('|').trim()
            )
        })
        .collect();
    let ind_head = format!(" {head}");
    let cont = "`--extra <EXTRA>` | `\"wrong\"` | Extra option";
    let prose_cont = "Trailing note without blank line.";
    let with_cont = format!("{table}{cont}\n");
    let with_prose = format!("{table}{prose_cont}\n");
    let with_nbsp = format!("{table}\u{00A0}\n| `--extra` |  | Extra |\n");
    let case =
        |l, from, to, needle, k: HeaderRefusal| replace_case(l, committed, (from, to, needle), k);

    vec![
        case(
            "pipe-less table",
            table,
            &pipeless,
            "--- | --- | ---",
            H::TableRowWithoutLeadingPipe,
        ),
        case(
            "indented table",
            head,
            &ind_head,
            &ind_head,
            H::TableRowWithoutLeadingPipe,
        ),
        case(
            "pipe-less continuation",
            table,
            &with_cont,
            cont,
            H::TableNotFollowedByBlankLine,
        ),
        case(
            "prose after table",
            table,
            &with_prose,
            prose_cont,
            H::TableNotFollowedByBlankLine,
        ),
        case(
            "nbsp after table",
            table,
            &with_nbsp,
            "\u{00A0}",
            H::TableNotFollowedByBlankLine,
        ),
    ]
}

#[test]
fn the_hands_reference_matches_clap() {
    let cli = built();
    let committed = std::fs::read_to_string(workspace().join(HANDS_REFERENCE))
        .expect("docs/reference/hands.md reads");
    let drifts = check_hands_reference(&cli, &committed);
    assert!(
        drifts.is_empty(),
        "{HANDS_REFERENCE} differs from Cli::command():\n{}",
        drifts
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
    let undescribed = undescribed_in(hands_verbs(&cli));
    assert!(
        undescribed.is_empty(),
        "arguments {HANDS_REFERENCE} would list with no description:\n{}",
        undescribed.join("\n")
    );
}

#[test]
fn the_hands_reference_check_catches_synopsis_table_and_section_drift() {
    let (cli, committed, serve_expected) = serve_reference_fixture();
    let verbs = hands_verbs(&cli);
    let expected: Vec<ParsedSection> = verbs.iter().map(|verb| expected_section(verb)).collect();
    let serve = verbs
        .iter()
        .copied()
        .find(|v| v.get_name() == "serve")
        .unwrap();

    // 1. Changed synopsis.
    let mutated_synopsis = format!("{} --drifted <DRIFTED>", serve_expected.usage);
    let drifted_synopsis = committed.replacen(&serve_expected.usage, &mutated_synopsis, 1);
    assert_eq!(
        check_hands_reference(&cli, &drifted_synopsis),
        [HandsRefDrift::Synopsis {
            verb: serve_expected.verb.clone(),
            expected: serve_expected.usage.clone(),
            actual: mutated_synopsis,
        }]
    );

    // 2. Changed default cell.
    let spec_arg = listed(serve)
        .into_iter()
        .find(|a| a.get_id() == "spec")
        .unwrap();
    let spec_cell = format!("| `{}` | {} |", spelled(spec_arg), default_values(spec_arg));
    let drifted_table = committed.replace(&spec_cell, &format!("| `{}` |  |", spelled(spec_arg)));
    let expected_drifted: Vec<String> = expected
        .iter()
        .filter(|s| s.table.contains(&spec_cell))
        .map(|s| s.verb.clone())
        .collect();
    let actual_drifted: Vec<String> = check_hands_reference(&cli, &drifted_table)
        .into_iter()
        .map(|d| match d {
            HandsRefDrift::OptionTable { verb, .. } => verb,
            other => panic!("expected OptionTable drift, got {other:?}"),
        })
        .collect();
    assert_eq!(actual_drifted, expected_drifted);

    // 3. Extra option row and 4. Missing option row.
    let extra = format!(
        "{}| `--extra <EXTRA>` |  | Extra option |\n",
        serve_expected.table
    );
    let missing = serve_expected
        .table
        .replacen(&hands_argument_row(spec_arg), "", 1);
    for mutated_table in [extra, missing] {
        let page = committed.replacen(&serve_expected.table, &mutated_table, 1);
        assert_eq!(
            check_hands_reference(&cli, &page),
            [HandsRefDrift::OptionTable {
                verb: serve_expected.verb.clone(),
                expected: serve_expected.table.clone(),
                actual: mutated_table,
            }]
        );
    }

    // 5. Trailing NBSP on section heading is not stripped.
    let expected_verbs: Vec<String> = expected.iter().map(|s| s.verb.clone()).collect();
    let serve_h2 = format!("## `{}`", serve_expected.verb);
    let nbsp_page = committed.replacen(&serve_h2, &format!("{serve_h2}\u{00A0}"), 1);
    let mut actual_nbsp = expected_verbs.clone();
    actual_nbsp[0] = format!("`{}`\u{00A0}", serve_expected.verb);
    assert_eq!(
        check_hands_reference(&cli, &nbsp_page),
        [HandsRefDrift::SectionHeadings {
            expected: expected_verbs.clone(),
            actual: actual_nbsp,
        }]
    );

    // 6. Missing section.
    let last_verb = expected_verbs.last().expect("at least two hands verbs");
    let missing_last = committed
        .split_once(&format!("## `{last_verb}`"))
        .unwrap()
        .0;
    assert_eq!(
        check_hands_reference(&cli, missing_last),
        [HandsRefDrift::SectionHeadings {
            expected: expected_verbs.clone(),
            actual: expected_verbs[..expected_verbs.len() - 1].to_vec(),
        }]
    );
}

#[test]
fn the_hands_reference_check_accepts_allowed_page_shapes() {
    let (cli, committed, serve_expected) = serve_reference_fixture();
    let usage_block = format!("```text\n{}\n```\n", serve_expected.usage);

    let longer = format!("````text\n{}\n````\n", serve_expected.usage);
    assert_eq!(
        check_hands_reference(&cli, &committed.replacen(&usage_block, &longer, 1)),
        []
    );

    let prose = "### Spelling, mutual exclusion, and defaults";
    let fenced_prose = format!("{prose}\n\n```markdown\n## example\n```");
    assert_eq!(
        check_hands_reference(&cli, &committed.replacen(prose, &fenced_prose, 1)),
        []
    );

    let indented_code = format!("    ## indented code example\n\n{prose}");
    assert_eq!(
        check_hands_reference(&cli, &committed.replacen(prose, &indented_code, 1)),
        []
    );

    let last_verb = hands_verbs(&cli)
        .into_iter()
        .last()
        .expect("last hands verb");
    let last_table = expected_section(last_verb).table;
    let end = committed.find(&last_table).expect("last table") + last_table.len();
    assert_eq!(check_hands_reference(&cli, &committed[..end]), []);
}

#[test]
fn the_hands_reference_check_refuses_unsupported_constructs() {
    let (cli, committed, serve_expected) = serve_reference_fixture();
    let cases = inserted_refusal_cases(&committed, &serve_expected)
        .into_iter()
        .chain(rewritten_page_refusal_cases(&committed, &serve_expected))
        .chain(rewritten_table_refusal_cases(&committed, &serve_expected));
    for (label, page, expected_line, expected_kind) in cases {
        let drifts = check_hands_reference(&cli, &page);
        assert_eq!(
            drifts,
            [HandsRefDrift::Refused {
                line: expected_line,
                kind: expected_kind,
            }],
            "{label}"
        );
        assert!(
            drifts[0]
                .to_string()
                .starts_with(&format!("line {expected_line}: ")),
            "{label}: display should name the line: {}",
            drifts[0]
        );
    }
}
