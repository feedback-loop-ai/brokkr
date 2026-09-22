//! A closed argument grammar for the harnesses brokkr knows how to launch
//! (decision 0066 ruling 6, answering the second council's H1, H2 and H3).
//!
//! The first repair scanned argv for the spellings it knew: `-c VALUE` and
//! `-c=VALUE` but not `-cVALUE`, `--mcp-config` but not `--plugin-dir`, the
//! first value of a tool list but not the second. A scanner that lists the
//! spellings it recognizes is fail-open by construction — every spelling
//! nobody listed passes through untouched — so the rule is inverted here.
//!
//! An argv for a harness brokkr KNOWS is parsed against a model of that
//! CLI's option grammar: which options exist, which take a value, in which
//! forms (split, equals-joined, attached), which are variadic, which may
//! repeat, and what each one DOES. A token the grammar cannot place is a
//! refusal naming that token, never a pass-through. Admission and
//! composition then judge the parsed STRUCTURE, so every spelling of one
//! option is judged at once because there is only one structure.
//!
//! The guarantee is a closed supported subset of each CLI, not support for
//! every option those CLIs will ever grow: a new option nobody modelled
//! refuses until it is modelled, which is the direction a capability fence
//! must fail in.

use std::fmt;

/// How many argv tokens one option's value occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arity {
    /// A switch: no value at all.
    Bare,
    /// Exactly one value.
    One,
    /// One or more values, ending at the next token that reads as an
    /// option — the shape Claude Code's tool lists take.
    Variadic,
}

/// Which of a harness's three tool lists an option writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    /// The tools that exist at all.
    Include,
    /// The tools admitted without a prompt.
    Allow,
    /// The tools denied by name. A denial NARROWS access: it is never an
    /// admission, whatever it names (second council M1).
    Deny,
}

/// What an option does, which is what admission judges. The value of an
/// `Inert` option is data whatever it spells; every other kind carries an
/// effect a realm has to have granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// The value is data: a model name, an effort, a prompt, a path.
    Inert,
    /// A switch with no capability effect of its own. An adapter's guard
    /// may still name it, which is judged separately.
    Switch,
    /// A `key=value` assignment into the harness's configuration.
    Config,
    /// A tool list.
    List(ListKind),
    /// Loads a server, a plugin or an opaque document that can configure
    /// either. Only a realm grant may open such a channel.
    Load,
    /// Names or selects a session the harness would rejoin.
    Session,
}

/// One modelled option of one harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spec {
    /// The name every spelling of this option is judged under.
    pub canonical: &'static str,
    pub aliases: &'static [&'static str],
    pub arity: Arity,
    /// Whether `--name=value` is a supported spelling.
    pub equals: bool,
    /// Whether the value may be ATTACHED to a short option: `-cVALUE`.
    pub attached: bool,
    /// Whether the option may appear more than once.
    pub repeat: bool,
    pub effect: Effect,
}

impl Spec {
    fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        std::iter::once(self.canonical).chain(self.aliases.iter().copied())
    }
}

/// One placed option, with the spelling that placed it and the tokens it
/// occupied. `joined` records that the value arrived inside the option's
/// own token, which is what keeps a serialization from turning an accepted
/// joined value into an ambiguous split pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub spec: &'static Spec,
    pub spelling: String,
    pub values: Vec<String>,
    pub joined: bool,
    /// The index of this option's own token in the argv it was parsed from.
    pub at: usize,
    /// How many tokens the option and its values occupied.
    pub tokens: usize,
}

impl Node {
    /// The canonical name this node is judged under.
    pub fn name(&self) -> &'static str {
        self.spec.canonical
    }

    /// The list this node writes, where it writes one.
    pub fn list(&self) -> Option<ListKind> {
        match self.spec.effect {
            Effect::List(kind) => Some(kind),
            _ => None,
        }
    }
}

/// One harness's whole parsed argv.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Command {
    pub nodes: Vec<Node>,
}

impl Command {
    /// The nodes writing one tool list, in argv order.
    pub fn lists(&self, kind: ListKind) -> impl Iterator<Item = &Node> {
        self.nodes
            .iter()
            .filter(move |node| node.list() == Some(kind))
    }
}

