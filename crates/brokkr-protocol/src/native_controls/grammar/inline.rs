//! The judgments of an inline Codex launch (rebuild units 5d-fix-b to
//! 5d-fix-c2; operator ruling of 2026-09-25, "narrow"; design D5.3), moved
//! here from `grammar.rs`, which sits at its line baseline, when they learned
//! the engine's isolation options and its permission profile, read bound to
//! the seat's workspace root and the secrets store's handed path exactly as
//! the final check reads it (decision 0065 slice two, U9c2; SI2; operator
//! ruling 4 of 2026-10-23).

use super::config::{rendered, table, Bound, Unisolated};
use super::{
    isolation, launch_setting, parse, parse_final, profile, rejoin_class, BoundedCause, Effect,
    Node, Power,
};
use crate::native_controls::{flatten, Origin, SandboxIntent, Segment};

/// The class a command's one permission profile renders, bound ([`profile`]).
type Profiled = Result<Option<SandboxIntent>, Unisolated>;

/// The options of an inline Codex launch's contributions, read together under
/// the codex grammar as the harness reads them, each beside the origin of the
/// segment it stands in (rebuild unit 5d-fix), and the class the command's one
/// permission profile renders bound to `bound` ([`profile`]; U9c2), `None`
/// where it defines none, or why it is not the engine's. An argv the grammar
/// cannot place is refused naming its position and its bounded label, never
/// its token.
fn codex_contributions(
    segments: &[Segment],
    bound: Bound<'_>,
) -> Result<(Vec<(Origin, Node)>, Profiled), BoundedCause> {
    let command = parse("codex", &flatten(segments))
        .expect("the codex grammar is modelled")
        .map_err(|problem| {
            BoundedCause(format!(
                "cannot be read whole under the 'codex' grammar (argument {}, {}: it {}), so none \
                 of its effects can be judged; an unclassified option is refused, never passed \
                 through",
                problem.at + 1,
                problem.label,
                problem.cause
            ))
        })?;
    let profiled = profile(&command, bound);
    let ends: Vec<(usize, Origin)> = segments
        .iter()
        .scan(0, |end, segment| {
            *end += segment.argv.len();
            Some((*end, segment.origin))
        })
        .collect();
    let nodes = command
        .nodes
        .into_iter()
        .map(|node| {
            let (_, origin) = ends
                .iter()
                .find(|(end, _)| node.at < *end)
                .expect("every option stands in a segment");
            (*origin, node)
        })
        .collect();
    Ok((nodes, profiled))
}

/// What one option of an inline Codex launch is to its judgment.
enum Judged {
    /// The engine's one sandbox fragment of the site's class.
    Class,
    /// The engine's one capture into the result path it owns.
    Capture,
    /// Admitted, and counted as neither.
    Admitted,
    /// Refused, for this effect.
    Refused(String),
}

/// An option the engine's own `local` contribution carries, read as the
/// site's class: its `--sandbox` of exactly `class`; the rejoin's assignment
/// of it (rebuild unit 5d-fix-c2), since `codex exec resume` takes no
/// `--sandbox`; or, where the command's one permission profile is the
/// engine's, bound and of exactly `class` (`profiled`), the profile's
/// selector, its definition beside it, and each measured isolation option
/// (U9c2). `None` for any other option, which is judged as anyone's.
fn engine_option(node: &Node, class: SandboxIntent, profiled: bool) -> Option<Judged> {
    let assigned = node.values.first().and_then(|value| table(value));
    match node.name() {
        "--sandbox" if node.values == [class.word()] => Some(Judged::Class),
        "--config" if node.values == [rejoin_class(class)] => Some(Judged::Class),
        "--config" if profiled && assigned == Some("default_permissions") => Some(Judged::Class),
        "--config" if profiled && assigned == Some("permissions") => Some(Judged::Admitted),
        _ if matches!(isolation(node), Some(Ok(_))) => Some(Judged::Admitted),
        _ => None,
    }
}

/// A result capture: admitted only as the engine's own, in its `local`
/// contribution, into exactly the result path it owns at a gate.
fn captured(origin: Origin, node: &Node, owned_capture: Option<&str>) -> Judged {
    match owned_capture {
        Some(target) if origin == Origin::Local && node.values == [target] => Judged::Capture,
        Some(_) => Judged::Refused(
            "a result capture other than the engine's own into exactly the result path it owns, \
             a harness write path outside the result sink"
                .to_string(),
        ),
        None => Judged::Refused(
            "a result capture at a work seat, whose result is the file the seat writes; a capture \
             the engine does not own is a harness write path outside the result sink"
                .to_string(),
        ),
    }
}

