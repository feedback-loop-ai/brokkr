//! The engine's complete MCP server set at the final check of a real
//! compiled launch (decision 0065 slice two, U7a; MB1, SI2). The parent's
//! check reads a sealed broker set from the serving inputs, as the serving
//! edge will once a driver carries one (U7d); until then the engine's own
//! composition carries the box's hands alone, so a broker set sealed beside
//! it refuses before any spawn, and no grant is enabled here.

use brokkr_protocol::hands::Sealed;
use brokkr_protocol::native_controls::{SealedBroker, SealedServers, SealedServing};

use super::*;

/// The reach the launches here are composed with and the driver binds where
/// the input carries none: the one that seals nothing (U8a2c).
pub(super) static REACH: std::sync::LazyLock<Sealed> = std::sync::LazyLock::new(Sealed::default);

/// The empty set's strict configuration a Claude launch gains where its
/// shape is admitted, as its serving builds it: where `redrive`'s
/// synthetic facts measure the shape on this host, on either host (U9b1).
pub(super) const EMPTY_SET: [&str; 3] = [
    "--strict-mcp-config",
    "--mcp-config",
    r#"{"mcpServers":{}}"#,
];

/// The brokers `carried` seals beside the box's hands, none for any other
/// server set.
pub(super) fn sealed_brokers(carried: &SealedServing) -> &[SealedBroker] {
    match &carried.isolation.servers {
        SealedServers::Brokered(brokers) => brokers,
        SealedServers::Empty | SealedServers::Hands | SealedServers::NoModelSurface => &[],
    }
}

/// A boxed Claude seat and its Codex fallback, compiled with no grant: the
/// hands set they were sealed with checks as composed, and the same launch
/// sealed with two brokers beside the hands refuses, Claude with the
/// final-set cause and Codex, whose measured fragment carries `brokkr`
/// alone, as hands that are not the engine's.
#[test]
fn a_compiled_launch_is_checked_against_its_complete_sealed_server_set() {
    let operator = Operator::new();
    let ungranted = operator.context(json!({}));
    let bundle = (operator.compile(&ungranted, Boundary::Namespace, None, None)).unwrap();
    let brokers: Vec<SealedBroker> = ["library-docs", "issue-tracker"]
        .map(|capability| SealedBroker {
            capability: capability.into(),
            plan: format!("/state/plans/{capability}.json"),
            plan_digest: "a".repeat(64),
            removals: Vec::new(),
        })
        .to_vec();
    let final_form = |problem: &str| {
        format!(
            "the final command of harness 'codex' {problem}; a complete command is parsed back \
             before its spawn and must express exactly the capability state its sealed plan \
             records, so it is refused rather than spawned (operator ruling 2 of 2026-09-23; \
             design D6)"
        )
    };
    for (candidate, refusal) in [
        (
            0,
            "provider 'claude' final MCP configuration is not the engine's sealed server set"
                .to_string(),
        ),
        (
            1,
            final_form("is sealed with hands that are not the engine's workspace hands"),
        ),
    ] {
        let facts = &bundle.sites["chain"];
        let (spawn, _) = sealed(&bundle, "chain", candidate);
        let mut carried = brokkr_runtime::engine::serving_inputs(
            facts.chain.get(candidate),
            Some(facts),
            spawn.class,
            bundle.boundary,
        )
        .unwrap();
        assert_eq!(carried.isolation.servers, SealedServers::Hands);
        assert_eq!(
            checked_against(&bundle, "chain", candidate, &carried),
            sealed_launch(&bundle, "chain", candidate),
            "{candidate}"
        );
        carried.isolation.servers = SealedServers::Brokered(brokers.clone());
        assert_eq!(
            checked_against(&bundle, "chain", candidate, &carried),
            Err(refusal),
            "{candidate}"
        );
    }
}

/// [`checked_launch`] against the serving inputs `carried`, however they
/// were sealed; moved here from the parent by U8a.
pub(super) fn checked_against(
    bundle: &Bundle,
    label: &str,
    candidate: usize,
    carried: &brokkr_protocol::native_controls::SealedServing,
) -> Result<Vec<String>, String> {
    use brokkr_protocol::native_controls::{
        check_final, managed, Checked, Dialect, LaunchRecord, Origin, Serving, Transport,
    };
    let facts = &bundle.sites[label];
    let (spawn, _) = sealed(bundle, label, candidate);
    let command = sealed_launch(bundle, label, candidate)?;
    let outcome = &facts.capabilities.as_ref().unwrap().outcomes[candidate];
    let harness = outcome.provider.as_str();
    let controls = managed(&json!({"native_controls": outcome.controls()}))?.unwrap();
    let record = LaunchRecord::decode(Some(&spawn.launch_record()))?;
    let authored: Vec<String> = record
        .segments
        .iter()
        .filter(|segment| segment.origin == Origin::Authored)
        .flat_map(|segment| segment.argv.iter().cloned())
        .collect();
    let brokkr = std::env::current_exe().unwrap();
    // A Codex launch is handed the store its profile denies, and its
    // isolation built the measured options (U9c2).
    let codex = harness == "codex";
    // A Claude launch whose admitted shape serves the empty set gains its
    // strict configuration ([`EMPTY_SET`]).
    let strict = harness == "claude"
        && carried.isolation.servers == SealedServers::Empty
        && command.ends_with(&EMPTY_SET.map(String::from));
    let isolating: Vec<String> = match (codex, strict) {
        (true, _) => ISOLATING.map(String::from).to_vec(),
        (false, true) => EMPTY_SET.map(String::from).to_vec(),
        (false, false) => Vec::new(),
    };
    check_final(
        harness,
        command,
        &controls,
        &record.expected,
        Dialect {
            permissions: carried.dialect.permissions.as_ref(),
            sandbox: &carried.dialect.sandbox,
            hands: &carried.dialect.hands,
            boundary: &carried.dialect.boundary,
            stands: carried.dialect.stands,
        },
        Serving {
            program: harness,
            workdir: "/w",
            store: codex.then_some(STORE),
            authored: &authored,
            pins: &carried.pins,
            mcp: &isolating,
            hands: carried.spec.as_ref().map(|spec| Transport {
                brokkr: &brokkr,
                workdir: Path::new("/w"),
                artifacts: Path::new("/a"),
                sealed: &REACH,
                spec,
            }),
            brokers: sealed_brokers(carried),
            ..Default::default()
        },
    )
    .map(Checked::into_argv)
    .map_err(|refusal| refusal.cause)
}
