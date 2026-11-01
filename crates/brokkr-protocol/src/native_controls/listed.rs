//! The composer's Claude and LaneTally arm: the plan's tool lists folded
//! into the seat's argv, or refused (design D6; decision 0066). Moved here
//! from `native_controls.rs`, which sits at its line baseline, when the
//! composer was split so that the final check's recomposition could hand
//! it the paths a permission profile is bound to (decision 0065 slice two,
//! U9c1, R1 of its second review).

use super::grammar::{self, Command, ListKind, Node};
use super::{
    conflicting, denies, distinct, final_tools, fold_lists, parse_origin, unanswered, unconsumed,
    unconsumed_naming, Controls, Failure, Folding, Limit, LimitOrigin, ListFlag, Piece, Place,
    Rewrite, Sources, Toolset,
};

/// What a Claude or LaneTally composition is composed for: the provider,
/// its plan, where the authored part ends in the seat's argv, `split`, and
/// how many arguments after it the plan types as the box's hands.
#[derive(Clone, Copy)]
pub(super) struct Listing<'a> {
    pub(super) provider: &'a str,
    pub(super) controls: &'a Controls,
    pub(super) split: usize,
    pub(super) typed_hands: usize,
}

impl Listing<'_> {
    /// Which contribution carries a list is where the engine placed it
    /// (unit 12-fix-c), never what the list names: the part before the
    /// fragment is the template's and the local permissions', the typed
    /// hands follow, and the rest is the managed boundary fragment.
    fn origin(&self, node: &Node) -> LimitOrigin {
        match node.at {
            at if at < self.split => LimitOrigin::Template,
            at if at < self.split + self.typed_hands => LimitOrigin::Hands,
            _ => LimitOrigin::Managed,
        }
    }
}

/// The seat's argv `extra` with the plan's tool lists folded in, for a
/// harness that reads them ([`super::compose_or_exclude`]'s Claude and
/// LaneTally arm).
pub(super) fn listed(listing: Listing<'_>, mut extra: Vec<String>) -> Result<Vec<String>, Failure> {
    let Listing {
        provider, controls, ..
    } = listing;
    // The plan's own argv is parsed under the same grammar. A list
    // it names is an EXPLICIT control on that list — including an
    // explicitly empty one; anything else is a restriction
    // transport, which is appended as written.
    let plan = parse_origin(provider, &controls.argv, false)?
        .expect("claude and lanetally have a grammar");
    // The two origins together, so a duplicate ACROSS them — a
    // seat's own `--tools` beside the boundary fragment's — is the
    // same refusal a duplicate within one origin is.
    let seat = parse_origin(provider, &extra, true)?.expect("claude and lanetally have a grammar");
    let verbatim: Vec<String> = plan
        .nodes
        .iter()
        .filter(|node| node.list().is_none())
        .flat_map(|node| controls.argv[node.at..node.at + node.tokens].to_vec())
        .collect();
    answered(provider, controls)?;
    let own = seat.lists(ListKind::Include).next();
    let carried: Vec<String> = seat
        .lists(ListKind::Allow)
        .next()
        .map(grammar::node_patterns)
        .unwrap_or_default()
        .into_iter()
        .map(str::to_string)
        .collect();
    let tools = resolved(listing, &seat, &plan, &carried)?;
    // The include list is written only where it is not already the
    // seat's own, spelled as it stands; the allow list gains what
    // the seat does not carry.
    let include = tools.include.as_ref().filter(|names| {
        own.is_none_or(|node| {
            !grammar::node_patterns(node)
                .into_iter()
                .eq(names.iter().map(String::as_str))
        })
    });
    let gained: Vec<String> = tools
        .allow
        .iter()
        .filter(|tool| !carried.contains(tool))
        .cloned()
        .collect();
    let deny: Vec<String> = controls
        .selection
        .deny
        .iter()
        .cloned()
        .chain(named(&plan, ListKind::Deny))
        .collect();
    let flags = controls.selection.flags.as_ref();
    let folding = foldings(&seat, &plan, gained, deny);
    mapped(provider, &plan, flags, include.is_some(), &folding)?;
    available(provider, controls, &tools, &folding)?;
    // Only a judged composition returns: the seat's lists stand
    // where no mapping writes any, and are folded where one does.
    if let Some(flags) = flags {
        extra = fold_lists(
            &extra,
            include.map(|names| Rewrite {
                own,
                flag: &flags[0],
                names,
            }),
            &folding,
            [&flags[1], &flags[2]],
        );
    }
    extra.extend(verbatim);
    Ok(extra)
}

