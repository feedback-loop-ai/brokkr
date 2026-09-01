//! The policy-confined trust class: command wrapping is deterministic
//! data-in, argv-out; absence of confinement is the trusted native class.

use brokkr_runtime::engine::confined_command;
use brokkr_runtime::Confine;
use std::path::{Path, PathBuf};

#[test]
fn confined_command_wraps_and_trusted_passes_through() {
    let command = vec!["python3".to_string(), "/b/drivers/x.py".to_string()];
    let workdir = Path::new("/work/repo");
    let roots = vec![PathBuf::from("/b")];

    assert_eq!(
        confined_command(&command, None, workdir, &roots),
        command,
        "trusted class: native child process, untouched"
    );

    let confine = Confine {
        image: "ubuntu:24.04".into(),
        network: false,
        mounts: vec!["/extra".into()],
    };
    let wrapped = confined_command(&command, Some(&confine), workdir, &roots);
    let rendered = wrapped.join(" ");
    assert_eq!(
        rendered,
        "docker run --rm -i -v /work/repo:/work/repo -v /b:/b:ro -w /work/repo \
         --network=none -v /extra:/extra:ro ubuntu:24.04 python3 /b/drivers/x.py",
        "a single-root bundle emits exactly the argv it always did"
    );

    let open = Confine {
        image: "img".into(),
        network: true,
        mounts: vec![],
    };
    let rendered = confined_command(&command, Some(&open), workdir, &roots).join(" ");
    assert!(
        !rendered.contains("--network=none"),
        "granted network is not cut"
    );
}

#[test]
fn every_composition_root_is_mounted_read_only() {
    // Decision 0017: an inherited confined seat's role file lives in its
    // ancestor's directory. One mount per root, leaf first — an
    // unmounted role would be a driver-level file-not-found hours into a
    // run, not a compile-time refusal.
    let command = vec!["python3".to_string()];
    let roots = vec![PathBuf::from("/lib/leaf"), PathBuf::from("/lib/base")];
    let confine = Confine {
        image: "img".into(),
        network: true,
        mounts: vec![],
    };
    let rendered = confined_command(&command, Some(&confine), Path::new("/work"), &roots).join(" ");
    assert_eq!(
        rendered,
        "docker run --rm -i -v /work:/work -v /lib/leaf:/lib/leaf:ro \
         -v /lib/base:/lib/base:ro -w /work img python3"
    );
}
