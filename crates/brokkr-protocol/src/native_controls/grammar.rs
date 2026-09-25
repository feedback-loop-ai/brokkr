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

/// What a capability-bearing control governs (decision 0065 slice one,
/// rebuild unit 10). The class is fixed by the grammar and never read from
/// the option's value: a permission mode is a permission control whichever
/// mode it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Power {
    /// A permission mode, an approval policy or a sandbox class, or a
    /// switch that lifts or replaces one.
    Permission,
    /// An additional directory, which grants file access.
    Filesystem,
    /// How MCP configuration is honoured.
    Mcp,
    /// A tool the harness offers beside its own set.
    Tools,
    /// Web search.
    Web,
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
    /// A `key=value` assignment into the harness's configuration, whose
    /// meaning is each assignment's [`Setting`].
    Config,
    /// A tool list.
    List(ListKind),
    /// Loads a server, a plugin or an opaque document that can configure
    /// either. Only a realm grant may open such a channel.
    Load,
    /// Names or selects a session the harness would rejoin.
    Session,
    /// A capability-bearing control of the operator ruling's catalogue
    /// (operator ruling 1 of 2026-09-23), in every spelling and whatever
    /// its value.
    Control(Power),
    /// DSH's route-only overlay: a path to one bound, contained,
    /// digest-checked route document, checked before staging by the
    /// adapter's own route check. It carries no capability, and the
    /// grammar admits it once.
    Route,
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

    /// Whether this node bears a capability, whatever its value (rebuild
    /// unit 10). A list, a load and a catalogue control always do. A
    /// configuration node does when ANY of its assignments does: every
    /// occurrence is classified on its own, so a harmless assignment beside
    /// a forbidden one cannot erase it, and an assignment with no bounded
    /// meaning is refused with its fixed cause rather than read as inert.
    /// A session selector, DSH's route overlay, a switch and inert data do
    /// not; each has its own fence.
    pub fn bears_capability(&self) -> Result<bool, &'static str> {
        match self.spec.effect {
            Effect::List(_) | Effect::Load | Effect::Control(_) => Ok(true),
            Effect::Config => {
                let mut bears = false;
                for value in &self.values {
                    bears |= matches!(setting(value)?, Setting::Capability(_));
                }
                Ok(bears)
            }
            Effect::Inert | Effect::Switch | Effect::Session | Effect::Route => Ok(false),
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

/// Why an argv could not be placed in its harness's grammar. The position
/// and a bounded label are named; the token itself is never rendered,
/// because a joined, attached or misplaced token can carry a value, a
/// value can carry a secret, and a refusal is read in a journal (rebuild
/// unit 10; design D6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub harness: String,
    /// The raw token, for a caller that builds its own bounded label from
    /// it. It is never part of this problem's rendering.
    pub token: String,
    /// The token's bounded, value-free label: see [`Grammar::label`].
    pub label: String,
    pub at: usize,
    /// Fixed text and canonical option names only.
    pub cause: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the '{}' command grammar cannot place argument {} ({}): it {}. A harness brokkr \
             launches is parsed against a model of its options, and a token that grammar cannot \
             place is refused rather than passed through, because a control nobody can read is a \
             control nobody can rule on (decision 0066 ruling 6)",
            self.harness,
            self.at + 1,
            self.label,
            self.cause
        )
    }
}

/// The label of a token the grammar names no option for and cannot safely
/// spell: an unmodelled short or attached form, or a name that is not a
/// plain bounded one.
pub const UNMODELLED_LABEL: &str =
    "an option the grammar does not model, whose spelling is not echoed";
/// The label of a token that does not read as an option.
pub const POSITIONAL_LABEL: &str = "a positional argument, whose text is not echoed";
/// The longest unmodelled long option name a label spells, in bytes.
const PLAIN_NAME_MAX: usize = 64;

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