/// Why an argv could not be placed in its harness's grammar. The token and
/// its position are named; a VALUE is never echoed, because a value can
/// carry a secret and a refusal is read in a journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub harness: String,
    pub token: String,
    pub at: usize,
    pub cause: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the '{}' command grammar cannot place argument {} ('{}'): it {}. A harness brokkr \
             launches is parsed against a model of its options, and a token that grammar cannot \
             place is refused rather than passed through, because a control nobody can read is a \
             control nobody can rule on (decision 0066 ruling 6)",
            self.harness,
            self.at + 1,
            self.token,
            self.cause
        )
    }
}

/// One harness's modelled options. No supported invocation carries a
/// bare positional word: the prompt reaches every harness on stdin and
/// the session identifiers are the engine's to place, so a bare word is
/// a token the grammar cannot put anywhere.
pub struct Grammar {
    pub harness: &'static str,
    pub options: &'static [Spec],
}

/// The two shapes most of a table is written in: a value-taking option
/// whose value is data, and a switch.
pub(crate) const fn inert(canonical: &'static str, aliases: &'static [&'static str]) -> Spec {
    Spec {
        canonical,
        aliases,
        arity: Arity::One,
        equals: true,
        attached: false,
        repeat: false,
        effect: Effect::Inert,
    }
}

pub(crate) const fn switch(canonical: &'static str, aliases: &'static [&'static str]) -> Spec {
    Spec {
        canonical,
        aliases,
        arity: Arity::Bare,
        equals: false,
        attached: false,
        repeat: false,
        effect: Effect::Switch,
    }
}

/// `codex exec`'s supported options, as the installed 0.154.0 help gives
/// them. `-c/--config` is the one assignment channel, and all five of its
/// spellings — `-c V`, `-c=V`, `-cV`, `--config V`, `--config=V` — parse
/// to the same node, so the realm-only judgment cannot be escaped by
/// choosing a spelling (second council H1).
static CODEX: &[Spec] = &[
    Spec {
        canonical: "--config",
        aliases: &["-c"],
        arity: Arity::One,
        equals: true,
        attached: true,
        repeat: true,
        effect: Effect::Config,
    },
    Spec {
        canonical: "--model",
        aliases: &["-m"],
        arity: Arity::One,
        equals: true,
        attached: true,
        repeat: false,
        effect: Effect::Inert,
    },
    inert("--effort", &[]),
    Spec {
        canonical: "--sandbox",
        aliases: &["-s"],
        arity: Arity::One,
        equals: true,
        attached: true,
        repeat: false,
        effect: Effect::Inert,
    },
    Spec {
        canonical: "--cd",
        aliases: &["-C"],
        arity: Arity::One,
        equals: true,
        attached: true,
        repeat: false,
        effect: Effect::Inert,
    },
    Spec {
        canonical: "--image",
        aliases: &["-i"],
        arity: Arity::One,
        equals: true,
        attached: true,
        repeat: true,
        effect: Effect::Inert,
    },
    Spec {
        canonical: "--output-last-message",
        aliases: &["-o"],
        arity: Arity::One,
        equals: true,
        attached: true,
        repeat: false,
        effect: Effect::Inert,
    },
    inert("--output-schema", &[]),
    inert("--color", &[]),
    Spec {
        canonical: "--add-dir",
        aliases: &[],
        arity: Arity::One,
        equals: true,
        attached: false,
        repeat: true,
        effect: Effect::Inert,
    },
    Spec {
        canonical: "--ask-for-approval",
        aliases: &["-a"],
        arity: Arity::One,
        equals: true,
        attached: true,
        repeat: false,
        effect: Effect::Inert,
    },
    // A named profile is another configuration document, and what it can
    // configure includes servers. It is a loading channel until its
    // bounded semantics are modelled.
    Spec {
        canonical: "--profile",
        aliases: &["-p"],
        arity: Arity::One,
        equals: true,
        attached: true,
        repeat: false,
        effect: Effect::Load,
    },
    switch("--json", &[]),
    switch("--include-plan-tool", &[]),
    switch("--full-auto", &[]),
    switch("--dangerously-bypass-approvals-and-sandbox", &[]),
    switch("--skip-git-repo-check", &[]),
    switch("--search", &[]),
];

/// The tool-list options Claude Code and LaneTally's wrapper share. Each
/// is VARIADIC: `--allowedTools Read mcp__x__fetch` admits two tools, and
/// the second is judged exactly as the first (second council H2).
const CLAUDE_TOOLS: &[Spec] = &[
    Spec {
        canonical: "--tools",
        aliases: &[],
        arity: Arity::Variadic,
        equals: true,
        attached: false,
        repeat: false,
        effect: Effect::List(ListKind::Include),
    },
    Spec {
        canonical: "--allowedTools",
        aliases: &["--allowed-tools"],
        arity: Arity::Variadic,
        equals: true,
        attached: false,
        repeat: false,
        effect: Effect::List(ListKind::Allow),
    },
    Spec {
        canonical: "--disallowedTools",
        aliases: &["--disallowed-tools"],
        arity: Arity::Variadic,
        equals: true,
        attached: false,
        repeat: false,
        effect: Effect::List(ListKind::Deny),
    },
];

/// Claude Code's supported options, as the installed 2.1.266 help gives
/// them. `--plugin-dir` stands beside `--mcp-config` and `--settings`:
/// all three load something that can configure a server, and none of them
/// is a channel a recipe may open (second council H2).
static CLAUDE: &[Spec] = &[
    CLAUDE_TOOLS[0],
    CLAUDE_TOOLS[1],
    CLAUDE_TOOLS[2],
    Spec {
        canonical: "--mcp-config",
        aliases: &[],
        arity: Arity::Variadic,
        equals: true,
        attached: false,
        repeat: true,
        effect: Effect::Load,
    },
    Spec {
        canonical: "--plugin-dir",
        aliases: &[],
        arity: Arity::Variadic,
        equals: true,
        attached: false,
        repeat: true,
        effect: Effect::Load,
    },
    Spec {
        canonical: "--settings",
        aliases: &[],
        arity: Arity::One,
        equals: true,
        attached: false,
        repeat: false,
        effect: Effect::Load,
    },
    Spec {
        canonical: "--agents",
        aliases: &[],
        arity: Arity::One,
        equals: true,
        attached: false,
        repeat: false,
        effect: Effect::Load,
    },
    switch("--strict-mcp-config", &[]),
    Spec {
        canonical: "--print",
        aliases: &["-p"],
        arity: Arity::Bare,
        equals: false,
        attached: false,
        repeat: false,
        effect: Effect::Switch,
    },
    switch("--verbose", &[]),
    switch("--no-session-persistence", &[]),
    switch("--fork-session", &[]),
    switch("--bg", &[]),
    inert("--output-format", &[]),
    inert("--input-format", &[]),
    inert("--model", &[]),
    inert("--fallback-model", &[]),
    inert("--effort", &[]),
    inert("--permission-mode", &[]),
    inert("--system-prompt", &[]),
    inert("--append-system-prompt", &[]),
    inert("--system-prompt-snapshot", &[]),
    inert("--max-turns", &[]),
    Spec {
        canonical: "--add-dir",
        aliases: &[],
        arity: Arity::Variadic,
        equals: true,
        attached: false,
        repeat: true,
        effect: Effect::Inert,
    },
    Spec {
        canonical: "--session-id",
        aliases: &[],
        arity: Arity::One,
        equals: true,
        attached: false,
        repeat: false,
        effect: Effect::Session,
    },
    Spec {
        canonical: "--resume",
        aliases: &["-r"],
        arity: Arity::One,
        equals: true,
        attached: false,
        repeat: false,
        effect: Effect::Session,
    },
    Spec {
        canonical: "--continue",
        aliases: &["-c"],
        arity: Arity::Bare,
        equals: false,
        attached: false,
        repeat: false,
        effect: Effect::Session,
    },
];

/// DSH's supported invocation. The engine extracts the model, the effort
/// and the one bound route overlay; every other argument is already
/// refused before any provider work, so the grammar is exactly those
/// three and nothing else — there is no forwarded remainder, and LaneTally
/// inherits nothing from Claude here.
static DSH: &[Spec] = &[
    inert("--model", &[]),
    inert("--effort", &[]),
    Spec {
        canonical: "--patch",
        aliases: &[],
        arity: Arity::One,
        equals: true,
        attached: false,
        repeat: false,
        effect: Effect::Inert,
    },
];

static CODEX_GRAMMAR: Grammar = Grammar {
    harness: "codex",
    options: CODEX,
};
static CLAUDE_GRAMMAR: Grammar = Grammar {
    harness: "claude",
    options: CLAUDE,
};
/// LaneTally wraps the same harness and forwards the same option grammar,
/// so it is parsed by the same table under its own name — not by importing
/// Claude's native inventory, which stays Claude's.
static LANETALLY_GRAMMAR: Grammar = Grammar {
    harness: "lanetally",
    options: CLAUDE,
};
static DSH_GRAMMAR: Grammar = Grammar {
    harness: "dsh",
    options: DSH,
};

/// Every table, for the sweep that keeps their shapes honest.
pub const TABLES: [&Grammar; 4] = [
    &CODEX_GRAMMAR,
    &CLAUDE_GRAMMAR,
    &LANETALLY_GRAMMAR,
    &DSH_GRAMMAR,
];

/// The grammar of a harness brokkr knows, or `None` for one it does not:
/// `exec` runs an operator's own command line and a custom driver is
/// opaque, so neither has a modelled option table and neither claims one.
pub fn grammar(harness: &str) -> Option<&'static Grammar> {
    Some(match harness {
        "codex" => &CODEX_GRAMMAR,
        "claude" => &CLAUDE_GRAMMAR,
        "lanetally" => &LANETALLY_GRAMMAR,
        "dsh" => &DSH_GRAMMAR,
        _ => return None,
    })
}

/// The tool list one flag NAME writes in a harness's grammar, where it
/// writes one. An adapter's selection mapping is checked against this, so
/// a mapping onto a flag the harness has no such list for is adapter data
/// that cannot reach a final command, rather than a silent mis-fold.
pub fn list_of(harness: &str, flag: &str) -> Option<ListKind> {
    grammar(harness)?
        .find(flag)
        .and_then(|spec| match spec.effect {
            Effect::List(kind) => Some(kind),
            _ => None,
        })
}

/// Whether a token reads as an option rather than as a value. The lone
/// `-` is stdin, which is a positional; the lone `--` is the terminator,
/// which no supported shape here carries.
fn reads_as_option(token: &str) -> bool {
    token.starts_with('-') && token != "-"
}

impl Grammar {
    fn find(&self, name: &str) -> Option<&'static Spec> {
        self.options
            .iter()
            .find(|spec| spec.names().any(|known| known == name))
    }

    /// The longest modelled short option whose value may be ATTACHED and
    /// which this token starts with: `-cmcp_servers.x=1` is `-c` carrying
    /// `mcp_servers.x=1`, and is the same node `-c mcp_servers.x=1` is.
    fn attached(&self, token: &str) -> Option<(&'static Spec, &'static str, String)> {
        self.options
            .iter()
            .filter(|spec| spec.attached)
            .flat_map(|spec| spec.names().map(move |name| (spec, name)))
            .filter(|(_, name)| !name.starts_with("--") && name.len() < token.len())
            .filter(|(_, name)| token.starts_with(name))
            .max_by_key(|(_, name)| name.len())
            .map(|(spec, name)| (spec, name, token[name.len()..].to_string()))
    }

    /// Parse one argv against this grammar, or say which token it could
    /// not place and why.
    pub fn parse(&self, argv: &[String]) -> Result<Command, Problem> {
        let problem = |at: usize, token: &str, cause: &str| Problem {
            harness: self.harness.to_string(),
            token: token.to_string(),
            at,
            cause: cause.to_string(),
        };
        let mut nodes: Vec<Node> = Vec::new();
        let mut index = 0;
        while index < argv.len() {
            let token = argv[index].as_str();
            let start = index;
            index += 1;
            if !reads_as_option(token) {
                return Err(problem(
                    start,
                    token,
                    "is a bare word, and no positional argument is part of the supported shape",
                ));
            }
            // `--name=value`, then `-cvalue`, then the bare name: the
            // joined spellings are tried first so a name that is also a
            // prefix of another cannot claim the wrong one.
            let (spec, spelling, mut values, joined) = match token.split_once('=') {
                Some((name, value)) if self.find(name).is_some_and(|spec| spec.equals) => (
                    self.find(name).expect("the name matched"),
                    name.to_string(),
                    vec![value.to_string()],
                    true,
                ),
                _ => match self.find(token) {
                    Some(spec) => (spec, token.to_string(), Vec::new(), false),
                    None => match self.attached(token) {
                        Some((spec, name, value)) => (spec, name.to_string(), vec![value], true),
                        None => {
                            return Err(problem(
                                start,
                                token,
                                match token.contains('=') {
                                    true => {
                                        "names no option, or names one that has no \
                                             equals-joined spelling"
                                    }
                                    false => "names no option",
                                },
                            ))
                        }
                    },
                },
            };
            // A joined value can only have come from an option that
            // declares a joined spelling, and no switch does — an
            // invariant the tables are swept for, so `--verbose=1` is
            // simply a name no option has.
            if !joined {
                match spec.arity {
                    Arity::Bare => {}
                    Arity::One | Arity::Variadic => {
                        // A split value that itself reads as an option is
                        // ambiguous: it may be this option's value or the
                        // next option, and a harness that reads it the
                        // other way turns a required denial into a prompt
                        // (second council H3). It is refused, never
                        // guessed at. The joined spelling stays available
                        // for text that has to start with a dash.
                        match argv.get(index) {
                            Some(value) if !reads_as_option(value) => {
                                values.push(value.clone());
                                index += 1;
                            }
                            Some(value) => {
                                return Err(problem(
                                    index,
                                    value,
                                    &format!(
                                        "stands where the value of '{}' belongs but reads as \
                                         an option, so which of the two it is cannot be told",
                                        spec.canonical
                                    ),
                                ))
                            }
                            None => {
                                return Err(problem(
                                    start,
                                    token,
                                    "takes a value and is the last argument, so it has none",
                                ))
                            }
                        }
                    }
                }
                if spec.arity == Arity::Variadic {
                    while let Some(value) = argv.get(index).filter(|value| !reads_as_option(value))
                    {
                        values.push(value.clone());
                        index += 1;
                    }
                }
            }
            if !spec.repeat
                && nodes
                    .iter()
                    .any(|node| node.spec.canonical == spec.canonical)
            {
                return Err(problem(
                    start,
                    token,
                    &format!(
                        "repeats option '{}', which the grammar admits once; a CLI that \
                         resolves a duplicate last-wins would resolve it against the control \
                         the engine composed",
                        spec.canonical
                    ),
                ));
            }
            nodes.push(Node {
                spec,
                spelling,
                values,
                joined,
                at: start,
                tokens: index - start,
            });
        }
        Ok(Command { nodes })
    }
}

