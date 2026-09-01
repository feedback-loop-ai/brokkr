//! Every distribution channel serves the release's own artifacts.
//!
//! The claim these tests defend is provenance: apt, dnf, cargo-binstall,
//! homebrew, scoop and nix all hand a user bytes that came out of one
//! attested build, named by one matrix in `release.yml`. So the tests
//! read that matrix and hold every channel's configuration against it,
//! and they run the repository-building scripts for real rather than
//! reading them for plausibility.
//!
//! Two environment variables steer the parts that need tools:
//!
//! - `BROKKR_PACKAGING_TOOLS=required` turns "tool missing, skipped" into
//!   a failure. CI sets it, so a skip can never be mistaken for a pass.
//! - `BROKKR_PACKAGING_DIST=<dir>` points at real nfpm output. Without
//!   it the apt test builds a synthetic `.deb` with `dpkg-deb`, which
//!   exercises the same script against the same file format.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

/// This file lives at `crates/brokkr-cli/tests/`.
fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = workspace().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn tools_are_required() -> bool {
    std::env::var("BROKKR_PACKAGING_TOOLS").as_deref() == Ok("required")
}

fn present(tool: &str) -> bool {
    Command::new(tool)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// The one place a tool's absence is turned into a decision: skip on a
/// laptop, fail where CI promised the tool would be there.
fn usable(tools: &[&str]) -> bool {
    let missing: Vec<&str> = tools.iter().copied().filter(|t| !present(t)).collect();
    if missing.is_empty() {
        return true;
    }
    assert!(
        !tools_are_required(),
        "BROKKR_PACKAGING_TOOLS=required, but these are missing: {missing:?}"
    );
    println!("packaging: skipping — missing {missing:?}");
    false
}

fn run(command: &mut Command) -> String {
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("{command:?}: {error}"));
    assert!(
        output.status.success(),
        "{command:?}\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn sha256(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The value of a `key = "value"` or `key: value` line, first match wins.
fn field(text: &str, key: &str) -> String {
    text.lines()
        .find_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix(key)?;
            let rest = rest.trim_start().strip_prefix(['=', ':'])?;
            Some(rest.trim().trim_matches('"').to_string())
        })
        .unwrap_or_else(|| panic!("no {key} in:\n{text}"))
}

fn workspace_version() -> String {
    field(&read("Cargo.toml"), "version")
}

fn repository_url() -> String {
    field(&read("Cargo.toml"), "repository")
}

/// The release matrix, read from the workflow that owns it: rust target
/// triple → the artifact file the release publishes for it.
fn release_artifacts() -> BTreeMap<String, String> {
    let workflow = read(".github/workflows/release.yml");
    let mut artifacts = BTreeMap::new();
    for line in workflow.lines() {
        let line = line.trim();
        let Some(entry) = line.strip_prefix("- {").and_then(|l| l.strip_suffix("}")) else {
            continue;
        };
        let mut target = None;
        let mut name = None;
        for pair in entry.split(',') {
            let (key, value) = pair.split_once(':').expect("matrix entries are key: value");
            match key.trim() {
                "target" => target = Some(value.trim().to_string()),
                "name" => name = Some(value.trim().to_string()),
                _ => {}
            }
        }
        let (target, name) = (target.expect("a target"), name.expect("a name"));
        let suffix = if target.contains("windows") {
            ".zip"
        } else {
            ".tar.gz"
        };
        artifacts.insert(target, format!("{name}{suffix}"));
    }
    assert_eq!(artifacts.len(), 5, "{artifacts:?}");
    artifacts
}

/// `[package.metadata.binstall.overrides."<target>"]` → its `pkg-url`.
fn binstall_overrides() -> BTreeMap<String, String> {
    let manifest = read("crates/brokkr-cli/Cargo.toml");
    let mut overrides = BTreeMap::new();
    let mut target: Option<String> = None;
    for line in manifest.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("[package.metadata.binstall.overrides.") {
            target = Some(rest.trim_end_matches(']').trim_matches('"').to_string());
        } else if line.starts_with('[') {
            target = None;
        } else if let (Some(target), Some(url)) = (&target, line.strip_prefix("pkg-url = ")) {
            overrides.insert(target.clone(), url.trim_matches('"').to_string());
        }
    }
    overrides
}

