//! Decision 0046 ruling 2, arm by arm: one probe table, the refusal the
//! three verbs make of it, and the two lines `doctor` renders from it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_core::realms::{Boundary, BOUNDARIES};
use brokkr_runtime::Bundle;
use serde_json::{json, Value};

use super::*;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

const POLICY: &str = r#"{"schema":"forge.phase-machine/v1","phases":["work","review","done","stop"],"initial":"work","terminal":["done","stop"],"rules":[{"id":"W","from":"work","result":"complete","next":"review","reason":"r"},{"id":"OK","from":"review","result":"clean","next":"done","reason":"r"}]}"#;

/// A bundle whose `work` seat is an exec site running the bundle's own
/// `./scripts/gate.sh` — the one shape every boundary admits at compile
/// (decision 0046 ruling 4) — with or without hands.
fn bundle_dir(dir: &Path, hands: Option<Value>) -> PathBuf {
    let bundle = dir.join("bundle");
    std::fs::create_dir_all(bundle.join("roles")).unwrap();
    std::fs::create_dir_all(bundle.join("scripts")).unwrap();
    std::fs::write(bundle.join("roles/role.md"), "# role\n").unwrap();
    std::fs::write(bundle.join("scripts/gate.sh"), "#!/bin/sh\ntrue\n").unwrap();
    std::fs::write(bundle.join("policy.json"), POLICY).unwrap();
    let exec = |results: Value| {
        json!({
            "role": "roles/role.md",
            "results": results,
            "driver": {"command": ["{brokkr}", "driver", "exec", "--", "bash", "./scripts/gate.sh"]},
        })
    };
    let mut work = exec(json!(["complete"]));
    if let Some(hands) = hands {
        work["hands"] = hands;
    }
    let review = exec(json!(["clean"]));
    std::fs::write(
        bundle.join("bundle.json"),
        json!({"name": "boxed", "policy": "policy.json", "seats": {"work": work, "review": review}})
            .to_string(),
    )
    .unwrap();
    bundle
}

fn compile(dir: &Path, hands: Option<Value>, boundary: Boundary) -> Bundle {
    Bundle::compile_under(
        &bundle_dir(dir, hands),
        &workspace().join("agents"),
        &workspace().join("adapters"),
        boundary,
    )
    .unwrap()
}

fn plant(bin: &Path, tool: &str) {
    std::fs::create_dir_all(bin).unwrap();
    std::fs::write(bin.join(tool), "").unwrap();
}

fn refusal(bundle: &Bundle, path: &Path) -> String {
    refuse_unboxable(bundle, path.as_os_str())
        .unwrap_err()
        .to_string()
}

/// The table over an empty search path and over a planted one, and the
/// path lookup that feeds it.
#[test]
fn the_table_probes_each_boundarys_tool_and_offers_the_two_that_need_none() {
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let none = offered(&on_path(empty.as_os_str()));
    assert_eq!(none[&Boundary::Namespace], Offer::MissingTool("bwrap"));
    assert_eq!(
        none[&Boundary::Seatbelt],
        Offer::Unbuilt {
            slice: "ii",
            needs: "sandbox-exec",
            found: None
        }
    );
    assert_eq!(
        none[&Boundary::Container],
        Offer::Unbuilt {
            slice: "iii",
            needs: "docker or podman",
            found: None
        }
    );
    for boundary in [Boundary::Harness, Boundary::Open] {
        assert!(matches!(none[&boundary], Offer::Offered(_)));
    }

    let bin = dir.path().join("bin");
    for tool in ["bwrap", "sandbox-exec", "podman"] {
        plant(&bin, tool);
    }
    let some = offered(&on_path(bin.as_os_str()));
    assert_eq!(
        some[&Boundary::Namespace],
        Offer::Offered(bin.join("bwrap").display().to_string())
    );
    assert_eq!(
        some[&Boundary::Seatbelt],
        Offer::Unbuilt {
            slice: "ii",
            needs: "sandbox-exec",
            found: Some(format!(
                "sandbox-exec {}",
                bin.join("sandbox-exec").display()
            ))
        }
    );
    // `docker` first, `podman` second: with only a podman planted the
    // container arm reports the podman.
    assert_eq!(
        some[&Boundary::Container],
        Offer::Unbuilt {
            slice: "iii",
            needs: "docker or podman",
            found: Some(format!("podman {}", bin.join("podman").display()))
        }
    );
    plant(&bin, "docker");
    let docker = offered(&on_path(bin.as_os_str()));
    assert_eq!(
        docker[&Boundary::Container],
        Offer::Unbuilt {
            slice: "iii",
            needs: "docker or podman",
            found: Some(format!("docker {}", bin.join("docker").display()))
        }
    );
    // A directory of the tool's name is not the tool.
    std::fs::create_dir_all(bin.join("bwrap.d")).unwrap();
    assert!(on_path(bin.as_os_str())("bwrap.d").is_none());
}

