//! `brokkr init` scaffolds a bundle that compiles under the
//! constitutional lint; `brokkr doctor` reports health without executing
//! any agent.
//!
//! Every verb below is run FROM INSIDE the scaffold, because that is what
//! the scaffold is: a workspace carrying its own `adapters/`, where the
//! trust tier its gate seats judge on is declared (decision 0021), read
//! from the workspace like every other root (decision 0023).

use std::process::Command;

/// Run `brokkr`. An `init` runs with a `claude` first on PATH: it
/// scaffolds for the first of claude, codex or dsh it finds, and these
/// proofs are about the claude scaffold whatever agent CLIs this host
/// carries. Every other verb sees the host's own PATH.
fn brokkr(args: &[&str], cwd: &std::path::Path) -> (Option<i32>, String, String) {
    let stubs = tempfile::tempdir().unwrap();
    std::fs::write(stubs.path().join("claude"), "").unwrap();
    let mut path = std::env::var_os("PATH").unwrap();
    if args.first() == Some(&"init") {
        path = std::env::join_paths(
            std::iter::once(stubs.path().to_path_buf()).chain(std::env::split_paths(&path)),
        )
        .unwrap();
    }
    let out = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .args(args)
        .env("PATH", path)
        .current_dir(cwd)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn init_scaffolds_a_compiling_bundle_and_refuses_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    let (code, _, stderr) = brokkr(&["init", bundle.to_str().unwrap()], dir.path());
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(stderr.contains("digest"), "stderr: {stderr}");
    // The scaffold says where to stand, once, on stderr.
    assert!(
        stderr.contains("run brokkr from inside"),
        "stderr: {stderr}"
    );

    // The scaffold passes the same compile gate as any bundle.
    let (code, stdout, stderr) = brokkr(&["compile", "--bundle", "."], &bundle);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(stdout.contains("\"starter\""));

    // The tightened ship taxonomy is present from the start.
    let policy = std::fs::read_to_string(bundle.join("policy.json")).unwrap();
    assert!(policy.contains("SHIP-COMPLETE"));
    assert!(policy.contains("SHIPPED-DIRTY"));

    // Decision 0021 ruling 1's roster is declared, not left to default:
    // the three gate seats say so, and the two work seats say so too.
    let scaffolded = std::fs::read_to_string(bundle.join("bundle.json")).unwrap();
    assert_eq!(scaffolded.matches("\"class\": \"gate\"").count(), 3);
    assert_eq!(scaffolded.matches("\"class\": \"work\"").count(), 2);
    // …and the tier they judge on is a file in the operator's tree,
    // theirs to demote, rather than a constant inside this binary.
    let adapter = std::fs::read_to_string(bundle.join("adapters/claude.json")).unwrap();
    assert!(adapter.contains("\"trust_tier\": \"trusted\""), "{adapter}");
    assert!(
        adapter.contains("\"egress\": \"uncontracted\""),
        "{adapter}"
    );

    // The review seat is a gate, and a gate that moves HEAD parks the
    // run: its charter is read-only, as the library's reviewer is.
    let reviewer = std::fs::read_to_string(bundle.join("agents/charters/reviewer.md")).unwrap();
    assert!(
        reviewer.contains("strictly read-only: change no files and make no commits"),
        "{reviewer}"
    );
    assert!(
        reviewer.contains("`clean` with `inputs: {\"fixes_applied\": false}`"),
        "{reviewer}"
    );
    assert!(!reviewer.contains("commit them"), "{reviewer}");

    // Refuses to clobber an existing bundle.
    let (code, _, stderr) = brokkr(&["init", bundle.to_str().unwrap()], dir.path());
    assert_eq!(code, Some(1));
    assert!(stderr.contains("refusing to overwrite"), "stderr: {stderr}");
}

