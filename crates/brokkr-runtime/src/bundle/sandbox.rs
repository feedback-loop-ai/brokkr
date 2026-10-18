//! The steps of design D5.3's two typed-sandbox paths (#288, slice I):
//! the reading of an inline Codex seat's authored command and the
//! fragment its class selects, which
//! [`lower_inline_sandbox`](super::lower_inline_sandbox) reads as its
//! sequence, and the judgment of one link of an agent's chain, which
//! [`admit_local_sandbox`](super::admit_local_sandbox) runs over every link.

use super::{
    admitted_sandbox, authored_sandbox_control, dispatch_driver, expressed_sandbox,
    refuse_authored_control, sandbox_fragment, unplaced_label, CompileError, Contribution,
    SeatClass,
};
use crate::agents::{Adapter, Adapters, Candidate, ResultDoor, Sandbox};
use brokkr_core::realms::Boundary;

/// Operator ruling 1 of 2026-09-23: the engine composes the class as its
/// own contribution, and nothing it composes is merged with or ordered
/// against a control of the author's, read under the harness's grammar.
pub(super) fn refuse_authored_sandbox(
    command: &[String],
    refuse: &impl Fn(String) -> CompileError,
) -> Result<(), CompileError> {
    let grammar = brokkr_protocol::native_controls::grammar::grammar("codex")
        .expect("the codex grammar is modelled");
    let authored = grammar
        .parse(brokkr_protocol::native_controls::harness_arguments(command))
        .map_err(|problem| {
            refuse(format!(
                "while its authored command cannot be read: the 'codex' command grammar cannot \
                 place argument {} ({}), whose token is not echoed because it can carry a value: \
                 it {}. A control nobody can read is a control nobody can rule on, so it is \
                 refused rather than passed through (decision 0066 ruling 6; operator ruling 1 \
                 of 2026-09-23)",
                problem.at + 1,
                unplaced_label(grammar, &problem.token),
                problem.cause
            ))
        })?;
    refuse_authored_control(&authored, authored_sandbox_control, "class", refuse)
}

/// The fragment an inline Codex seat's class selects, named for a refusal,
/// and the door its result leaves by.
pub(super) type InlineFragment<'a> = (&'static str, Option<&'a [String]>, ResultDoor);

/// Rebuild unit 5d-fix (chief F2): a gate delivers through the
/// last-message door alone, captured into the engine-owned result path.
pub(super) fn inline_codex_fragment<'a>(
    adapter: &'a Adapter,
    seat_class: SeatClass,
    refuse: &impl Fn(String) -> CompileError,
) -> Result<InlineFragment<'a>, CompileError> {
    Ok(match seat_class {
        SeatClass::Gate => {
            if adapter.harness.result != ResultDoor::LastMessage {
                return Err(refuse(
                    "at an inline Codex gate, but the codex adapter declares its \
                     `hands.harness.result` door as 'file'; a gate delivers only through the \
                     last-message door, the harness's capture of its final message into the \
                     engine-owned result path, so file delivery is refused (decision 0046 ruling \
                     4; operator ruling of 2026-09-25; rebuild unit 5d-fix)"
                        .to_string(),
                ));
            }
            (
                "`hands.harness.gate` fragment",
                adapter.harness.gate.as_deref(),
                ResultDoor::LastMessage,
            )
        }
        SeatClass::Work => (
            "`hands.harness.work` fragment",
            adapter.harness.work.as_deref(),
            ResultDoor::File,
        ),
    })
}

/// A typed class as each link of an agent's chain is judged for it: the
/// site, the class it requests, its boundary and its own class.
pub(super) struct SandboxAsk<'a> {
    pub(super) what: &'a str,
    pub(super) requested: Sandbox,
    pub(super) boundary: Boundary,
    pub(super) seat_class: SeatClass,
}