/// binstall's substitution, as far as our templates use it.
fn resolve(template: &str, target: &str, version: &str, repository: &str) -> String {
    let suffix = if target.contains("windows") {
        ".zip"
    } else {
        ".tar.gz"
    };
    template
        .replace("{ repo }", repository)
        .replace("{ version }", version)
        .replace("{ target }", target)
        .replace("{ name }", "brokkr")
        .replace("{ archive-suffix }", suffix)
}

/// Part 4. Every target the release builds for has a binstall override,
/// and resolving it lands exactly on that target's release asset — the
/// two cannot drift because the expectation is read from the matrix.
#[test]
fn binstall_resolves_to_the_artifact_the_release_matrix_publishes() {
    let (version, repository) = (workspace_version(), repository_url());
    let overrides = binstall_overrides();

    for (target, artifact) in release_artifacts() {
        let template = overrides
            .get(&target)
            .unwrap_or_else(|| panic!("no binstall override for {target}"));
        assert_eq!(
            resolve(template, &target, &version, &repository),
            format!("{repository}/releases/download/v{version}/{artifact}"),
            "{target}"
        );
    }

    let manifest = read("crates/brokkr-cli/Cargo.toml");
    // The archives hold the bare binary at their root, so bin-dir must
    // carry no directory component — binstall's default one would look
    // inside a directory that is not there.
    assert!(
        manifest.contains(r#"bin-dir = "{ bin }{ binary-ext }""#),
        "{manifest}"
    );
    // The windows asset is a zip; every other one a gzipped tar.
    assert_eq!(
        manifest.matches(r#"pkg-fmt = "zip""#).count(),
        1,
        "{manifest}"
    );
}

/// Part 1. The package metadata an operator has to trust: dual licence,
/// a homepage, and one binary — no shim rides along (decision 0019).
#[test]
fn the_nfpm_configuration_ships_one_binary_under_the_dual_licence() {
    let config = read("packaging/nfpm.yaml");
    assert_eq!(field(&config, "name"), "brokkr", "{config}");
    assert_eq!(field(&config, "license"), "MIT OR Apache-2.0", "{config}");
    assert_eq!(
        field(&config, "homepage"),
        "https://github.com/feedback-loop-ai/brokkr",
        "{config}"
    );

    let destinations: Vec<&str> = config
        .lines()
        .filter_map(|line| line.trim().strip_prefix("dst: "))
        .collect();
    assert_eq!(
        destinations,
        [
            "/usr/bin/brokkr",
            "/usr/share/doc/brokkr/LICENSE-MIT",
            "/usr/share/doc/brokkr/LICENSE-APACHE",
        ],
        "{config}"
    );
    assert!(!config.contains("/usr/bin/forge"), "{config}");
}

/// Part 1, the other half: the packages enter the same sidecar →
/// SHA256SUMS → attestation pipeline the tarballs already ride, rather
/// than a side channel that skips the manifest.
#[test]
fn the_release_workflow_puts_the_packages_through_the_attested_pipeline() {
    let workflow = read(".github/workflows/release.yml");
    let packaging = workflow
        .split("- name: package (linux .deb and .rpm)")
        .nth(1)
        .expect("the linux packaging step");
    let (packaging, rest) = packaging
        .split_once("- name: attest packaged binary and checksum")
        .expect("packaging comes before the attestation");

    for fragment in [
        "--packager deb --target \"dist/${{ matrix.name }}.deb\"",
        "--packager rpm --target \"dist/${{ matrix.name }}.rpm\"",
        "shasum -a 256",
    ] {
        assert!(packaging.contains(fragment), "{packaging}");
    }
    // Attestation covers everything in dist/, and the publish job folds
    // every sidecar into the one manifest it then attests.
    assert!(rest.contains("subject-path: dist/*"), "{rest}");
    assert!(rest.contains("cat *.sha256 > SHA256SUMS"), "{rest}");
}

/// Part 2's constitutional half (decision 0012): the signing key is a
/// named repository secret consumed by a workflow step, and no key
/// material is anywhere in the tree.
#[test]
fn the_repository_signing_key_is_a_secret_name_and_never_key_material() {
    let workflow = read(".github/workflows/release.yml");
    let pages = workflow
        .split("\n  pages:\n")
        .nth(1)
        .expect("the pages job")
        .split("\n  channels:\n")
        .next()
        .expect("the pages job ends");

    assert!(
        pages.contains("BROKKR_APT_SIGNING_KEY: ${{ secrets.BROKKR_APT_SIGNING_KEY }}"),
        "{pages}"
    );
    // An unset secret refuses to publish rather than publishing an
    // unsigned repository users would be told to trust.
    assert!(
        pages.contains("refusing to publish an unsigned repository"),
        "{pages}"
    );
    assert!(
        pages.contains("--clearsign --output site/apt/dists/stable/InRelease"),
        "{pages}"
    );
    assert!(pages.contains("gpg --armor --export"), "{pages}");

    for file in [
        "packaging/README.md",
        "packaging/apt/build-repo.sh",
        "packaging/rpm/build-repo.sh",
        ".github/workflows/release.yml",
    ] {
        let text = read(file);
        assert!(!text.contains("BEGIN PGP PRIVATE KEY"), "{file}");
        assert!(!text.contains("BEGIN OPENSSH PRIVATE KEY"), "{file}");
    }
}

/// Part 2. The apt repository builder, run for real: a pool, an index
/// per architecture, and a `Release` whose every digest and size is the
/// truth about the file it names.
#[test]
fn the_apt_repository_builder_writes_a_release_that_checks_out() {
    if !usable(&["bash", "dpkg-deb", "dpkg-scanpackages", "gzip"]) {
        return;
    }
    let work = tempfile::tempdir().expect("a temporary directory");
    let debs = packages_under_test(work.path(), "deb");
    let site = work.path().join("site/apt");

    run(Command::new("bash")
        .arg(workspace().join("packaging/apt/build-repo.sh"))
        .arg("--debs")
        .arg(&debs)
        .arg("--out")
        .arg(&site)
        .env("SOURCE_DATE_EPOCH", "1756771200"));

    let dists = site.join("dists/stable");
    let release = std::fs::read_to_string(dists.join("Release")).expect("a Release file");
    assert!(release.contains("Origin: Brokkr"), "{release}");
    assert!(release.contains("Components: main"), "{release}");
    // apt parses this in the C locale: five fields and an explicit zone.
    let date = release
        .lines()
        .find_map(|line| line.strip_prefix("Date: "))
        .expect("a Date line");
    assert!(date.ends_with(" UTC"), "{date}");
    assert_eq!(date.split_whitespace().count(), 6, "{date}");

    // One index directory per architecture the pool actually holds.
    let mut architectures: Vec<String> = std::fs::read_dir(dists.join("main"))
        .expect("the component directory")
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .strip_prefix("binary-")
                .map(str::to_string)
        })
        .collect();
    architectures.sort();
    assert!(!architectures.is_empty());

    // Signing is the workflow's job; the builder must not have left
    // anything that looks signed behind.
    assert!(!dists.join("InRelease").exists());
    assert!(!dists.join("Release.gpg").exists());

    let mut checked = 0;
    let mut section = None;
    for line in release.lines() {
        if let Some(name) = line.strip_suffix(':').filter(|_| !line.starts_with(' ')) {
            section = Some(name.to_string());
            continue;
        }
        let Some(entry) = line.strip_prefix(' ') else {
            continue;
        };
        if section.as_deref() != Some("SHA256") {
            continue;
        }
        let mut parts = entry.split_whitespace();
        let (digest, size, path) = (
            parts.next().expect("a digest"),
            parts.next().expect("a size"),
            parts.next().expect("a path"),
        );
        let indexed = dists.join(path);
        assert_eq!(sha256(&indexed), digest, "{path}");
        assert_eq!(
            indexed
                .metadata()
                .expect("the indexed file")
                .len()
                .to_string(),
            size,
            "{path}"
        );
        checked += 1;
    }
    // Packages and Packages.gz, for each architecture.
    assert_eq!(checked, 2 * architectures.len(), "{release}");
    assert!(
        release.contains(&format!("Architectures: {}", architectures.join(" "))),
        "{release}"
    );

    for arch in &architectures {
        let packages = std::fs::read_to_string(dists.join(format!("main/binary-{arch}/Packages")))
            .expect("an index");
        assert!(packages.contains("Package: brokkr"), "{packages}");
        assert!(
            packages.contains(&format!("Architecture: {arch}")),
            "{packages}"
        );
        let filename = field(&packages, "Filename");
        assert!(filename.starts_with("pool/main/b/brokkr/"), "{filename}");
        assert!(site.join(&filename).exists(), "{filename}");
    }
}