/// `init` under a PATH holding only the named agent CLIs: the scaffold
/// directory, its stderr, and the compile run from inside it.
fn init_with_only(present: &[&str]) -> (tempfile::TempDir, std::path::PathBuf, String) {
    let dir = tempfile::tempdir().unwrap();
    let bins = dir.path().join("bins");
    std::fs::create_dir(&bins).unwrap();
    for binary in present {
        std::fs::write(bins.join(binary), "").unwrap();
    }
    let bundle = dir.path().join("bundle");
    let out = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .args(["init", bundle.to_str().unwrap()])
        .env("PATH", &bins)
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    let (code, stdout, error) = brokkr(&["compile", "--bundle", "."], &bundle);
    assert_eq!(code, Some(0), "{error}");
    assert!(stdout.contains("\"starter\""), "{stdout}");
    (dir, bundle, stderr)
}

fn json_at(path: &std::path::Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// A claude-less host with codex gets a codex scaffold that compiles: the
/// codex adapter and no claude one, every seat hired from codex with
/// hands and no tool allowance, and a realm under `harness`, where
/// codex's own sandbox classes hold those hands — so nothing asks for
/// bubblewrap.
#[test]
fn a_codex_only_host_gets_a_codex_scaffold_that_compiles() {
    let (_dir, bundle, stderr) = init_with_only(&["codex"]);
    assert!(
        stderr
            .contains("Scaffolded for `codex`, the first of claude, codex and dsh found on PATH."),
        "{stderr}"
    );
    assert!(!stderr.contains("hands need bubblewrap"), "{stderr}");
    assert!(bundle.join("adapters/codex.json").is_file());
    assert!(!bundle.join("adapters/claude.json").exists());
    let adapter = json_at(&bundle.join("adapters/codex.json"));
    assert_eq!(adapter["trust_tier"], "trusted");
    assert_eq!(adapter["judges"], serde_json::json!(["astra", "sol"]));
    let map = json_at(&bundle.join("realms.json"));
    assert_eq!(map["schema"], "forge.realms/v4");
    assert_eq!(map["realms"][0]["boundary"], "harness");
    for (agent, models) in [
        ("intake", ["sol", "terra"]),
        ("implementer", ["sol", "terra"]),
        ("reviewer", ["astra", "sol"]),
    ] {
        let definition = json_at(&bundle.join(format!("agents/{agent}.json")));
        assert_eq!(definition["models"], serde_json::json!(models), "{agent}");
        assert_eq!(definition["hands"]["kind"], "workspace", "{agent}");
        assert!(definition.get("tools").is_none(), "{agent}: {definition}");
    }
    // Its README speaks of the adapters it wrote, and of no tool map.
    let readme = std::fs::read_to_string(bundle.join("agents/README.md")).unwrap();
    assert!(!readme.contains("claude.json"), "{readme}");
    assert!(
        readme.contains("## Tool grants\n\nNone. No seat is hired from claude"),
        "{readme}"
    );
}

/// A run writes its journal, results and ledger under `.forge/`, and the
/// ship gate closes out only on a clean tree: `init` ignores `.forge/`
/// from inside, beside the map and in the repository, and keeps an
/// ignore file the operator already has there.
#[test]
fn init_ignores_the_runs_own_forge_directory() {
    let git = |repo: &std::path::Path, args: &[&str]| {
        let out = Command::new("git")
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgSign=false",
            ])
            .args(args)
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    };
    let repo = tempfile::tempdir().unwrap();
    git(repo.path(), &["init", "-q"]);
    let (code, _, stderr) = brokkr(&["init", "."], repo.path());
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(
        std::fs::read_to_string(repo.path().join(".forge/.gitignore")).unwrap(),
        "*\n!.gitignore\n"
    );
    git(repo.path(), &["add", "-A"]);
    git(repo.path(), &["commit", "-q", "-m", "brokkr starter"]);
    // The commit carries the ignore file, so a clone ignores `.forge/` too.
    assert_eq!(
        git(repo.path(), &["ls-files", ".forge"]),
        ".forge/.gitignore\n"
    );
    std::fs::create_dir_all(repo.path().join(".forge/results")).unwrap();
    std::fs::write(repo.path().join(".forge/forge.db"), "").unwrap();
    std::fs::write(repo.path().join(".forge/results/seat.json"), "{}").unwrap();
    assert_eq!(git(repo.path(), &["status", "--porcelain"]), "");

    let repo = tempfile::tempdir().unwrap();
    std::fs::create_dir(repo.path().join(".forge")).unwrap();
    std::fs::write(repo.path().join(".forge/.gitignore"), "theirs\n").unwrap();
    let (code, _, stderr) = brokkr(&["init", "my-bundle"], repo.path());
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(
        std::fs::read_to_string(repo.path().join(".forge/.gitignore")).unwrap(),
        "theirs\n"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("my-bundle/.forge/.gitignore")).unwrap(),
        "*\n!.gitignore\n"
    );
}