/// Every pattern the plan's own lists of `kind` name.
fn named(plan: &Command, kind: ListKind) -> Vec<String> {
    plan.lists(kind)
        .flat_map(grammar::node_patterns)
        .map(str::to_string)
        .collect()
}

/// The final tool lists [`final_tools`] resolves for the seat, whose own
/// allow list carries `carried`. The typed hands' include list is the base
/// the holdings fill. Every other explicit include list an engine
/// contribution carries is a limit (design D6): the template's own, each
/// one the plan's argv names, and the managed fragment's, whatever its
/// allow list names (unit 12-fix-b, R2). The seat's argv holds one include
/// and one allow list at most: a duplicate across its origins refused
/// before.
fn resolved(
    listing: Listing<'_>,
    seat: &Command,
    plan: &Command,
    carried: &[String],
) -> Result<Toolset, Failure> {
    let Listing {
        provider,
        controls,
        typed_hands,
        ..
    } = listing;
    // W is the site's typed hands' own tool, and nothing without
    // them; T is what the engine lowered from its typed allow.
    let mut hands_tools: Vec<String> = match typed_hands {
        0 => Vec::new(),
        _ => vec![format!(
            "mcp__{}__{}",
            crate::hands::SERVER_NAME,
            crate::hands::TOOL_NAME
        )],
    };
    // W also admits each broker the hands contribution's own MCP
    // document serves, by its served name (U9b1d; ruling R4). This is
    // the one reading of a spelling D6 allows, and only of that
    // document: the final check before spawn binds it to the sealed
    // set (`final_set`) and binds the allow list to the same set's
    // brokers (`allowed`), so no unsealed server is ever admitted.
    hands_tools.extend(
        (seat.nodes.iter())
            .filter(|node| listing.origin(node) == LimitOrigin::Hands)
            .filter(|node| node.name() == "--mcp-config")
            .flat_map(|node| &node.values)
            .flat_map(|document| super::mcp::brokered(document)),
    );
    let limits: Vec<Limit> = seat
        .lists(ListKind::Include)
        .map(|node| (listing.origin(node), node))
        .filter(|(origin, _)| *origin != LimitOrigin::Hands)
        .chain(
            plan.lists(ListKind::Include)
                .map(|node| (LimitOrigin::Plan, node)),
        )
        .map(|(origin, node)| Limit {
            origin,
            flag: node.name().to_string(),
            names: distinct(grammar::node_patterns(node)),
        })
        .collect();
    // The box's hands stand where the plan types them, and are never
    // inferred from an include list (rebuild unit 13-fix-c, R1).
    let hands = typed_hands > 0;
    // A managed allow list admits as much as the selection's does.
    let allow: Vec<String> = controls
        .selection
        .allow
        .iter()
        .cloned()
        .chain(named(plan, ListKind::Allow))
        .collect();
    let carrier = seat
        .lists(ListKind::Allow)
        .next()
        .map_or((LimitOrigin::Template, ""), |node| {
            (listing.origin(node), node.name())
        });
    final_tools(
        &controls.admits,
        Sources {
            include: &controls.selection.include,
            allow: &allow,
            carried,
            hands: &hands_tools,
            local: &controls.provenance.local,
        },
        &limits,
        hands,
    )
    .map_err(|conflict| conflicting(provider, &limits, carrier, conflict))
}

/// What the plan holds and what it says each holding admits answer for
/// each other exactly: an admission is never inferred for a holding, and
/// never carried for a capability the plan denies.
fn answered(provider: &str, controls: &Controls) -> Result<(), Failure> {
    if let Some(capability) = controls
        .held
        .iter()
        .find(|name| !controls.admits.contains_key(*name))
    {
        return Err(unanswered(
            provider,
            vec![
                Piece::Words("holds "),
                Piece::Capability(capability),
                Piece::Words(" but admits no tool for it"),
            ],
        )
        .into());
    }
    if let Some(capability) = controls
        .admits
        .keys()
        .find(|name| !controls.held.contains(name))
    {
        return Err(unanswered(
            provider,
            vec![
                Piece::Words("admits tools for "),
                Piece::Capability(capability),
                Piece::Words(", which it does not hold"),
            ],
        )
        .into());
    }
    Ok(())
}