/// Rows 5.6 and 5.7: one refusal per boundary on an empty search path,
/// `harness` and `open` passing, `namespace` passing with a planted
/// `bwrap` and no overlay bind and still asking 0.10 of it with one, the
/// two unbuilt boundaries refusing with and without their tool, and a
/// plain bundle passing everywhere.
#[test]
fn every_boundary_is_judged_against_the_search_path_and_the_slice_that_builds_it() {
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let hands = Some(json!("workspace"));

    let namespace = refusal(
        &compile(dir.path(), hands.clone(), Boundary::Namespace),
        &empty,
    );
    assert!(
        namespace.contains("`namespace` boundary needs `bwrap` on PATH"),
        "{namespace}"
    );
    assert!(
        namespace.contains("seats [\"work\"] declare hands"),
        "{namespace}"
    );
    assert!(namespace.contains("never simulated"), "{namespace}");
    assert!(namespace.contains("decision 0046 ruling 2"), "{namespace}");

    let seatbelt = refusal(
        &compile(dir.path(), hands.clone(), Boundary::Seatbelt),
        &empty,
    );
    assert!(
        seatbelt.contains("`seatbelt` boundary is built by slice (ii)"),
        "{seatbelt}"
    );
    assert!(seatbelt.contains("sandbox-exec not on PATH"), "{seatbelt}");
    assert!(seatbelt.contains("seats [\"work\"]"), "{seatbelt}");
    assert!(
        seatbelt.contains("a realm may declare `harness` today"),
        "{seatbelt}"
    );

    let container = refusal(
        &compile(dir.path(), hands.clone(), Boundary::Container),
        &empty,
    );
    assert!(
        container.contains("`container` boundary is built by slice (iii)"),
        "{container}"
    );
    assert!(
        container.contains("docker or podman not on PATH"),
        "{container}"
    );
    assert!(container.contains("seats [\"work\"]"), "{container}");

    for boundary in [Boundary::Harness, Boundary::Open] {
        let bundle = compile(dir.path(), hands.clone(), boundary);
        refuse_unboxable(&bundle, empty.as_os_str()).unwrap();
    }

    // A planted bubblewrap offers `namespace`; an overlay bind then asks
    // it for a version, and an empty file states none.
    let bin = dir.path().join("bin");
    plant(&bin, "bwrap");
    let boxed = compile(dir.path(), hands.clone(), Boundary::Namespace);
    refuse_unboxable(&boxed, bin.as_os_str()).unwrap();
    let overlaid = compile(
        dir.path(),
        Some(json!({"kind": "workspace", "binds": [{"path": "/opt/x", "mode": "overlay"}]})),
        Boundary::Namespace,
    );
    let older = refusal(&overlaid, &bin);
    assert!(older.contains("seat 'work'"), "{older}");
    assert!(older.contains("0.10 or newer"), "{older}");

    // The unbuilt two refuse with their tool found as much as without,
    // and say which they found.
    plant(&bin, "sandbox-exec");
    let found = refusal(
        &compile(dir.path(), hands.clone(), Boundary::Seatbelt),
        &bin,
    );
    assert!(found.contains("slice (ii)"), "{found}");
    assert!(found.contains("sandbox-exec"), "{found}");
    assert!(
        found.contains(&format!("{} found", bin.join("sandbox-exec").display())),
        "{found}"
    );
    for engine in ["docker", "podman"] {
        let only = dir.path().join(format!("only-{engine}"));
        plant(&only, engine);
        let found = refusal(
            &compile(dir.path(), hands.clone(), Boundary::Container),
            &only,
        );
        assert!(found.contains("slice (iii)"), "{found}");
        assert!(
            found.contains(&format!("{engine} {} found", only.join(engine).display())),
            "{found}"
        );
    }

    // A bundle that boxes nothing asks nothing of the machine.
    for boundary in BOUNDARIES {
        let plain = compile(dir.path(), None, boundary);
        refuse_unboxable(&plain, empty.as_os_str()).unwrap();
    }
}