/// One link of the chain: its harness, the fragment the engine selects for
/// it, the class that fragment expresses, and the authored command beside.
pub(super) fn judge_sandbox_link(
    ask: &SandboxAsk<'_>,
    link: usize,
    candidate: &Candidate,
    adapters: &Adapters,
) -> Result<(), CompileError> {
    let SandboxAsk {
        what,
        requested,
        boundary,
        seat_class,
    } = *ask;
    let class = requested.name();
    let provider = &candidate.provider;
    // The HARNESS is what the command dispatches, read off the command
    // itself: a provider label is not evidence of a sandbox class.
    let harness = dispatch_driver(&candidate.argv)
        .unwrap_or_else(|| crate::capabilities::OPAQUE_HARNESS.to_string());
    if harness != "codex" {
        return Err(CompileError::Invalid(format!(
            "seat '{what}' link {link} requests 'tools.sandbox' '{class}' but dispatches the \
             '{harness}' harness through provider '{provider}'; only the codex harness's own \
             `--sandbox` fragments express a sandbox class today, and a provider label, a \
             permission mode or an unmodelled driver is not evidence of one — refused under \
             the `{boundary}` boundary (design D5.3)"
        )));
    }
    let adapter = adapters
        .adapter(provider)
        .expect("resolution mapped every link of the chain");
    let Some((part, fragment)) = sandbox_fragment(boundary, seat_class, adapter, candidate) else {
        return Err(CompileError::Invalid(format!(
            "seat '{what}' link {link} requests 'tools.sandbox' '{class}' under the `open` \
             boundary, where a work seat runs at the harness's own default and no engine \
             fragment expresses a class; a presumed provider default is not a representation \
             — refused (design D5.3)"
        )));
    };
    if fragment.is_empty() {
        return Err(CompileError::Invalid(format!(
            "seat '{what}' link {link} requests 'tools.sandbox' '{class}' under the \
             `{boundary}` boundary, but provider '{provider}' supplies no {part} to express \
             it; a missing fragment is not a representation — refused (design D5.3)"
        )));
    }
    let whom = format!("seat '{what}' link {link}");
    judge_fragment_class(ask, link, &whom, provider, part, fragment)?;
    // The other contributions must not carry a competing control: the
    // authored part of the command is read after brokkr's own dispatch
    // tokens, under the same grammar.
    let (authored, _) = candidate.parts();
    let authored = brokkr_protocol::native_controls::harness_arguments(authored);
    if let Some(found) =
        expressed_sandbox(&whom, "authored command", authored, Contribution::Written)?
    {
        return Err(CompileError::Invalid(format!(
            "seat '{what}' link {link} requests 'tools.sandbox' '{class}', but the authored \
             command of provider '{provider}' already carries `--sandbox` '{found}', a \
             competing control the selected {part} would stand beside; authored bytes cannot \
             supply or contest a typed representation — refused under the `{boundary}` \
             boundary (design D5.3)"
        )));
    }
    Ok(())
}

/// The class the selected fragment expresses must be exactly the one
/// requested, and the one design D5.3's table admits on this path. `whom`
/// names the link to the fragment's reading.
fn judge_fragment_class(
    ask: &SandboxAsk<'_>,
    link: usize,
    whom: &str,
    provider: &str,
    part: &str,
    fragment: &[String],
) -> Result<(), CompileError> {
    let SandboxAsk {
        what,
        requested,
        boundary,
        seat_class,
    } = *ask;
    let class = requested.name();
    match expressed_sandbox(whom, part, fragment, Contribution::Written)? {
        Some(found) if found == class => {
            // The fragment represents the class; the TABLE decides
            // whether this path may hold it at all (review return F1).
            let (admitted, site_kind) = admitted_sandbox(boundary, seat_class);
            if requested != admitted {
                let admitted = admitted.name();
                return Err(CompileError::Invalid(format!(
                    "seat '{what}' link {link} requests 'tools.sandbox' '{class}' at \
                     {site_kind} under the `{boundary}` boundary, where decision 0065 slice \
                     one admits only '{admitted}' (design D5.3: a box and a harness gate hold \
                     read-only, harness work holds workspace-write); the {part} of provider \
                     '{provider}' expresses '{class}' too, but adapter data is a \
                     representation and not an authority, so a fragment cannot widen that \
                     table — refused"
                )));
            }
        }
        Some(found) => {
            return Err(CompileError::Invalid(format!(
                "seat '{what}' link {link} requests 'tools.sandbox' '{class}', but the {part} \
                 the engine selects for provider '{provider}' under the `{boundary}` boundary \
                 expresses '{found}'; a fragment is neither called narrower nor clamped, the \
                 typed class must match it exactly — refused (design D5.3)"
            )))
        }
        None => {
            return Err(CompileError::Invalid(format!(
                "seat '{what}' link {link} requests 'tools.sandbox' '{class}', but the {part} \
                 the engine selects for provider '{provider}' under the `{boundary}` boundary \
                 names no `--sandbox` class at all; a fragment that expresses nothing is not a \
                 representation — refused (design D5.3)"
            )))
        }
    }
    Ok(())
}