/// A dsh-only host: dsh holds intake and implement, and the review gate —
/// which an untrusted adapter with no judges can never hold — stays on
/// claude, and the output says so.
#[test]
fn a_dsh_only_host_names_the_gate_dsh_cannot_hold() {
    let (_dir, bundle, stderr) = init_with_only(&["dsh"]);
    assert!(
        stderr.contains("The review gate cannot be held by dsh: its adapter is untrusted"),
        "{stderr}"
    );
    let readme = std::fs::read_to_string(bundle.join("agents/README.md")).unwrap();
    assert!(
        readme.contains("The review gate cannot be held by dsh"),
        "{readme}"
    );
    for adapter in ["dsh", "claude", "exec"] {
        assert!(bundle.join(format!("adapters/{adapter}.json")).is_file());
    }
    assert!(!bundle.join("adapters/codex.json").exists());
    assert_eq!(
        json_at(&bundle.join("adapters/dsh.json"))["trust_tier"],
        "untrusted"
    );
    for (agent, models) in [
        ("intake", ["flash", "pro"]),
        ("implementer", ["pro", "flash"]),
        ("reviewer", ["fable", "opus"]),
    ] {
        let definition = json_at(&bundle.join(format!("agents/{agent}.json")));
        assert_eq!(definition["models"], serde_json::json!(models), "{agent}");
        assert!(definition.get("hands").is_none(), "{agent}: {definition}");
    }
}

/// On macOS the scaffold's realm declares `harness`, because `namespace`
/// cannot be built there: nothing asks for bubblewrap, and the output
/// says which boundary was written and why.
#[cfg(target_os = "macos")]
#[test]
fn a_macos_scaffold_declares_harness_and_asks_nothing_of_bubblewrap() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    let empty_path = dir.path().join("empty-path");
    std::fs::create_dir(&empty_path).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .args(["init", bundle.to_str().unwrap()])
        .env("PATH", &empty_path)
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert!(!stderr.contains("hands need bubblewrap"), "{stderr}");
    assert!(
        stderr.contains("`realms.json` declares the `harness` boundary"),
        "{stderr}"
    );
    let map = std::fs::read_to_string(bundle.join("realms.json")).unwrap();
    assert!(map.contains("\"boundary\": \"harness\""), "{map}");
}

#[cfg(target_os = "linux")]
#[test]
fn init_names_the_scaffolded_seats_that_need_missing_bubblewrap() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    let empty_path = dir.path().join("empty-path");
    std::fs::create_dir(&empty_path).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .args(["init", bundle.to_str().unwrap()])
        .env("PATH", &empty_path)
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert!(
        stderr.contains(
            "hands need bubblewrap: no `bwrap` on PATH, and the boundary is never simulated"
        ),
        "{stderr}"
    );
    assert!(
        stderr.contains("scaffolded seats [\"ship\", \"verify\"] declare hands"),
        "{stderr}"
    );
    // Decision 0046: the warning speaks the boundary vocabulary instead
    // of saying the shipped gates require Linux.
    assert!(
        stderr.contains("run under the realm's boundary"),
        "{stderr}"
    );
    assert!(
        stderr.contains("`namespace`, the default, needs bubblewrap on PATH"),
        "{stderr}"
    );
    assert!(
        stderr.contains("a realm may declare `harness` instead (decision 0046)"),
        "{stderr}"
    );
    assert!(!stderr.contains("require Linux"), "{stderr}");
}