/// Parse one harness's argv, where brokkr has a grammar for it. `None` is
/// a harness with no modelled table: `exec` and an opaque custom driver,
/// whose final command brokkr never composes.
pub fn parse(harness: &str, argv: &[String]) -> Option<Result<Command, Problem>> {
    grammar(harness).map(|grammar| grammar.parse(argv))
}

/// The key of a Codex configuration assignment: `mcp_servers.brokkr.args=[]`
/// keys `mcp_servers.brokkr.args`. Quoting and surrounding space are folded
/// away so a quoted spelling of a key is the same key.
pub fn config_key(value: &str) -> String {
    value
        .split('=')
        .next()
        .unwrap_or_default()
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '"' && *c != '\'')
        .collect()
}

/// Whether a Codex configuration key stands in, or under, the table
/// `name` — judged on the DOTTED components, so `mcp_serversx` is not
/// under `mcp_servers`.
pub fn config_under(key: &str, name: &str) -> bool {
    key == name || key.starts_with(&format!("{name}."))
}

/// A tool pattern's name: `WebFetch(domain:example.org)` names `WebFetch`.
pub fn tool_name(pattern: &str) -> &str {
    pattern.split('(').next().unwrap_or(pattern).trim()
}

/// The patterns of one tool-list value, split at a comma or a space that
/// stands OUTSIDE parentheses: `Bash(git log:*),Read` is two patterns, and
/// the star inside the first belongs to its argument, not to a tool name.
pub fn tool_patterns(value: &str) -> Vec<&str> {
    let (mut patterns, mut depth, mut start) = (Vec::new(), 0usize, 0);
    for (index, c) in value.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' | ' ' if depth == 0 => {
                patterns.push(&value[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    patterns.push(&value[start..]);
    patterns
}

/// Every tool pattern one list node names, across all of its values.
pub fn node_patterns(node: &Node) -> Vec<&str> {
    node.values
        .iter()
        .flat_map(|value| tool_patterns(value))
        .filter(|pattern| !pattern.is_empty())
        .collect()
}