/// `doctor`'s one line, from the same table: a Linux box with bubblewrap
/// and docker and no sandbox-exec, and an empty search path.
#[test]
fn the_doctor_line_names_what_is_offered_and_why_the_rest_is_not() {
    let linux = |tool: &str| match tool {
        "bwrap" => Some("0.11.0".to_string()),
        "docker" => Some("27.0".to_string()),
        _ => None,
    };
    let line = doctor_line(&offered(&linux));
    assert_eq!(
        line,
        "namespace (bubblewrap 0.11.0) · harness · open offered; seatbelt built by slice (ii) \
         of decision 0046 ruling 6 (sandbox-exec not on PATH); container built by slice (iii) \
         of decision 0046 ruling 6 (docker 27.0 found)"
    );
    let bare = doctor_line(&offered(&|_: &str| None));
    assert_eq!(
        bare,
        "harness · open offered; namespace needs bwrap on PATH (not found); seatbelt built by \
         slice (ii) of decision 0046 ruling 6 (sandbox-exec not on PATH); container built by \
         slice (iii) of decision 0046 ruling 6 (docker or podman not on PATH)"
    );
}

/// The `hands` line judged against the realm's boundary: healthy under
/// `namespace` with bubblewrap and under `harness` or `open` always, a
/// warning under `namespace` without bubblewrap and under an unbuilt
/// boundary — with and without seats to name.
#[test]
fn the_hands_line_follows_the_boundary_not_bubblewrap_alone() {
    let with_bwrap = offered(&|tool: &str| (tool == "bwrap").then(|| "0.11.0".to_string()));
    let without: BTreeMap<Boundary, Offer> = offered(&|_: &str| None);
    let seats = ["ship", "verify"];

    let (ok, line) = hands_line(
        Boundary::Namespace,
        &with_bwrap[&Boundary::Namespace],
        &seats,
    );
    assert!(ok);
    assert_eq!(
        line,
        "0.11.0 · seats [\"ship\", \"verify\"] declare hands and can run"
    );
    let (ok, line) = hands_line(Boundary::Namespace, &with_bwrap[&Boundary::Namespace], &[]);
    assert!(ok);
    assert_eq!(line, "0.11.0 · boxed seats can run");
    let (ok, line) = hands_line(Boundary::Namespace, &without[&Boundary::Namespace], &seats);
    assert!(!ok);
    assert_eq!(
        line,
        "bubblewrap (bwrap) not found — seats [\"ship\", \"verify\"] declare hands and will \
         refuse to spawn under `namespace`"
    );
    for boundary in [Boundary::Harness, Boundary::Open] {
        let (ok, line) = hands_line(boundary, &without[&boundary], &seats);
        assert!(ok);
        assert_eq!(
            line,
            format!(
                "seats [\"ship\", \"verify\"] declare hands and can run under `{boundary}` — no \
                 box of Brokkr's is built there"
            )
        );
    }
    let (ok, line) = hands_line(Boundary::Seatbelt, &with_bwrap[&Boundary::Seatbelt], &seats);
    assert!(!ok);
    assert_eq!(
        line,
        "seats [\"ship\", \"verify\"] declare hands and will refuse to spawn: `seatbelt` is \
         built by slice (ii) of decision 0046 ruling 6, not by this engine"
    );
    let (ok, line) = hands_line(Boundary::Container, &without[&Boundary::Container], &[]);
    assert!(!ok);
    assert!(
        line.starts_with("boxed seats will refuse to spawn: `container` is built by slice (iii)"),
        "{line}"
    );
}