#[test]
fn doctor_checks_the_map_named_by_realms() {
    let dir = tempfile::tempdir().unwrap();
    let map = dir.path().join("fleet.json");
    std::fs::write(
        &map,
        serde_json::json!({
            "schema": "forge.realms/v3",
            "realms": [{"name": "app", "path": ".", "default_branch": "main",
                        "house": "missing-house.md"}],
            "journal": "forge.db"
        })
        .to_string(),
    )
    .unwrap();
    let (code, stdout, stderr) = brokkr(&["doctor", "--realms", map.to_str().unwrap()], dir.path());
    assert_eq!(code, Some(1), "stderr: {stderr}");
    assert!(stdout.contains("realm 'app' names house"), "{stdout}");
    assert!(stdout.contains("missing-house.md"), "{stdout}");
}

#[test]
fn doctor_prints_scaffolded_dialect_absence_and_missing_tool_warning() {
    let absent = tempfile::tempdir().unwrap();
    let (code, _, stderr) = brokkr(&["init", "."], absent.path());
    assert_eq!(code, Some(0), "{stderr}");
    let (_, stdout, _) = brokkr(&["doctor"], absent.path());
    assert!(
        stdout.contains("ok       dialect starter: none declared"),
        "{stdout}"
    );

    let openspec = tempfile::tempdir().unwrap();
    std::fs::create_dir(openspec.path().join("openspec")).unwrap();
    std::fs::write(
        openspec.path().join("openspec/config.yaml"),
        "schema: spec-driven\n",
    )
    .unwrap();
    let (code, _, stderr) = brokkr(&["init", "."], openspec.path());
    assert_eq!(code, Some(0), "{stderr}");
    let empty_path = openspec.path().join("empty-path");
    std::fs::create_dir(&empty_path).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .arg("doctor")
        .env("PATH", &empty_path)
        .current_dir(openspec.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("warn     dialect starter: openspec"),
        "{stdout}"
    );
    assert!(
        stdout.contains("tool binary 'openspec' not found"),
        "{stdout}"
    );
    assert!(
        stdout.contains("the design route will refuse to run"),
        "{stdout}"
    );
    assert!(
        stdout.contains("dialect starter requires openspec/config.yaml: present"),
        "{stdout}"
    );
}

/// The other half of that: the scaffold WRITES a trust declaration, and
/// a tier is an operator's ruling (decision 0021 ruling 3). `init` guards
/// its bundle against clobbering; the declaration is workspace data and
/// is guarded on the same terms, so scaffolding into a tree that already
/// declares one cannot silently re-promote what the operator demoted.
#[test]
fn init_refuses_to_overwrite_an_operators_trust_declaration() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    std::fs::create_dir_all(bundle.join("adapters")).unwrap();
    let declaration = bundle.join("adapters/claude.json");
    std::fs::write(&declaration, "{\"trust_tier\": \"untrusted\"}\n").unwrap();

    let (code, _, stderr) = brokkr(&["init", bundle.to_str().unwrap()], dir.path());
    assert_eq!(code, Some(1), "stderr: {stderr}");
    assert!(stderr.contains("refusing to overwrite"), "stderr: {stderr}");
    // The demotion is still the operator's, and nothing else was written.
    let kept = std::fs::read_to_string(&declaration).unwrap();
    assert!(kept.contains("untrusted"), "{kept}");
    assert!(!bundle.join("bundle.json").exists());
}

