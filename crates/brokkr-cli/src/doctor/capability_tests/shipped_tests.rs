//! The shipped repository's own realm, read as doctor reads it: what it
//! grants (the researcher's read-only web, #1329) and every native power
//! it denies, moved whole from capability_tests.rs, which stands at its
//! 2,000-line ceiling.

use super::*;

/// The shipped repository, read as doctor reads it: the realm grants
/// Claude's and DSH's read tools to the `researcher` office alone (the
/// operator's read-only web ruling of 2026-11-24, #1329) and nothing else,
/// Codex's search is named with its cold measurement and its resumed gap.
/// DSH's web is measured and LaneTally stays unmeasured.
#[test]
fn the_shipped_realm_grants_the_researcher_read_only_web_and_names_every_native_power_it_denies() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut availability = Availability::unspecified();
    // `exec` is installed wherever a POSIX `sh` is, which the provider
    // probe records like any other binary: it is named on the same terms.
    for provider in ["claude", "codex", "dsh", "exec", "lanetally"] {
        availability.record(provider, Presence::Available);
    }
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    let (world, adapters) = (
        brokkr_runtime::realms::World::inspect(&workspace, None),
        workspace.join("adapters"),
    );
    let given = (&availability, &folder::Inspected::Neither);
    capabilities::report_capabilities(&mut report, &world, &workspace, &adapters, given);
    assert!(report.healthy);
    let rendered = report.render();
    assert!(
        rendered.starts_with(
            "ok       capabilities brokkr 'web-fetch': provider 'claude' through dialect \
             'claude-native-fetch', provider 'dsh' through dialect 'dsh-native-fetch' · offices \
             [researcher] only\n\
             ok       capabilities brokkr 'web-search': provider 'claude' through dialect \
             'claude-native-search', provider 'dsh' through dialect 'dsh-native-search' · offices \
             [researcher] only\n"
        ),
        "{rendered}"
    );
    // Each installed Claude, Codex or DSH seat that holds nothing is one
    // complete plan, admitted with every known power OFF.
    for provider in ["claude", "codex", "dsh"] {
        let plan = admitted("brokkr", provider);
        assert!(rendered.contains(&plan), "{plan}\n---\n{rendered}");
    }
    for expected in [
        "warn     capabilities brokkr native codex 'web-search': NOT granted here: the \
         adapter-level plan above switches it off · evidence: codex-cli 0.154.0, cold `codex \
         exec` only",
        "whether the OFF switch holds on a RESUMED codex session is unmeasured",
        "ok       capabilities brokkr native claude 'web-fetch': granted to offices \
         [researcher] only through dialect 'claude-native-fetch'; the adapter-level plan of a \
         seat of office 'researcher' that wants it is admitted with it ON; for a seat that \
         does not hold it, the adapter-level plan above switches it off",
        "ok       capabilities brokkr native claude 'web-search': granted to offices \
         [researcher] only through dialect 'claude-native-search'; the adapter-level plan of a \
         seat of office 'researcher' that wants it is admitted with it ON; for a seat that \
         does not hold it, the adapter-level plan above switches it off",
        "no live denial or enablement of WebSearch has been measured",
        "ok       capabilities brokkr native dsh 'web-fetch': granted to offices \
         [researcher] only through dialect 'dsh-native-fetch'; the adapter-level plan of a \
         seat of office 'researcher' that wants it is admitted with it ON; for a seat that \
         does not hold it, the adapter-level plan above switches it off",
        "ok       capabilities brokkr native dsh 'web-search': granted to offices \
         [researcher] only through dialect 'dsh-native-search'; the adapter-level plan of a \
         seat of office 'researcher' that wants it is admitted with it ON; for a seat that \
         does not hold it, the adapter-level plan above switches it off",
        "warn     capabilities brokkr native lanetally: native inventory unmeasured: the \
         LaneTally wrapper forwards argv to claude",
    ] {
        assert!(rendered.contains(expected), "{expected}\n---\n{rendered}");
    }
    // Design D6: generic exec dispatch cannot certify an arbitrary child
    // program's native inventory, and the readout says so in full.
    let exec = "warn     capabilities brokkr native exec: native inventory unmeasured: exec runs \
                whatever command the bundle names; the engine cannot certify what an arbitrary \
                child program reaches on its own, so its native inventory is unmeasured. Its \
                hands, boundary and command authority are decisions 0043 and 0046's, unchanged. \
                Nothing is granted through it and no native denial is claimed";
    assert_eq!(
        report
            .lines
            .iter()
            .filter(|line| line.contains(" native exec"))
            .collect::<Vec<_>>(),
        [exec]
    );
}
