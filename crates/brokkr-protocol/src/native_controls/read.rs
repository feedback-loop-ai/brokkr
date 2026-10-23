//! The capability state one parsed command expresses, read for the final
//! check (rebuild units 13, 13-fix and 13-fix-b; design D6). Moved here
//! from `native_controls.rs`, which sits at its line baseline, when the
//! reading learned Codex's isolation options and permission profile
//! (decision 0065 slice two, U9c1; SI2).

use super::grammar::{self, Bound, Command, Effect, Node};
use super::{Expressed, Isolation, SandboxIntent, State, CLASSES};

/// The native capability one assignment switches OFF, where it is exactly
/// a declared, measured OFF: on [`grammar::LAUNCH_SETTINGS`] in the one
/// canonical spelling [`grammar::launch_setting`] reads, with the admitted
/// value a declared OFF argv's.
fn measured_off(assignment: &str) -> Option<&'static str> {
    let key = grammar::launch_setting(assignment).ok()?;
    grammar::LAUNCH_SETTINGS
        .iter()
        .find(|admitted| admitted.key == key)
        .and_then(|admitted| admitted.denies)
}

impl Isolation {
    /// The option a refusal names this isolation by.
    pub(super) fn option(self) -> &'static str {
        match self {
            Isolation::UserConfig => "the isolation option '--ignore-user-config'",
            Isolation::Apps => "the isolation option '--disable apps'",
            Isolation::Plugins => "the isolation option '--disable plugins'",
        }
    }
}

impl super::Serving<'_> {
    /// The paths this serving binds a permission profile to: its workdir
    /// and the store's handed path.
    pub(super) fn bound(&self) -> Bound<'_> {
        Bound {
            workdir: self.workdir,
            store: self.store,
        }
    }
}