#[test]
fn init_refuses_an_existing_realm_map_and_operator_owned_dialect_data() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("realm-map");
    std::fs::create_dir(&bundle).unwrap();
    let map = bundle.join("realms.json");
    std::fs::write(&map, "operator-owned\n").unwrap();
    let (code, _, stderr) = brokkr(&["init", bundle.to_str().unwrap()], dir.path());
    assert_eq!(code, Some(1), "{stderr}");
    assert!(stderr.contains("realms.json") && stderr.contains("refusing to overwrite"));
    assert_eq!(std::fs::read_to_string(map).unwrap(), "operator-owned\n");

    let openspec = tempfile::tempdir().unwrap();
    std::fs::create_dir(openspec.path().join("openspec")).unwrap();
    std::fs::write(
        openspec.path().join("openspec/config.yaml"),
        "schema: spec-driven\n",
    )
    .unwrap();
    let bundle = openspec.path().join("bundle");
    std::fs::create_dir_all(bundle.join("dialects")).unwrap();
    let dialect = bundle.join("dialects/openspec.json");
    std::fs::write(&dialect, "operator-owned\n").unwrap();
    let (code, _, stderr) = brokkr(&["init", bundle.to_str().unwrap()], openspec.path());
    assert_eq!(code, Some(1), "{stderr}");
    assert!(stderr.contains("dialect data") && stderr.contains("refusing to overwrite"));
    assert_eq!(
        std::fs::read_to_string(dialect).unwrap(),
        "operator-owned\n"
    );
    assert!(!bundle.join("bundle.json").exists());

    std::fs::remove_file(bundle.join("dialects/openspec.json")).unwrap();
    std::fs::create_dir(bundle.join("dialects/openspec")).unwrap();
    let instruction = bundle.join("dialects/openspec/specify.md");
    std::fs::write(&instruction, "operator-owned instruction\n").unwrap();
    let (code, _, stderr) = brokkr(&["init", bundle.to_str().unwrap()], openspec.path());
    assert_eq!(code, Some(1), "{stderr}");
    assert!(stderr.contains("specify.md") && stderr.contains("refusing to overwrite"));
    assert_eq!(
        std::fs::read_to_string(instruction).unwrap(),
        "operator-owned instruction\n"
    );
    assert!(!bundle.join("bundle.json").exists());
}

#[test]
fn init_refuses_to_overwrite_exec_trust_or_deterministic_seat_scripts() {
    let dir = tempfile::tempdir().unwrap();

    let exec_bundle = dir.path().join("exec-bundle");
    std::fs::create_dir_all(exec_bundle.join("adapters")).unwrap();
    let exec = exec_bundle.join("adapters").join("exec.json");
    std::fs::write(&exec, "operator-owned\n").unwrap();
    let (code, _, stderr) = brokkr(&["init", exec_bundle.to_str().unwrap()], dir.path());
    assert_eq!(code, Some(1), "stderr: {stderr}");
    assert!(stderr.contains(&exec.display().to_string()), "{stderr}");
    assert_eq!(std::fs::read_to_string(exec).unwrap(), "operator-owned\n");

    for script in ["verify-seat.sh", "ship-seat.sh"] {
        let bundle = dir.path().join(script);
        std::fs::create_dir_all(bundle.join("scripts")).unwrap();
        let path = bundle.join("scripts").join(script);
        std::fs::write(&path, "operator-owned\n").unwrap();
        let (code, _, stderr) = brokkr(&["init", bundle.to_str().unwrap()], dir.path());
        assert_eq!(code, Some(1), "stderr: {stderr}");
        assert!(stderr.contains(&path.display().to_string()), "{stderr}");
        assert_eq!(std::fs::read_to_string(path).unwrap(), "operator-owned\n");
    }
}

/// Decision 0021, from the operator's side: the scaffold's gate seats
/// stand on a declaration in the operator's own tree, so demoting the
/// tier there refuses the very next compile — naming the seat and the
/// driver — rather than quietly leaving three judges unbacked.
#[test]
fn demoting_the_scaffolded_tier_refuses_the_scaffolded_gates() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    brokkr(&["init", bundle.to_str().unwrap()], dir.path());

    let adapter = bundle.join("adapters/claude.json");
    let declared = std::fs::read_to_string(&adapter).unwrap();
    std::fs::write(
        &adapter,
        declared.replace(
            "\"trust_tier\": \"trusted\"",
            "\"trust_tier\": \"untrusted\"",
        ),
    )
    .unwrap();

    let (code, _, stderr) = brokkr(&["compile", "--bundle", "."], &bundle);
    assert_eq!(code, Some(1), "stderr: {stderr}");
    assert!(
        stderr.contains("gate class") && stderr.contains("claude"),
        "stderr: {stderr}"
    );
}

