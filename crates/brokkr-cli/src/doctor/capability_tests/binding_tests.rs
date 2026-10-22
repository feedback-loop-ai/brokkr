//! U9a1 (SC5; U2's carried finding): the three states a realm's grant of a
//! capability is read as — no grant read, a grant that failed, a native
//! binding — each print their own line for an installed harness's native
//! power of that name, and a binding to another provider holds nothing.

use super::*;

/// What doctor says of the one realm the map names, `name` granting
/// `grants`, with only Codex installed.
fn realm_lines(name: &str, grants: Option<Value>) -> (bool, Vec<String>) {
    let dir = workspace_with(Some(json!([realm(name, grants)])));
    lines(dir.path(), &installed(&["codex"]))
}

#[test]
fn no_grant_read_is_not_granted_and_the_unheld_plan_switches_it_off() {
    let nothing = "ok       capabilities unread: grants nothing; every native capability is \
                   governed by the no-grant default — switched off, or the seat is refused";
    assert_eq!(
        realm_lines("unread", None),
        (
            true,
            vec![
                nothing.to_string(),
                admitted("unread", "codex"),
                format!(
                    "warn     capabilities unread native codex 'web-search': NOT granted here: \
                     {OFF} · {EVIDENCE}"
                ),
            ]
        )
    );
}

#[test]
fn a_grant_that_failed_is_unknown_and_neither_granted_nor_denied() {
    let grants = json!({"web-search": {"dialect": "absent-search"}});
    let absent = "realm 'failed': capability 'web-search': tool dialect 'absent-search' is not \
                  at 'dialects/tools/absent-search.json' in the operator configuration";
    assert_eq!(
        realm_lines("failed", Some(grants)),
        (
            false,
            vec![
                format!("MISSING  capabilities failed 'web-search': {absent}"),
                refused("failed", "codex", absent),
                format!(
                    "warn     capabilities failed native codex 'web-search': UNKNOWN here: this \
                     realm's grant of 'web-search' did not validate (its failing line is \
                     above), so neither a grant nor a denial is claimed, and no seat compiles in \
                     this realm until it is repaired · {EVIDENCE}"
                ),
            ]
        )
    );
}

#[test]
fn a_native_binding_is_granted_to_its_own_provider_and_to_no_other() {
    let grants = json!({"web-search": {"dialect": "codex-native-search"}});
    assert_eq!(
        realm_lines("bound", Some(grants)),
        (
            true,
            vec![
                "ok       capabilities bound 'web-search': dialect 'codex-native-search' \
                 (provider-native, provider 'codex') · tools [web_search] · all requesting \
                 offices · restrictions none"
                    .to_string(),
                admitted("bound", "codex"),
                format!(
                    "ok       capabilities bound native codex 'web-search': granted to all \
                     requesting offices through dialect 'codex-native-search'; the \
                     adapter-level plan of a seat of office 'adapter-plan' that wants it is \
                     admitted with it ON; for a seat that does not hold it, {OFF} · {EVIDENCE}"
                ),
            ]
        )
    );
    // The same capability bound to another provider's key holds nothing
    // on Codex: its power of that name is not granted here.
    let grants = json!({"web-search": {"dialect": "stuck-search"}});
    let (_, lines) = realm_lines("elsewhere", Some(grants));
    assert_eq!(
        lines.last().map(String::as_str),
        Some(
            format!(
                "warn     capabilities elsewhere native codex 'web-search': NOT granted here: \
                 {OFF} · {EVIDENCE}"
            )
            .as_str()
        )
    );
}