/// The allow and deny lists as the composition resolves them against the
/// seat's own: what its node for each already carries, and what the plan
/// adds of `gained` and `deny`.
fn foldings(
    seat: &Command,
    plan: &Command,
    gained: Vec<String>,
    deny: Vec<String>,
) -> Vec<Folding> {
    let mut folding: Vec<Folding> = Vec::new();
    for (kind, names) in [(ListKind::Allow, gained), (ListKind::Deny, deny)] {
        let node = seat.lists(kind).next();
        let carried: Vec<String> = node
            .map(grammar::node_patterns)
            .unwrap_or_default()
            .into_iter()
            .map(str::to_string)
            .collect();
        let names: Vec<String> = distinct(names)
            .into_iter()
            .filter(|tool| !carried.contains(tool))
            .collect();
        folding.push(Folding {
            at: node.map(Place::of),
            create: !names.is_empty() || plan.lists(kind).next().is_some(),
            carried,
            names,
        });
    }
    folding
}

/// Where the plan's lists can be written (decision 0066 ruling 3; rebuild
/// unit 12-fix-e): onto the adapter's selection mapping `flags`, each
/// flag one the harness's own grammar reads as that list, whether the
/// composition writes an include list, `include`, or any of `folding`.
fn mapped(
    provider: &str,
    plan: &Command,
    flags: Option<&[ListFlag; 3]>,
    include: bool,
    folding: &[Folding],
) -> Result<(), Failure> {
    match flags {
        // A plan that carries a list for a provider whose adapter
        // maps none cannot be folded anywhere, and is refused rather
        // than dropped (decision 0066 ruling 3); so is a final list
        // that differs from the seat's own. Otherwise the seat's
        // lists stand.
        None => {
            if let Some(node) = plan.nodes.iter().find(|node| node.list().is_some()) {
                return Err(unconsumed_naming(
                    provider,
                    vec![
                        Piece::Words("a managed "),
                        Piece::Flag(node.name()),
                        Piece::Words(" with no selection mapping to fold it into,"),
                    ],
                )
                .into());
            }
            if include || folding.iter().any(|list| list.create) {
                return Err(unconsumed(
                    provider,
                    "a final tool list with no selection mapping to write it into,",
                )
                .into());
            }
        }
        // The adapter's mapping must name a flag the harness's own
        // grammar reads as THIS list: a mapping onto anything else
        // cannot reach the final command, and is refused rather than
        // folded into whatever the name happens to be.
        Some(flags) => {
            let writes = [include, folding[0].create, folding[1].create];
            for (slot, kind) in [ListKind::Include, ListKind::Allow, ListKind::Deny]
                .into_iter()
                .enumerate()
            {
                if writes[slot] && grammar::list_of(provider, &flags[slot].flag) != Some(kind) {
                    // The mapping is the plan's, so its flag is an
                    // identity (rebuild unit 12-fix-e).
                    return Err(unconsumed_naming(
                        provider,
                        vec![
                            Piece::Words("a selection mapped onto "),
                            Piece::Flag(&flags[slot].flag),
                            Piece::Words(", which its grammar does not read as that tool list,"),
                        ],
                    )
                    .into());
                }
            }
        }
    }
    Ok(())
}

/// No tool the composition admits is also denied: `tools` as
/// [`final_tools`] resolved them beside what `folding` allows and denies.
fn available(
    provider: &str,
    controls: &Controls,
    tools: &Toolset,
    folding: &[Folding],
) -> Result<(), Failure> {
    // Availability is judged on every path that composes (rebuild
    // unit 13-fix-d, R3), with a selection mapping or without one:
    // every held tool stays available, whether or not an emitted
    // list names it (rebuild unit 13-fix-c, R3), and so does the
    // hands tool, which the allow list always carries (R1).
    let admitted: Vec<&String> = tools
        .include
        .as_ref()
        .unwrap_or(&controls.selection.include)
        .iter()
        .chain(folding[0].all())
        .chain(controls.admits.values().flatten())
        .collect();
    // Named by its tool alone, through the one bounded renderer
    // (rebuild unit 12-fix-d): the pattern's payload is never said.
    // A bare denial denies every pattern of its tool, as the final
    // check reads it (rebuild unit 13-fix, F6).
    if let Some(tool) = folding[1].all().into_iter().find(|denial| {
        admitted
            .iter()
            .any(|admission| admission == denial || denies(denial, grammar::tool_name(admission)))
    }) {
        return Err(unconsumed_naming(
            provider,
            vec![Piece::Tool(tool), Piece::Words(" both admitted and denied")],
        )
        .into());
    }
    Ok(())
}