/// A catalogue switch: no value, and the capability class it governs.
pub(crate) const fn control(canonical: &'static str, power: Power) -> Spec {
    Spec {
        effect: Effect::Control(power),
        ..switch(canonical, &[])
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
        effect: Effect::Control(Power::Permission),
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
        effect: Effect::Control(Power::Filesystem),
    },
    Spec {
        canonical: "--ask-for-approval",
        aliases: &["-a"],
        arity: Arity::One,
        equals: true,
        attached: true,
        repeat: false,
        effect: Effect::Control(Power::Permission),
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
    control("--include-plan-tool", Power::Tools),
    control("--full-auto", Power::Permission),
    control(
        "--dangerously-bypass-approvals-and-sandbox",
        Power::Permission,
    ),
    switch("--skip-git-repo-check", &[]),
    control("--search", Power::Web),
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
    control("--strict-mcp-config", Power::Mcp),
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
    Spec {
        effect: Effect::Control(Power::Permission),
        ..inert("--permission-mode", &[])
    },
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
        effect: Effect::Control(Power::Filesystem),
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
        effect: Effect::Route,
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

    /// A token's bounded, value-free label (rebuild unit 10; design D6):
    /// the canonical name of the modelled option it spells, read before
    /// any `=` or as a short option carrying an attached value, whichever
    /// alias was written; an unmodelled long name that is plain — ASCII
    /// letters, digits and dashes, at most 64 bytes — as that name alone,
    /// its joined value dropped; the terminator; and otherwise a fixed
    /// label. Truncating a raw token is not redaction, so an over-long or
    /// unplain name is never cut short and spelled: it gets the fixed one.
    pub fn label(&self, token: &str) -> String {
        if !reads_as_option(token) {
            return POSITIONAL_LABEL.to_string();
        }
        if token == "--" {
            return "the terminator '--'".to_string();
        }
        let name = token.split_once('=').map_or(token, |(name, _)| name);
        if let Some(spec) = self
            .find(name)
            .or_else(|| self.attached(token).map(|(spec, _, _)| spec))
        {
            return format!("'{}'", spec.canonical);
        }
        let plain = name.len() <= PLAIN_NAME_MAX
            && name.strip_prefix("--").is_some_and(|rest| {
                rest.starts_with(|c: char| c.is_ascii_alphanumeric())
                    && rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            });
        match plain {
            true => format!("'{name}'"),
            false => UNMODELLED_LABEL.to_string(),
        }
    }

    fn problem(&self, at: usize, token: &str, cause: &str) -> Problem {
        Problem {
            harness: self.harness.to_string(),
            token: token.to_string(),
            label: self.label(token),
            at,
            cause: cause.to_string(),
        }
    }

    /// Parse one argv against this grammar, or say which token it could
    /// not place and why.
    pub fn parse(&self, argv: &[String]) -> Result<Command, Problem> {
        self.parse_span(argv, 0, argv.len())
    }

    /// Parse the options standing in `argv[start..end]`, naming every
    /// position in the whole argv. No value is taken from beyond `end`, so
    /// a final positional can never become an option's value.
    fn parse_span(&self, argv: &[String], start: usize, end: usize) -> Result<Command, Problem> {
        let problem = |at: usize, token: &str, cause: &str| self.problem(at, token, cause);
        let argv = &argv[..end];
        let mut nodes: Vec<Node> = Vec::new();
        let mut index = start;
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

    /// Parse a complete serving command after its binary (rebuild unit
    /// 10): the subcommand and trailing positionals the engine places,
    /// each at a fixed position, and the options between them under this
    /// grammar. Codex opens with `exec`; a rejoin is `exec resume`, and
    /// ends with exactly the plain session identifier and the stdin `-`,
    /// which no option's value can reach. Claude and LaneTally select a
    /// session by option and DSH's argv is its driver's input, so neither
    /// carries a positional. Nothing else is a position: an unexpected
    /// word refuses, and `--image resume` stays an image's value. A token
    /// refused at one of these reserved positions is labelled as the
    /// positional it stands for, whatever it spells, so a misplaced
    /// option-looking payload is not echoed as an option name (design D6).
    pub fn parse_final(&self, argv: &[String]) -> Result<Final, Problem> {
        let word = |at: usize| argv.get(at).map(String::as_str);
        let refuse = |at: usize, cause: &str| {
            Err(Problem {
                label: POSITIONAL_LABEL.to_string(),
                ..self.problem(at, word(at).unwrap_or_default(), cause)
            })
        };
        if self.harness != "codex" {
            return Ok(Final {
                subcommands: Vec::new(),
                command: self.parse(argv)?,
                session: None,
            });
        }
        if word(0) != Some("exec") {
            return refuse(
                0,
                "stands where the 'exec' subcommand a codex serving command opens with belongs",
            );
        }
        if word(1) != Some("resume") {
            return Ok(Final {
                subcommands: vec!["exec"],
                command: self.parse_span(argv, 1, argv.len())?,
                session: None,
            });
        }
        if argv.len() < 4 {
            return refuse(
                1,
                "opens a rejoin that does not end with its session identifier and the stdin \
                 positional '-'",
            );
        }
        let (session, stdin) = (argv.len() - 2, argv.len() - 1);
        if word(stdin) != Some("-") {
            return refuse(
                stdin,
                "stands where the stdin positional '-' that ends a rejoin belongs",
            );
        }
        if !plain_session(&argv[session]) {
            return refuse(
                session,
                "stands where a rejoin's session identifier belongs but is not a plain one: \
                 ASCII letters, digits and dashes, not leading with a dash, at most 128 bytes",
            );
        }
        Ok(Final {
            subcommands: vec!["exec", "resume"],
            command: self.parse_span(argv, 2, session)?,
            session: Some(argv[session].clone()),
        })
    }
}

/// A complete serving command, parsed at its fixed positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Final {
    /// The subcommand words, in order: `["exec"]` or `["exec", "resume"]`
    /// for codex, none for the others.
    pub subcommands: Vec<&'static str>,
    /// The options between the subcommands and the trailing positionals.
    pub command: Command,
    /// The session a codex rejoin names, before its stdin `-`.
    pub session: Option<String>,
}

/// A codex thread identifier as a rejoin may carry it positionally.
fn plain_session(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && !id.starts_with('-')
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Parse one harness's argv, where brokkr has a grammar for it. `None` is
/// a harness with no modelled table: `exec` and an opaque custom driver,
/// whose final command brokkr never composes.
pub fn parse(harness: &str, argv: &[String]) -> Option<Result<Command, Problem>> {
    grammar(harness).map(|grammar| grammar.parse(argv))
}

/// Parse one harness's complete serving command after its binary, where
/// brokkr has a grammar for it: see [`Grammar::parse_final`].
pub fn parse_final(harness: &str, argv: &[String]) -> Option<Result<Final, Problem>> {
    grammar(harness).map(|grammar| grammar.parse_final(argv))
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

/// What one Codex configuration assignment means, read under a bounded
/// key grammar (rebuild unit 10; design D6). Nothing here evaluates TOML:
/// the key is read into its dotted parts, and the meaning is one of a
/// closed set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    /// An assignment into a capability-bearing table, named canonically:
    /// the table itself, a whole-table value or any descendant, whatever
    /// the value.
    Capability(&'static str),
    /// A non-capability key with a bounded typed meaning, named
    /// canonically, whose value was checked.
    Inert(&'static str),
}

/// The top-level Codex configuration tables that carry a capability
/// (the realm delta's catalogue): MCP servers, web search in every
/// spelling, tool and feature tables, approval and sandbox policy, and a
/// profile, which is another configuration document.
pub const CAPABILITY_TABLES: [&str; 10] = [
    "mcp_servers",
    "web_search",
    "web_search_mode",
    "tools",
    "features",
    "approval_policy",
    "sandbox_mode",
    "sandbox_workspace_write",
    "profile",
    "profiles",
];

/// The one inert key a Codex command needs — the effort the engine
/// translates `--effort` into — and its bounded levels, the adapter's.
const EFFORT_KEY: &str = "model_reasoning_effort";
const EFFORT_LEVELS: [&str; 7] = ["none", "minimal", "low", "medium", "high", "xhigh", "max"];
/// The bounds of a key: its dotted parts, and its bytes.
const KEY_PARTS_MAX: usize = 16;
const KEY_BYTES_MAX: usize = 256;

/// Whitespace TOML admits around a key's dots and around `=`.
fn blank(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// The dotted parts of a key: each a bare name of ASCII letters, digits,
/// `_` and `-`, or a quoted one with no escape. An escape could spell one
/// name as another, so it is not read. [`setting`] has already refused a
/// key holding a control character.
fn key_parts(key: &str) -> Option<Vec<&str>> {
    let mut parts = Vec::new();
    let mut rest = key.trim_matches(blank);
    loop {
        let (part, tail) = match rest.chars().next()? {
            quote @ ('"' | '\'') => {
                let body = &rest[1..];
                let end = body.find(quote)?;
                let part = &body[..end];
                if part.is_empty() || part.contains('\\') {
                    return None;
                }
                (part, &body[end + 1..])
            }
            _ => {
                let end = rest
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
                    .unwrap_or(rest.len());
                if end == 0 {
                    return None;
                }
                (&rest[..end], &rest[end..])
            }
        };
        parts.push(part);
        let tail = tail.trim_start_matches(blank);
        if tail.is_empty() {
            break;
        }
        rest = tail.strip_prefix('.')?.trim_start_matches(blank);
    }
    (parts.len() <= KEY_PARTS_MAX).then_some(parts)
}

/// The meaning of one `--config` value, `KEY=VALUE`, or the fixed cause
/// it is refused for. The `=` is the first one outside a quoted key part.
/// A capability table is named whatever the value; the one inert key
/// admits only its bounded levels, bare or quoted; every other key, a
/// malformed one and an assignment without a value refuse, so no
/// unclassified configuration passes through as opaque data. The cause
/// never echoes the key or the value.
pub fn setting(assignment: &str) -> Result<Setting, &'static str> {
    const NOT_ASSIGNMENT: &str = "is not a KEY=VALUE configuration assignment";
    const MALFORMED: &str = "assigns through a key the grammar cannot read: each dotted part is a \
                             bare name or a quoted one without escapes, within 16 parts and 256 \
                             bytes";
    const NO_VALUE: &str = "assigns no value";
    const UNCLASSIFIED: &str =
        "assigns a key no bounded meaning is modelled for, so it is refused \
                                rather than passed through as opaque configuration";
    const EFFORT: &str = "assigns 'model_reasoning_effort' a value outside its bounded levels \
                          (none, minimal, low, medium, high, xhigh, max)";
    let mut quote = None;
    let split = assignment.char_indices().find(|&(_, c)| {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(open), _) if open == c => quote = None,
            _ => {}
        }
        quote.is_none() && c == '='
    });
    let Some((at, _)) = split else {
        return Err(NOT_ASSIGNMENT);
    };
    let (key, value) = (&assignment[..at], &assignment[at + 1..]);
    if key.len() > KEY_BYTES_MAX || key.chars().any(char::is_control) {
        return Err(MALFORMED);
    }
    let parts = key_parts(key).ok_or(MALFORMED)?;
    let value = value.trim_matches(blank);
    if value.is_empty() {
        return Err(NO_VALUE);
    }
    if let Some(table) = CAPABILITY_TABLES.iter().find(|table| **table == parts[0]) {
        return Ok(Setting::Capability(table));
    }
    if parts != [EFFORT_KEY] {
        return Err(UNCLASSIFIED);
    }
    let level = ['"', '\'']
        .iter()
        .find_map(|quote| {
            value
                .strip_prefix(*quote)
                .and_then(|value| value.strip_suffix(*quote))
        })
        .unwrap_or(value);
    match EFFORT_LEVELS.contains(&level) {
        true => Ok(Setting::Inert(EFFORT_KEY)),
        false => Err(EFFORT),
    }
}

/// The separators a managed tool list may be joined with. A comma is what
/// the shipped mappings declare; Claude's own splitter also splits at a
/// space outside parentheses, so a space is not a separator the engine
/// may join with.
pub fn managed_separator(separator: &str) -> Result<char, &'static str> {
    match separator {
        "," => Ok(','),
        _ => Err("is not the one separator a managed tool list is joined with, ','"),
    }
}

/// The bounds of a managed tool list.
const PATTERNS_MAX: usize = 64;
const TOOL_NAME_MAX: usize = 128;
const SPECIFIER_MAX: usize = 256;

/// The patterns of one MANAGED tool-list value — a list the engine
/// composes — or the fixed cause it is refused for (rebuild unit 10;
/// design D6). The empty value is an explicit empty list, distinct from
/// no list. Otherwise the value is patterns joined by single commas, each
/// a plain tool name (ASCII letters, digits and `_`, leading with a
/// letter) optionally followed by one parenthesized specifier. The
/// specifier may hold spaces, `:`, `*`, `/` and `.`, but no parenthesis,
/// comma, quote, backslash or control character, because each is a place
/// where this reading and the harness's own splitter could disagree. A
/// wildcard never stands in a tool name, an empty pattern (a doubled,
/// leading or trailing comma) refuses, and so does a space outside the
/// parentheses. The cause never echoes the value.
pub fn managed_patterns(value: &str) -> Result<Vec<&str>, &'static str> {
    const EMPTY: &str = "joins an empty pattern: a doubled, leading or trailing separator";
    const NAME: &str = "names a tool that is not a plain name of ASCII letters, digits and '_' \
                        leading with a letter, within 128 bytes";
    const SPECIFIER: &str = "carries a specifier that is not one parenthesized, nonempty run \
                             within 256 bytes without a parenthesis, comma, quote, backslash or \
                             control character";
    const COUNT: &str = "joins more than 64 patterns";
    if value.is_empty() {
        return Ok(Vec::new());
    }
    let patterns: Vec<&str> = value.split(',').collect();
    if patterns.len() > PATTERNS_MAX {
        return Err(COUNT);
    }
    for pattern in &patterns {
        if pattern.is_empty() {
            return Err(EMPTY);
        }
        let (name, specifier) = match pattern.split_once('(') {
            Some((name, rest)) => (name, Some(rest)),
            None => (*pattern, None),
        };
        if name.is_empty()
            || name.len() > TOOL_NAME_MAX
            || !name.starts_with(|c: char| c.is_ascii_alphabetic())
            || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(NAME);
        }
        if let Some(rest) = specifier {
            let inner = rest.strip_suffix(')').ok_or(SPECIFIER)?;
            if inner.is_empty()
                || inner.len() > SPECIFIER_MAX
                || inner
                    .chars()
                    .any(|c| matches!(c, '(' | ')' | ',' | '"' | '\'' | '\\') || c.is_control())
            {
                return Err(SPECIFIER);
            }
        }
    }
    Ok(patterns)
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