/// Part 3. The dnf side of the same site: one directory per `$basearch`,
/// a `.repo` file that checks the metadata signature, and — where
/// createrepo_c exists — real repodata.
#[test]
fn the_rpm_repository_builder_lays_out_a_dnf_tree() {
    if !usable(&["bash"]) {
        return;
    }
    let work = tempfile::tempdir().expect("a temporary directory");
    let rpms = work.path().join("rpms");
    std::fs::create_dir_all(&rpms).expect("a directory");

    // Real nfpm output when CI hands it over; otherwise a stand-in that
    // exercises the layout and the refusal, which is all this script
    // decides before createrepo_c takes over.
    let real = provided_packages("rpm");
    let mut expected = Vec::new();
    if real.is_empty() {
        std::fs::write(rpms.join("brokkr-linux-x86_64.rpm"), b"not an rpm").expect("a file");
        std::fs::write(rpms.join("brokkr-linux-aarch64.rpm"), b"not an rpm").expect("a file");
        expected.extend(["x86_64".to_string(), "aarch64".to_string()]);
    } else {
        for package in &real {
            let name = package.file_name().expect("a name");
            std::fs::copy(package, rpms.join(name)).expect("a copy");
            let name = name.to_string_lossy();
            expected.push(
                if name.contains("aarch64") {
                    "aarch64"
                } else {
                    "x86_64"
                }
                .to_string(),
            );
        }
    }

    let site = work.path().join("site/rpm");
    let base_url = "https://feedback-loop-ai.github.io/brokkr/rpm";
    let mut command = Command::new("bash");
    command
        .arg(workspace().join("packaging/rpm/build-repo.sh"))
        .arg("--rpms")
        .arg(&rpms)
        .arg("--out")
        .arg(&site)
        .arg("--base-url")
        .arg(base_url);
    let full = !real.is_empty() && usable(&["createrepo_c"]);
    if !full {
        command.arg("--layout-only");
    }
    run(&mut command);

    let repo = std::fs::read_to_string(site.join("brokkr.repo")).expect("a .repo file");
    assert!(
        repo.contains("baseurl=https://feedback-loop-ai.github.io/brokkr/rpm/$basearch"),
        "{repo}"
    );
    // Metadata is signed, packages are not (yet) — the file says so
    // rather than claiming a guarantee this slice does not provide.
    assert!(repo.contains("repo_gpgcheck=1"), "{repo}");
    assert!(repo.contains("gpgcheck=0"), "{repo}");
    assert!(
        repo.contains(
            "gpgkey=https://feedback-loop-ai.github.io/brokkr/brokkr-archive-keyring.asc"
        ),
        "{repo}"
    );

    for arch in &expected {
        assert!(site.join(arch).is_dir(), "{arch}");
        if full {
            assert!(
                site.join(arch).join("repodata/repomd.xml").is_file(),
                "{arch}"
            );
        }
    }
}

