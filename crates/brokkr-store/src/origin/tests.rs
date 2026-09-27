use super::*;
use crate::tests::env_guard::EnvGuard;
use crate::Store;
use serde_json::json;

/// What a machine is allowed to be identified by, and what is not an
/// identity at all. The fingerprint is opaque and stable: the same
/// machine twice is the same token, a different one is not, and neither
/// the hostname nor the home path it was made from can be read back out
/// of it.
#[test]
fn a_machine_fingerprint_needs_a_source_that_says_something() {
    let dir = tempfile::tempdir().unwrap();
    let named = dir.path().join("machine-id");
    let blank = dir.path().join("blank");
    let missing = dir.path().join("absent");
    std::fs::write(&named, "d9b1e0c4f1a24e0e8b3c\n").unwrap();
    std::fs::write(&blank, "  \n").unwrap();
    let path = |p: &std::path::Path| p.to_str().unwrap().to_string();

    // The first source that can be read wins, and a missing one is
    // simply skipped.
    let first = host_from(&[&path(&missing), &path(&named)], || None).unwrap();
    assert_eq!(first.len(), 16);
    assert_eq!(host_from(&[&path(&named)], || None), Some(first.clone()));
    assert!(
        !first.contains("d9b1e0c4"),
        "the source is not readable back"
    );

    // No source at all falls back to what the caller was given, and a
    // caller with nothing to give gets nothing.
    assert_eq!(
        host_from(&[&path(&missing)], || Some("a-hostname".into())),
        host_from(&[], || Some("a-hostname".into()))
    );
    assert_ne!(host_from(&[], || Some("a-hostname".into())), Some(first));
    assert_eq!(host_from(&[&path(&missing)], || None), None);

    // A source that reads as whitespace says nothing, and saying
    // nothing is not an identity.
    assert_eq!(host_from(&[&path(&blank)], || None), None);
    assert_eq!(host_from(&[], || Some(String::new())), None);

    // And the real one exists, on the machine running this test — on
    // every released platform, not only the one with /etc/machine-id.
    assert!(local_host().is_some());
}

/// Where no identity file exists (macOS, Windows), the machine's name
/// comes from the environment the platform actually exports, and as a
/// last resort from `hostname` itself. Blank answers are no answer.
#[test]
fn a_machine_without_an_identity_file_still_names_itself() {
    let mut env = EnvGuard::lock();
    let variable = "BROKKR_TEST_MACHINE_NAME_7f3c";
    env.remove(variable);
    // Nothing set: the command is asked, and asked once.
    let mut asked = 0;
    let answered = machine_name(&[variable], || {
        asked += 1;
        Some("bench-host".to_string())
    });
    assert_eq!(answered.as_deref(), Some("bench-host"));
    assert_eq!(asked, 1);
    // A blank command answer is none.
    assert_eq!(machine_name(&[variable], || Some("  \n".to_string())), None);
    assert_eq!(machine_name(&[variable], || None), None);
    // A set variable wins without asking.
    env.set(variable, "exported-name");
    assert_eq!(
        machine_name(&[variable], || panic!("the command must not be asked")).as_deref(),
        Some("exported-name")
    );
    // A blank variable does not win.
    env.set(variable, "   ");
    assert_eq!(
        machine_name(&[variable], || Some("fallback".to_string())).as_deref(),
        Some("fallback")
    );

    // The real command answers on the machine running this test; a
    // missing program and a failing one are both no answer.
    let printed = hostname_command().expect("hostname prints on every released platform");
    assert!(!printed.trim().is_empty());
    // The fallback `local_host` hands `host_from`: `hostname` when neither
    // exported variable names the machine, else the variable.
    env.remove("HOSTNAME");
    env.remove("COMPUTERNAME");
    assert_eq!(machine_fallback(), Some(printed));
    env.set("HOSTNAME", "exported-host");
    assert_eq!(machine_fallback().as_deref(), Some("exported-host"));
    assert_eq!(hostname_from("brokkr-no-such-program-7f3c"), None);
    assert_eq!(hostname_from("false"), None);

    // And the home half follows the platform's spelling: the first set
    // variable wins, and none set is empty rather than a panic.
    let home_variable = "BROKKR_TEST_HOME_7f3c";
    env.remove(home_variable);
    assert_eq!(home_from(&[home_variable]), "");
    env.set(home_variable, "/somewhere");
    assert_eq!(
        home_from(&["BROKKR_TEST_UNSET_7f3c", home_variable]),
        "/somewhere"
    );
    let _ = account_home();
}

/// A machine whose identity file answers is never asked by any other
/// means: the fallback, which may spawn `hostname`, is not even run.
#[test]
fn the_fallback_is_asked_only_when_no_source_answers() {
    let dir = tempfile::tempdir().unwrap();
    let named = dir.path().join("machine-id");
    std::fs::write(&named, "d9b1e0c4f1a24e0e8b3c\n").unwrap();
    let named = named.to_str().unwrap();
    let answered = host_from(&[named], || panic!("the fallback must not be asked"));
    assert_eq!(answered, host_from(&[named], || None));
}

/// The process asks once: the first answer is kept, and every later
/// caller is handed that one rather than asking the machine again.
#[test]
fn the_fingerprint_is_asked_once_per_process() {
    let answer = local_host();
    assert_eq!(LOCAL.get(), Some(&answer));
    assert_eq!(local_host(), answer);
}

/// #354's acceptance: no process is spawned while a write transaction is
/// open. The spawner is injected where `hostname` would run, and each
/// time it runs it asks for the journal's write lock with no patience at
/// all, so a lock held by the create would answer busy at once.
#[test]
fn no_process_is_spawned_while_the_journal_write_lock_is_held() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let mut store = Store::open(&path).unwrap();
    let mut probes = Vec::new();
    let spawner = || {
        let peer = rusqlite::Connection::open(&path).unwrap();
        let lock = peer.execute_batch("BEGIN IMMEDIATE; ROLLBACK");
        probes.push(lock.map_err(|error| error.sqlite_error_code()));
        Some("a-hostname".to_string())
    };
    store
        .create_run_from("r1", "feature", "bundle", &json!({}), || {
            host_from(&[], || machine_name(&[], spawner))
        })
        .unwrap();
    assert_eq!(probes, [Ok(())], "spawned once, with the lock free");
    let recorded: Option<String> = store
        .conn
        .query_row("SELECT origin_host FROM runs", [], |row| row.get(0))
        .unwrap();
    assert_eq!(recorded, host_from(&[], || Some("a-hostname".into())));
}