#[test]
fn doctor_reports_health_and_validates_a_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    brokkr(&["init", bundle.to_str().unwrap()], dir.path());

    let db = dir.path().join("forge.db");
    let (code, stdout, _) = brokkr(
        &["doctor", "--bundle", ".", "--db", db.to_str().unwrap()],
        &bundle,
    );
    // git and python3 exist on dev and CI machines; claude may only warn.
    assert_eq!(code, Some(0), "doctor output: {stdout}");
    assert!(stdout.contains("git"));
    assert!(stdout.contains("bundle"));
    // The pinned contract versions are part of the health report.
    assert!(
        stdout.contains("contracts: engine")
            && stdout.contains("event_schema")
            && stdout.contains("database_schema")
            && stdout.contains("driver_protocol"),
        "doctor output: {stdout}"
    );
    assert!(!stdout.contains("MISSING  git"));

    // A broken bundle turns the report unhealthy.
    std::fs::write(bundle.join("policy.json"), "{}").unwrap();
    let (code, stdout, _) = brokkr(
        &["doctor", "--bundle", ".", "--db", db.to_str().unwrap()],
        &bundle,
    );
    assert_eq!(code, Some(1), "doctor output: {stdout}");
    assert!(stdout.contains("MISSING  bundle"));
}

#[test]
fn doctor_names_every_unpinned_model_seat_and_the_single_repair() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    brokkr(&["init", bundle.to_str().unwrap()], dir.path());
    let bundle_file = bundle.join("bundle.json");
    let mut config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&bundle_file).unwrap()).unwrap();
    for (phase, charter, kind) in [
        ("implement", "implementer", "claude"),
        ("verify", "verifier", "codex"),
    ] {
        let seat = &mut config["seats"][phase];
        seat.as_object_mut().unwrap().remove("agent");
        seat["role"] = serde_json::json!(format!("agents/charters/{charter}.md"));
        seat["driver"] = serde_json::json!({
            "command": ["{brokkr}", "driver", kind, "--"]
        });
    }
    std::fs::write(&bundle_file, serde_json::to_string_pretty(&config).unwrap()).unwrap();
    let db = dir.path().join("forge.db");
    let (code, stdout, _) = brokkr(
        &["doctor", "--bundle", ".", "--db", db.to_str().unwrap()],
        &bundle,
    );
    assert_eq!(code, Some(1), "{stdout}");
    assert!(stdout.contains("'implement'"), "{stdout}");
    assert!(stdout.contains("'verify'"), "{stdout}");
    assert!(stdout.contains("--model <concrete-model-id>"), "{stdout}");
}

/// A directory holding one executable that answers `--version`, put FIRST
/// on the PATH doctor runs under: doctor's "installed" is the provider
/// probe's answer, and the probe asks a binary for its version banner. No
/// model is reached — the stand-in can say nothing else.
#[cfg(unix)]
fn path_with_a_stand_in(root: &std::path::Path, binary: &str) -> std::ffi::OsString {
    use std::os::unix::fs::PermissionsExt;
    let bin = root.join("stand-in-bin");
    std::fs::create_dir_all(&bin).unwrap();
    let program = bin.join(binary);
    std::fs::write(&program, "#!/bin/sh\necho '0.0.0 (stand-in)'\n").unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let rest = std::env::var_os("PATH").unwrap_or_default();
    std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(&rest))).unwrap()
}

#[cfg(unix)]
fn doctor_lines(cwd: &std::path::Path, path: &std::ffi::OsStr) -> Vec<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .arg("doctor")
        .env("PATH", path)
        .current_dir(cwd)
        .output()
        .unwrap();
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