/// Part 3's refusal: an rpm whose name does not say which `$basearch` it
/// belongs to stops the build rather than landing somewhere plausible.
#[test]
fn the_rpm_builder_refuses_a_package_it_cannot_place() {
    if !usable(&["bash"]) {
        return;
    }
    let work = tempfile::tempdir().expect("a temporary directory");
    let rpms = work.path().join("rpms");
    std::fs::create_dir_all(&rpms).expect("a directory");
    std::fs::write(rpms.join("brokkr-linux-s390x.rpm"), b"not an rpm").expect("a file");

    let output = Command::new("bash")
        .arg(workspace().join("packaging/rpm/build-repo.sh"))
        .arg("--rpms")
        .arg(&rpms)
        .arg("--out")
        .arg(work.path().join("site"))
        .arg("--base-url")
        .arg("https://example.invalid/rpm")
        .arg("--layout-only")
        .output()
        .expect("the script runs");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("no architecture in its name"),
        "{output:?}"
    );
}

/// Parts 5 and 6. The bump script renders every channel from one
/// attested manifest: same digests, same version, and running it twice
/// changes nothing the second time.
#[test]
fn the_bump_script_renders_every_channel_from_one_manifest() {
    if !usable(&["bash", "awk", "sed"]) {
        return;
    }
    let work = tempfile::tempdir().expect("a temporary directory");
    let root = work.path().join("tree");
    for relative in [
        "flake.nix",
        "packaging/homebrew/brokkr.rb",
        "packaging/scoop/brokkr.json",
    ] {
        let destination = root.join(relative);
        std::fs::create_dir_all(destination.parent().expect("a parent")).expect("a directory");
        std::fs::copy(workspace().join(relative), &destination).expect("a copy");
    }

    // A synthetic manifest in the exact shape the release publishes:
    // `<digest>  <artifact>`, one line per asset.
    let digests: BTreeMap<String, String> = release_artifacts()
        .into_values()
        .enumerate()
        .map(|(index, artifact)| {
            let digit = char::from_digit(index as u32 + 1, 10).expect("a digit");
            (artifact, std::iter::repeat_n(digit, 64).collect::<String>())
        })
        .collect();
    let sums = work.path().join("SHA256SUMS");
    let manifest: String = digests
        .iter()
        .map(|(artifact, digest)| format!("{digest}  {artifact}\n"))
        .collect();
    std::fs::write(&sums, &manifest).expect("a manifest");

    let bump = || {
        run(Command::new("bash")
            .arg(workspace().join("packaging/bump-from-sums.sh"))
            .arg("--version")
            .arg("9.9.9")
            .arg("--sums")
            .arg(&sums)
            .arg("--root")
            .arg(&root))
    };
    bump();
    let rendered: Vec<String> = [
        "flake.nix",
        "packaging/homebrew/brokkr.rb",
        "packaging/scoop/brokkr.json",
    ]
    .iter()
    .map(|relative| std::fs::read_to_string(root.join(relative)).expect("a rendered file"))
    .collect();
    bump();

    for (index, relative) in [
        "flake.nix",
        "packaging/homebrew/brokkr.rb",
        "packaging/scoop/brokkr.json",
    ]
    .iter()
    .enumerate()
    {
        let text = std::fs::read_to_string(root.join(relative)).expect("a rendered file");
        assert_eq!(text, rendered[index], "{relative} is not idempotent");
        assert!(text.contains("9.9.9"), "{relative}: {text}");
        assert!(
            !text.contains("0000000000000000000000000000000000000000000000000000000000000000"),
            "{relative} still carries a placeholder: {text}"
        );
    }

    let (flake, formula, manifest_json) = (&rendered[0], &rendered[1], &rendered[2]);
    for (artifact, digest) in &digests {
        if artifact.ends_with(".zip") {
            assert!(
                manifest_json.contains(digest),
                "{artifact}: {manifest_json}"
            );
            assert!(
                !flake.contains(digest),
                "{artifact} does not belong in the flake"
            );
        } else {
            assert!(flake.contains(digest), "{artifact}: {flake}");
            assert!(formula.contains(digest), "{artifact}: {formula}");
        }
    }
    // Scoop's autoupdate keeps its own placeholder; the rendered URL is
    // the concrete one.
    assert!(
        manifest_json.contains("/download/v9.9.9/brokkr-windows-x86_64.zip"),
        "{manifest_json}"
    );
    assert!(
        manifest_json.contains("/download/v$version/brokkr-windows-x86_64.zip"),
        "{manifest_json}"
    );
}