/// One option of an inline Codex launch, standing in the contribution of
/// `origin`, judged by the grammar's own effect classification.
fn judged(
    origin: Origin,
    node: &Node,
    class: SandboxIntent,
    owned_capture: Option<&str>,
    profiled: bool,
) -> Judged {
    let engine = (origin == Origin::Local).then(|| engine_option(node, class, profiled));
    if let Some(Some(judged)) = engine {
        return judged;
    }
    let refused = match (node.name(), node.spec.effect) {
        ("--output-last-message", _) => return captured(origin, node, owned_capture),
        // The grammar types the root selector's value as data, but D5.3
        // refuses it wherever it stands (unit 2-fix A1).
        ("--cd", _) => {
            "a root selector, which moves the root the sandbox class is measured from".to_string()
        }
        (_, Effect::Inert | Effect::Switch) => return Judged::Admitted,
        (_, Effect::Config) => match node
            .values
            .iter()
            .find_map(|value| launch_setting(value).err())
        {
            Some(cause) => format!("a configuration assignment that {cause}"),
            None => return Judged::Admitted,
        },
        (_, Effect::Control(Power::Permission)) => {
            "a permission control, which sets, lifts or replaces the sandbox or its approvals"
                .to_string()
        }
        (_, Effect::Control(Power::Filesystem)) => {
            "a writable root beyond the sandbox class's reach".to_string()
        }
        (_, Effect::Load) => {
            "a configuration document the engine cannot see into, which can set the sandbox"
                .to_string()
        }
        (_, _) => "a capability-bearing control outside the closed set an inline Codex launch \
                   admits"
            .to_string(),
    };
    Judged::Refused(refused)
}

/// The one judgment of a whole inline Codex launch (rebuild unit 5d-fix-b,
/// moved by unit 5d-fix-c1 so admission and the launch call exactly this pure
/// function; operator ruling of 2026-09-25, "narrow"; design D5.3): the
/// authored command, the adapter's template, the engine's `local` fragment
/// and the native plan, in that order, none exempt.
///
/// It admits a closed set of effects, read by the grammar's own effect
/// classification: exactly one sandbox fragment of exactly `class`, in the
/// engine's `local` fragment — its `--sandbox`, the rejoin's assignment, or
/// the engine's permission profile read bound to the run's `paths`, the
/// seat's workspace root and the store's handed path, as the final check
/// reads it, beside each measured isolation option (U9c2); where
/// `owned_capture` names the
/// engine-owned result path (a gate), exactly one `--output-last-message`
/// into exactly it, in that fragment, and where it is `None` (a work seat)
/// none; configuration on the closed [`LAUNCH_SETTINGS`] allowlist; and
/// options whose value is data or that switch nothing. Every other sandbox,
/// approval, writable-root, capture, root-selector, profile or
/// configuration-document effect, every assignment off the allowlist and
/// every option the grammar cannot place refuses, naming the contribution,
/// the position and the canonical option, never a value. A profile that is
/// not the engine's, or renders another class, refuses in its own words, and
/// where no store is handed every profile refuses, as it does where no run's
/// `paths` exist at all: compile has no run, so it hands none (operator
/// ruling of 2026-10-23). The cause reads as the predicate of "the inline
/// Codex launch of seat …".
///
/// [`LAUNCH_SETTINGS`]: super::LAUNCH_SETTINGS
pub fn judge_inline_codex_launch(
    class: SandboxIntent,
    argv: &[Segment],
    owned_capture: Option<&str>,
    paths: Option<Bound<'_>>,
) -> Result<(), BoundedCause> {
    if class == SandboxIntent::Unspecified {
        return Err(BoundedCause(
            "is judged with no sandbox class, so no fragment of the engine's could express the \
             class it was admitted with"
                .to_string(),
        ));
    }
    // No run's paths bind a profile as no store does: the default hands none.
    let (contributions, profiled) = codex_contributions(argv, paths.unwrap_or_default())?;
    let profiled = profiled.map_err(|fault| BoundedCause(fault.to_string()))?;
    if let Some(other) = profiled.filter(|rendered| *rendered != class) {
        return Err(BoundedCause(format!(
            "carries a permission profile rendering the '{}' class where the site's is '{}', and \
             one launch runs one class",
            other.word(),
            class.word()
        )));
    }
    let (mut sandboxes, mut captures) = (0, 0);
    for (origin, node) in contributions {
        match judged(origin, &node, class, owned_capture, profiled.is_some()) {
            Judged::Class => sandboxes += 1,
            Judged::Capture => captures += 1,
            Judged::Admitted => {}
            Judged::Refused(effect) => {
                return Err(BoundedCause(format!(
                    "carries '{}' (argument {}) in its `{}` contribution, {effect}; the launch \
                     admits only the engine's one sandbox fragment of the site's class, at a gate \
                     the engine's one capture into the result path it owns, and configuration on \
                     a closed allowlist, so every other effect is refused rather than reconciled \
                     or ordered, whoever composed it",
                    node.name(),
                    node.at + 1,
                    origin.word()
                )))
            }
        }
    }
    counted(class, sandboxes, owned_capture.is_some() && captures == 0)
}

