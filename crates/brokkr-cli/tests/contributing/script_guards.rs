//! Structured-command tests for the push-to-main, Callgrind PR-path, and
//! `pr-caches` workflow guards (#1417, decision 0071 rulings 3, 5 and 9).
//!
//! Proves that inert shell comments and metadata scalars never trip script
//! guards, while actual command invocations and cache mismatches still fail.

use super::{
    callgrind_on_the_pull_request_path, pr_caches_unbound, push_forbidden_commands, workspace,
};

#[test]
fn push_guard_ignores_inert_comments_and_catches_real_commands() {
    let ci = std::fs::read_to_string(workspace().join(".github/workflows/ci.yml")).unwrap();

    // An inert comment naming `cargo test` in a push-running step gives no problem.
    let with_comment = ci.replacen(
        "      - name: formatting is canonical\n        run: cargo fmt --all -- --check\n",
        "      - name: formatting is canonical\n        run: |\n          # cargo test is run in a separate job\n          cargo fmt --all -- --check\n",
        1,
    );
    assert_ne!(with_comment, ci);
    assert_eq!(
        push_forbidden_commands(&with_comment),
        Vec::<(String, String)>::new()
    );

    // An actual `cargo test` without `--no-run` in a push-running step is caught.
    let with_real_test = ci.replacen(
        "      - name: formatting is canonical\n        run: cargo fmt --all -- --check\n",
        "      - name: formatting is canonical\n        run: |\n          cargo fmt --all -- --check\n          cargo test -p brokkr-core\n",
        1,
    );
    assert_ne!(with_real_test, ci);
    assert_eq!(
        push_forbidden_commands(&with_real_test),
        vec![(
            "quality".to_string(),
            "cargo test -p brokkr-core".to_string()
        )]
    );

    // Removing `--no-run` from a `pr-caches` `case` arm is caught.
    let cache_runs_tests = ci.replacen(
        "packaging) cargo test --locked -p brokkr-cli --no-run ;;",
        "packaging) cargo test --locked -p brokkr-cli ;;",
        1,
    );
    assert_ne!(cache_runs_tests, ci);
    assert_eq!(
        push_forbidden_commands(&cache_runs_tests),
        vec![(
            "pr-caches".to_string(),
            "cargo test --locked -p brokkr-cli".to_string()
        )]
    );
}

#[test]
fn callgrind_guard_ignores_comments_and_metadata_and_catches_real_invocations() {
    let ci = std::fs::read_to_string(workspace().join(".github/workflows/ci.yml")).unwrap();

    // Shell comments, step names, and env values mentioning valgrind/gungraun give no problem.
    let inert = ci.replacen(
        "      - name: formatting is canonical\n        run: cargo fmt --all -- --check\n",
        "      - name: formatting is canonical (valgrind runs nightly)\n        env:\n          NOTE: gungraun\n        run: |\n          # valgrind is run on the schedule\n          cargo fmt --all -- --check\n",
        1,
    );
    assert_ne!(inert, ci);
    assert_eq!(
        callgrind_on_the_pull_request_path(&[("ci.yml".to_string(), inert)]),
        Vec::<(String, String)>::new()
    );

    // Real valgrind installs, callgrind-limits flags, and cargo bench are caught.
    for (planted, expected) in [
        (
            "      - run: sudo apt-get install -y valgrind\n",
            "sudo apt-get install -y valgrind",
        ),
        (
            "      - run: cargo test -- --callgrind-limits=100\n",
            "cargo test -- --callgrind-limits=100",
        ),
    ] {
        let mut mutated = ci.clone();
        mutated.push_str(planted);
        assert_eq!(
            callgrind_on_the_pull_request_path(&[("ci.yml".to_string(), mutated)]),
            vec![("ci.yml".to_string(), expected.to_string())]
        );
    }
}

#[test]
fn pr_caches_guard_ignores_commented_commands() {
    let root = workspace();
    let ci = std::fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    let shards = std::fs::read_to_string(root.join("scripts/test-shards.sh")).unwrap();

    // A comment in the warmed job cannot satisfy a mismatched `pr-caches` build arm.
    let fake_match = ci
        .replacen(
            "bootstrap-budgets-macos) cargo build --release --locked -p brokkr-cli ;;",
            "bootstrap-budgets-macos) cargo build --release --locked -p brokkr-core ;;",
            1,
        )
        .replacen(
            "      - run: cargo build --release --locked -p brokkr-cli\n      - name: install and first-run budgets hold\n",
            "      - run: |\n          # cargo build --release --locked -p brokkr-core\n          cargo build --release --locked -p brokkr-cli\n      - name: install and first-run budgets hold\n",
            1,
        );
    assert_ne!(fake_match, ci);
    assert_eq!(
        pr_caches_unbound(&fake_match, &shards),
        vec!["pr-caches' build for bootstrap-budgets-macos is not what bootstrap-budgets-macos builds".to_string()]
    );
}