/// The complete native lines doctor owes the scaffolded realm for Claude,
/// built from the evidence the SCAFFOLDED adapter declares: doctor prints
/// adapter data, and these are the data.
#[cfg(unix)]
fn scaffolded_claude_denials(workspace: &std::path::Path) -> Vec<String> {
    let adapter: serde_json::Value =
        serde_json::from_slice(&std::fs::read(workspace.join("adapters/claude.json")).unwrap())
            .unwrap();
    ["web-fetch", "web-search"]
        .iter()
        .map(|key| {
            let evidence = &adapter["native_capabilities"]["known"][key]["evidence"];
            let limitations: Vec<&str> = evidence["limitations"]
                .as_array()
                .unwrap()
                .iter()
                .map(|limitation| limitation.as_str().unwrap())
                .collect();
            format!(
                "warn     capabilities starter native claude '{key}': NOT granted here: every \
                 seat on claude is launched with it switched off by the adapter's declared \
                 control · evidence: {} · still unmeasured: {}",
                evidence["scope"].as_str().unwrap(),
                limitations.join("; ")
            )
        })
        .collect()
}

/// Decision 0065, task 8.2: what `init` scaffolds is proved through
/// doctor. The scaffolded realm grants nothing and says so; with Claude
/// installed, both of its native tools are named as not granted, each
/// beside its evidence scope and the live checks still owed.
#[cfg(unix)]
#[test]
fn doctor_reads_the_scaffold_as_granting_nothing_and_names_claudes_native_tools() {
    let dir = tempfile::tempdir().unwrap();
    let (code, _, stderr) = brokkr(&["init", "."], dir.path());
    assert_eq!(code, Some(0), "{stderr}");
    let path = path_with_a_stand_in(dir.path(), "claude");
    let lines = doctor_lines(dir.path(), &path);
    let expected = scaffolded_claude_denials(dir.path());
    assert!(
        expected[1].contains("no live denial or enablement of WebSearch has been measured")
            && expected[1].contains("are both unmeasured live"),
        "the scaffold carries the evidence limits: {}",
        expected[1]
    );
    let capabilities: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains(" capabilities starter"))
        .filter(|line| !line.contains(" native exec:"))
        .collect();
    assert_eq!(
        capabilities,
        [
            "ok       capabilities starter: grants nothing; every native capability is governed \
             by the no-grant default — switched off, or the seat is refused",
            expected[0].as_str(),
            expected[1].as_str(),
        ],
        "{lines:#?}"
    );
}

/// Scaffold into a fresh canonicalised root carrying one init-stacks
/// fixture's markers (none for `None`), and return the implementer's
/// generated `tools` and doctor's capability lines for the starter realm.
#[cfg(unix)]
fn scaffolded_capabilities(fixture: Option<&str>) -> (serde_json::Value, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(dir.path()).unwrap();
    if let Some(fixture) = fixture {
        let markers = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/init-stacks")
            .join(fixture);
        for entry in std::fs::read_dir(markers).unwrap() {
            let marker = entry.unwrap().path();
            std::fs::copy(&marker, root.join(marker.file_name().unwrap())).unwrap();
        }
    }
    let (code, _, stderr) = brokkr(&["init", "."], &root);
    assert_eq!(code, Some(0), "{fixture:?}: {stderr}");
    let implementer: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("agents/implementer.json")).unwrap())
            .unwrap();
    let path = path_with_a_stand_in(&root, "claude");
    let capabilities = doctor_lines(&root, &path)
        .into_iter()
        .filter(|line| line.contains(" capabilities starter"))
        .filter(|line| !line.contains(" native exec:"))
        .collect();
    (implementer["tools"].clone(), capabilities)
}

/// Decision 0065 rebuild unit 5 (task 5.2): a scaffold whose agents carry
/// generated typed restrictions reads through doctor exactly as the
/// unrestricted scaffold does — the same "grants nothing" line and the same
/// two Claude native lines, each switched off. `npm` and `npx` are the
/// migration names the shipped adapters now map too; the scaffolded work
/// agent lists the one its stack runs and nothing wider.
#[cfg(unix)]
#[test]
fn doctor_reads_a_stacks_typed_restrictions_as_granting_nothing() {
    // The control: no stack, no typed restriction, and the three lines
    // the test above pins whole.
    let (unrestricted, control) = scaffolded_capabilities(None);
    let mut rows = vec![("unrestricted", unrestricted, control.first().cloned())];
    let mut expected = vec![(
        "unrestricted",
        serde_json::Value::Null,
        Some(
            "ok       capabilities starter: grants nothing; every native capability is governed \
             by the no-grant default — switched off, or the seat is refused"
                .to_string(),
        ),
    )];
    let mut lines = Vec::new();
    for (fixture, runner) in [("node-npm", "npm"), ("turbo-plain", "npx")] {
        let (tools, capabilities) = scaffolded_capabilities(Some(fixture));
        rows.push((fixture, tools, None));
        expected.push((
            fixture,
            serde_json::json!({"allow": [runner, "git", "ls", "rg", "mkdir"], "mcp": []}),
            None,
        ));
        lines.push((fixture, capabilities));
    }
    assert_eq!(rows, expected);
    assert_eq!(
        lines,
        [("node-npm", control.clone()), ("turbo-plain", control)]
    );
}