/// The launch's sandbox fragments counted, and whether a gate carries no
/// capture: exactly one fragment, and a gate's capture.
fn counted(class: SandboxIntent, sandboxes: usize, uncaptured: bool) -> Result<(), BoundedCause> {
    if sandboxes == 0 {
        return Err(BoundedCause(format!(
            "carries no sandbox fragment of the site's '{}' class in the engine's `local` \
             contribution, so the class it was admitted with would not reach the harness",
            class.word()
        )));
    }
    if sandboxes > 1 {
        return Err(BoundedCause(
            "carries the site's class in more than one sandbox fragment, a flag and the rejoin's \
             assignment together; the launch admits exactly one"
                .to_string(),
        ));
    }
    if uncaptured {
        return Err(BoundedCause(
            "is a gate's, and no contribution carries the engine's capture into the result path \
             it owns, which the last-message door needs, so the gate's result could not be \
             delivered"
                .to_string(),
        ));
    }
    Ok(())
}

/// The judgment of an inline Codex launch's FINAL command, exactly as the
/// harness receives it after its binary (rebuild unit 5d-fix-c2; chief F2
/// of runs `0065-rebuild-unit-5d-fix-b-see-t-8067eebc` and
/// `0065-rebuild-unit-5d-fix-c1-see--b800f52a`). The driver composes the
/// command from the contributions it was handed: it translates `--effort`,
/// generates `--json` and `-C`, and moves the class into its lead. `contributions`
/// are the options it placed after its own lead, each beside the origin it
/// came from, the translated effort carrying the origin of the pin it
/// translated, the engine's isolation options its `local` contribution, and
/// the native plan last.
///
/// The command is read whole under the codex grammar at its fixed
/// positions ([`parse_final`]), so a part the composition duplicated is the
/// repeat it is. It must be exactly the driver's own lead — `exec --json -C
/// <workdir>` cold, `exec resume --json` on a rejoin, each followed by the
/// class as the engine's permission profile bound to `workdir` and `store`
/// where a store is handed (U9c2), and otherwise on a rejoin as `-c
/// sandbox_mode="<class>"`, with the rejoin's `<session> -` last — around
/// the contributions, byte for byte, so no part escapes attribution. Then
/// every option but the root the lead names is judged by
/// [`judge_inline_codex_launch`], the same function admission and the
/// dispatch door call, handed the same paths: the lead's `--json` as the
/// adapter's template, the class it carries as the engine's `local` class it
/// re-expresses, and the contributions as handed. Argument positions in a
/// cause count those judged options, from the lead's `--json`.
pub fn judge_inline_codex_command(
    class: SandboxIntent,
    command: &[String],
    contributions: &[Segment],
    owned_capture: Option<&str>,
    workdir: &str,
    store: Option<&str>,
) -> Result<(), BoundedCause> {
    let composed = parse_final("codex", command)
        .expect("the codex grammar is modelled")
        .map_err(|problem| {
            BoundedCause(format!(
                "cannot be read whole as the harness receives it under the 'codex' grammar \
                 (argument {}, {}: it {}), so none of its effects can be judged",
                problem.at + 1,
                problem.label,
                problem.cause
            ))
        })?;
    let text = |parts: &[&str]| {
        parts
            .iter()
            .map(|part| part.to_string())
            .collect::<Vec<_>>()
    };
    let profile = rendered(class.word(), Bound { workdir, store });
    let (lead, classed, trail) = match (&composed.session, profile) {
        (None, profile) => (
            text(&["exec", "--json", "-C", workdir]),
            profile.unwrap_or_default(),
            Vec::new(),
        ),
        (Some(session), profile) => (
            text(&["exec", "resume", "--json"]),
            profile.unwrap_or_else(|| text(&["-c", &rejoin_class(class)])),
            text(&[session, "-"]),
        ),
    };
    let expected = [lead, classed.clone(), flatten(contributions), trail].concat();
    if command != expected.as_slice() {
        return Err(BoundedCause(
            "is not the driver's own lead around the contributions it was handed, so a part of \
             it cannot be attributed to whoever composed it, and a part nobody composed is \
             refused rather than judged by its bytes"
                .to_string(),
        ));
    }
    let generated = (!classed.is_empty()).then(|| Segment::new(Origin::Local, &classed));
    let judged: Vec<Segment> = [Segment::new(Origin::Template, &text(&["--json"]))]
        .into_iter()
        .chain(generated)
        .chain(contributions.iter().cloned())
        .collect();
    let paths = Some(Bound { workdir, store });
    judge_inline_codex_launch(class, &judged, owned_capture, paths)
}