/// Why one parsed command's [`State`] cannot be read, each at the argument
/// it stands at, in the fixed words [`read_state`] renders: an option's
/// canonical name and a grammar's fixed cause, never a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum Unread {
    #[error(
        "carries '{0}' (argument {1}), whose value names no sandbox class: read-only, \
         workspace-write or danger-full-access"
    )]
    Classless(&'static str, usize),
    #[error("expresses the sandbox class a second time (argument {0})")]
    Reclassed(usize),
    #[error("carries '{0}' (argument {1}), a session selector other than a rejoin's")]
    Selector(&'static str, usize),
    #[error("carries '{0}' (argument {1}), whose value {2}")]
    Listed(&'static str, usize, &'static str),
    #[error("carries '--config' (argument {0}), whose value {1}")]
    Unbounded(usize, &'static str),
    #[error(
        "carries '--config' (argument {0}) into the '{1}' configuration in a spelling or with a \
         value whose meaning is not established, so it is read as neither ON nor OFF"
    )]
    Unestablished(usize, &'static str),
    #[error(transparent)]
    Unisolated(#[from] grammar::Unisolated),
}

/// The class `word` names, expressed by `node` at argument `at`, where
/// `state` expresses none yet; or why it cannot be read.
fn class(state: &State, node: &Node, at: usize, word: &str) -> Result<Expressed, Unread> {
    let Some(class) = CLASSES.iter().find(|class| class.word() == word) else {
        return Err(Unread::Classless(node.name(), at));
    };
    match state.class() {
        None => Ok(Expressed::Class(class.word())),
        Some(_) => Err(Unread::Reclassed(at)),
    }
}

/// The managed patterns `node`'s list values write, or why one cannot be
/// read.
fn listed(node: &Node, at: usize) -> Result<Vec<String>, Unread> {
    let mut patterns = Vec::new();
    for value in &node.values {
        let read = grammar::managed_patterns(value)
            .map_err(|cause| Unread::Listed(node.name(), at, cause))?;
        patterns.extend(read.into_iter().map(str::to_string));
    }
    Ok(patterns)
}

/// The effect one assignment `value` of `node` at argument `at` expresses
/// beside `state`, `None` for an inert one or a profile's definition, or
/// why it cannot be read. An assignment into a capability table is read by
/// meaning or refused: the selector of the command's `profile`, exactly a
/// rejoin's class, exactly a measured OFF, or a capability server's entry.
fn configured(
    state: &State,
    node: &Node,
    value: &str,
    profile: Option<SandboxIntent>,
    at: usize,
) -> Result<Option<Expressed>, Unread> {
    let table = match grammar::setting(value) {
        Ok(grammar::Setting::Inert(_) | grammar::Setting::Capability("permissions")) => {
            return Ok(None)
        }
        Ok(grammar::Setting::Capability("default_permissions")) => {
            return class(state, node, at, profile.map_or("", |class| class.word())).map(Some)
        }
        Ok(grammar::Setting::Capability(table)) => table,
        Err(cause) => return Err(Unread::Unbounded(at, cause)),
    };
    if let Some(known) = CLASSES
        .iter()
        .find(|known| grammar::rejoin_class(**known) == value)
    {
        class(state, node, at, known.word()).map(Some)
    } else if let Some(capability) = measured_off(value) {
        Ok(Some(Expressed::Off(capability)))
    } else if table == "mcp_servers" {
        Ok(Some(Expressed::Control(
            node.name(),
            vec![value.to_string()],
        )))
    } else {
        Err(Unread::Unestablished(at, table))
    }
}

/// Add each effect the assignments of `node` at argument `at` express to
/// `state` ([`configured`]), or say why one cannot be read.
fn configure(
    state: &mut State,
    node: &Node,
    profile: Option<SandboxIntent>,
    at: usize,
) -> Result<(), Unread> {
    for value in &node.values {
        let effect = configured(state, node, value, profile, at)?;
        state.effects.extend(effect);
    }
    Ok(())
}

/// The effect a capability-bearing option `node` expresses: an isolation
/// option, the cause of a toggle the engine does not compose, or the
/// control itself.
fn controlled(node: &Node) -> Result<Expressed, Unread> {
    match grammar::isolation(node) {
        Some(read) => Ok(Expressed::Isolation(read?)),
        None => Ok(Expressed::Control(node.name(), node.values.clone())),
    }
}

/// Read one parsed command's [`State`], or the fixed words of what cannot
/// be read: a list value outside the managed grammar, an assignment with
/// no bounded meaning, an assignment into a capability table whose meaning
/// is not established, a class that is none or is expressed twice, or a
/// session selector other than a rejoin's. Nothing read is echoed.
///
/// An assignment into a capability table is read by meaning or refused
/// (rebuild unit 13-fix, F3; [`configured`]). Any other spelling or value —
/// a spaced key, a trailing space, `web_search="live"`, `web_search=garbage`,
/// a web search mode, a sandbox or approval table — is read as neither ON
/// nor OFF, and refuses.
///
/// Codex's permission profile is read whole first, bound to `bound`
/// ([`grammar::profile`]): its class is expressed where its selector
/// stands, and the definition expresses nothing of its own. Each isolation
/// option is its own effect, and a feature toggle the engine does not
/// compose refuses (U9c1; SI2).
pub(super) fn read_state(command: &Command, bound: Bound<'_>) -> Result<State, String> {
    expressed(command, bound).map_err(|fault| fault.to_string())
}

/// [`read_state`], its fault typed.
pub(super) fn expressed(command: &Command, bound: Bound<'_>) -> Result<State, Unread> {
    let profile = grammar::profile(command, bound)?;
    let mut state = State::default();
    for node in &command.nodes {
        let at = node.at + 1;
        match node.spec.effect {
            Effect::List(kind) => state.effects.push(Expressed::List(kind, listed(node, at)?)),
            Effect::Config => configure(&mut state, node, profile, at)?,
            Effect::Control(grammar::Power::Permission) if node.name() == "--sandbox" => {
                let effect = class(&state, node, at, &node.values[0])?;
                state.effects.push(effect);
            }
            Effect::Control(_) | Effect::Load => state.effects.push(controlled(node)?),
            Effect::Session if node.name() == "--resume" => {
                state.session = Some(node.values[0].clone())
            }
            Effect::Session => return Err(Unread::Selector(node.name(), at)),
            // A switch that copies, forks or relocates a conversation is a
            // selector, as the adapter's structural reader judges it.
            Effect::Switch if crate::adapters::CLAUDE_SELECTORS_BARE.contains(&node.name()) => {
                return Err(Unread::Selector(node.name(), at))
            }
            Effect::Inert | Effect::Switch | Effect::Route => {}
        }
    }
    Ok(state)
}