/// Design D8: independent results survive an agent library that does not
/// load. The library's failure is its own line, and the native lines —
/// Claude's denials and generic exec's unmeasured reason — still print.
#[cfg(unix)]
#[test]
fn a_broken_agent_library_takes_no_native_capability_line_with_it() {
    let dir = tempfile::tempdir().unwrap();
    let (code, _, stderr) = brokkr(&["init", "."], dir.path());
    assert_eq!(code, Some(0), "{stderr}");
    std::fs::write(dir.path().join("agents/implementer.json"), "not json").unwrap();
    let path = path_with_a_stand_in(dir.path(), "claude");
    let lines = doctor_lines(dir.path(), &path);
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("warn     agents:")),
        "{lines:#?}"
    );
    // The scaffolded exec adapter declares no assessment of its own, so
    // doctor reads it as every pre-ruling adapter is read: unmeasured,
    // with the absence as the reason — never as an empty inventory.
    let mut expected = scaffolded_claude_denials(dir.path());
    expected.push(
        "warn     capabilities starter native exec: native inventory unmeasured: the adapter \
         declares no native_capabilities assessment. Nothing is granted through it and no \
         native denial is claimed"
            .to_string(),
    );
    for line in &expected {
        assert!(lines.contains(line), "{line}\n---\n{lines:#?}");
    }
}

#[test]
fn doctor_judges_each_boundary_and_compiles_the_discovered_dialect() {
    use serde_json::{json, Value};
    let dir = tempfile::tempdir().unwrap();
    let (code, _, error) = brokkr(&["init", "."], dir.path());
    assert_eq!(code, Some(0), "{error}");
    let empty = dir.path().join("empty-path");
    std::fs::create_dir(&empty).unwrap();
    let map_path = dir.path().join("realms.json");
    let mut map: Value = serde_json::from_slice(&std::fs::read(&map_path).unwrap()).unwrap();
    map["schema"] = json!("forge.realms/v4");
    let doctor = || {
        let output = Command::new(env!("CARGO_BIN_EXE_brokkr"))
            .args(["doctor", "--bundle", "."])
            .env("PATH", &empty)
            .current_dir(dir.path())
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap()
    };
    for (word, expected) in [
        ("namespace", "warn     hands:"),
        ("harness", "ok       hands:"),
        ("open", "ok       hands:"),
        ("seatbelt", "slice (ii)"),
        ("container", "slice (iii)"),
    ] {
        map["realms"][0]["boundary"] = json!(word);
        std::fs::write(&map_path, map.to_string()).unwrap();
        let output = doctor();
        assert!(output.contains(expected), "{word}: {output}");
        assert!(
            output.contains("boundaries: harness · open offered"),
            "{output}"
        );
        assert!(output.contains("ok       bundle:"), "{output}");
    }
    // A declared dialect which cannot load must also fail bundle compilation;
    // merely adding a diagnostic beside a compile with default dialect is wrong.
    map["realms"][0]["boundary"] = json!("namespace");
    map["realms"][0]["dialect"] = json!("missing-dialect.json");
    std::fs::write(&map_path, map.to_string()).unwrap();
    let output = doctor();
    assert!(output.contains("MISSING  bundle:"), "{output}");
    assert!(output.contains("missing-dialect.json"), "{output}");
    assert!(!output.contains("ok       bundle:"), "{output}");
}