/// The bump script refuses a manifest that is missing an artifact rather
/// than rendering a channel with a stale digest.
#[test]
fn the_bump_script_refuses_an_incomplete_manifest() {
    if !usable(&["bash"]) {
        return;
    }
    let work = tempfile::tempdir().expect("a temporary directory");
    let sums = work.path().join("SHA256SUMS");
    std::fs::write(
        &sums,
        format!("{}  brokkr-linux-x86_64.tar.gz\n", "a".repeat(64)),
    )
    .expect("a manifest");

    let output = Command::new("bash")
        .arg(workspace().join("packaging/bump-from-sums.sh"))
        .arg("--version")
        .arg("9.9.9")
        .arg("--sums")
        .arg(&sums)
        .arg("--root")
        .arg(work.path())
        .output()
        .expect("the script runs");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("not a complete release manifest"),
        "{output:?}"
    );
}

/// Parts 5 and 6, in the committed tree: the templates carry the
/// placeholder digest, so no channel can be published from this
/// repository by hand and pass for a real one — and each names exactly
/// the artifacts the release matrix builds.
#[test]
fn the_committed_channel_templates_are_unrendered_and_name_the_real_artifacts() {
    let placeholder = "0000000000000000000000000000000000000000000000000000000000000000";
    let artifacts: Vec<String> = release_artifacts().into_values().collect();

    let flake = read("flake.nix");
    let formula = read("packaging/homebrew/brokkr.rb");
    let scoop = read("packaging/scoop/brokkr.json");

    assert_eq!(flake.matches(placeholder).count(), 4, "{flake}");
    assert_eq!(formula.matches(placeholder).count(), 4, "{formula}");
    assert_eq!(scoop.matches(placeholder).count(), 1, "{scoop}");

    for artifact in &artifacts {
        let windows = artifact.ends_with(".zip");
        assert_eq!(flake.contains(artifact), !windows, "{artifact}");
        assert_eq!(formula.contains(artifact), !windows, "{artifact}");
        assert_eq!(scoop.contains(artifact), windows, "{artifact}");
    }

    // One binary, in every channel (decision 0019 ruling 9).
    assert!(formula.contains(r#"bin.install "brokkr""#), "{formula}");
    assert!(scoop.contains(r#""bin": "brokkr.exe""#), "{scoop}");
    assert!(
        flake.contains(r#"install -Dm755 brokkr "$out/bin/brokkr""#),
        "{flake}"
    );
    for template in [&flake, &formula, &scoop] {
        assert!(!template.contains("bin/forge"), "{template}");
    }
}

/// Part 7. The quickstart's install step carries a row per channel, and
/// every command in it either ran here or says where it is wired.
#[test]
fn the_quickstart_install_step_carries_the_manager_matrix() {
    let quickstart = read("docs/guides/quickstart.md");
    let install = quickstart
        .split("## 1. Install")
        .nth(1)
        .expect("the install step")
        .split("\n## ")
        .next()
        .expect("the step ends");

    for row in [
        "brew install",
        "apt-get install brokkr",
        "dnf install brokkr",
        "cargo binstall brokkr-cli",
        "scoop install brokkr",
        "nix profile install",
        "tar xzf",
    ] {
        assert!(install.contains(row), "no row for {row}:\n{install}");
    }
    // The accuracy law: nothing that needs the bench's secret or the
    // sibling repositories is written as if it were tested.
    assert!(install.contains("wired at the bench"), "{install}");
    // The forward note the framing asked for, about the bootstrap
    // slice's spine.
    assert!(install.contains("slice-bootstrap"), "{install}");
}

/// The packaging README is where the operator's steps live: the secrets
/// to provision, and what this slice deliberately did not do.
#[test]
fn the_packaging_readme_names_the_secrets_and_the_out_of_scope_channels() {
    let readme = read("packaging/README.md");
    for secret in [
        "BROKKR_APT_SIGNING_KEY",
        "BROKKR_APT_SIGNING_KEY_PASSPHRASE",
        "BROKKR_TAP_TOKEN",
        "BROKKR_BUCKET_TOKEN",
    ] {
        assert!(
            readme.contains(secret),
            "no {secret} in packaging/README.md"
        );
    }
    for channel in ["AUR", "winget", "snap", "flatpak"] {
        assert!(
            readme.contains(channel),
            "no {channel} in packaging/README.md"
        );
    }
    assert!(readme.contains("wired at the bench"), "{readme}");
    assert!(!readme.contains("BEGIN PGP PRIVATE KEY"), "{readme}");
}

/// Real nfpm output, when CI built it: one binary at `/usr/bin/brokkr`,
/// the dual licence, and no shim.
#[test]
fn the_built_package_carries_one_binary_and_the_declared_metadata() {
    let debs = provided_packages("deb");
    if debs.is_empty() {
        assert!(
            !tools_are_required(),
            "BROKKR_PACKAGING_TOOLS=required, but BROKKR_PACKAGING_DIST held no .deb"
        );
        println!("packaging: skipping — no BROKKR_PACKAGING_DIST packages");
        return;
    }
    if !usable(&["dpkg-deb"]) {
        return;
    }

    for deb in debs {
        let contents = run(Command::new("dpkg-deb").arg("--contents").arg(&deb));
        assert!(contents.contains("./usr/bin/brokkr"), "{contents}");
        assert!(!contents.contains("./usr/bin/forge"), "{contents}");
        assert!(contents.contains("LICENSE-MIT"), "{contents}");
        assert!(contents.contains("LICENSE-APACHE"), "{contents}");

        let control = run(Command::new("dpkg-deb").arg("--field").arg(&deb));
        assert_eq!(field(&control, "Package"), "brokkr", "{control}");
        assert_eq!(
            field(&control, "Homepage"),
            "https://github.com/feedback-loop-ai/brokkr",
            "{control}"
        );
        assert_eq!(field(&control, "Version"), workspace_version(), "{control}");
    }
}

/// `.deb` or `.rpm` files CI built with nfpm, if it handed any over.
fn provided_packages(extension: &str) -> Vec<PathBuf> {
    let Ok(dist) = std::env::var("BROKKR_PACKAGING_DIST") else {
        return Vec::new();
    };
    let dist = workspace().join(dist);
    let Ok(entries) = std::fs::read_dir(&dist) else {
        panic!("BROKKR_PACKAGING_DIST={} is not readable", dist.display());
    };
    let mut packages: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|found| found == extension))
        .collect();
    packages.sort();
    packages
}

/// The packages the apt test indexes: nfpm's own when CI provides them,
/// otherwise a pair built here with `dpkg-deb` so the script is still
/// run against real archives of the real format.
fn packages_under_test(work: &Path, extension: &str) -> PathBuf {
    let debs = work.join("packages");
    std::fs::create_dir_all(&debs).expect("a directory");

    let provided = provided_packages(extension);
    if !provided.is_empty() {
        for package in provided {
            std::fs::copy(&package, debs.join(package.file_name().expect("a name")))
                .expect("a copy");
        }
        // One architecture is enough for a real-package run; the
        // synthetic path below covers both.
        return debs;
    }

    for (arch, name) in [
        ("amd64", "brokkr-linux-x86_64"),
        ("arm64", "brokkr-linux-aarch64"),
    ] {
        let root = work.join(format!("build-{arch}"));
        std::fs::create_dir_all(root.join("DEBIAN")).expect("a directory");
        std::fs::create_dir_all(root.join("usr/bin")).expect("a directory");
        std::fs::write(root.join("usr/bin/brokkr"), b"#!/bin/sh\necho brokkr\n").expect("a file");
        std::fs::write(
            root.join("DEBIAN/control"),
            format!(
                "Package: brokkr\nVersion: {}\nArchitecture: {arch}\n\
                 Maintainer: feedback-loop-ai <valentin@cmd.bg>\nSection: devel\n\
                 Priority: optional\nHomepage: https://github.com/feedback-loop-ai/brokkr\n\
                 Description: a stand-in for the repository builder test\n",
                workspace_version()
            ),
        )
        .expect("a file");
        run(Command::new("dpkg-deb")
            .arg("--build")
            .arg(&root)
            .arg(debs.join(format!("{name}.deb"))));
    }
    debs
}
