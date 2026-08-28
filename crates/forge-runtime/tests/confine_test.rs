//! The policy-confined trust class: command wrapping is deterministic
//! data-in, argv-out; absence of confinement is the trusted native class.

use forge_runtime::engine::confined_command;
use forge_runtime::Confine;
use std::path::Path;

#[test]
fn confined_command_wraps_and_trusted_passes_through() {
    let command = vec!["python3".to_string(), "/b/drivers/x.py".to_string()];
    let workdir = Path::new("/work/repo");
    let bundle = Path::new("/b");

    assert_eq!(
        confined_command(&command, None, workdir, bundle),
        command,
        "trusted class: native child process, untouched"
    );

    let confine = Confine {
        image: "ubuntu:24.04".into(),
        network: false,
        mounts: vec!["/extra".into()],
    };
    let wrapped = confined_command(&command, Some(&confine), workdir, bundle);
    let rendered = wrapped.join(" ");
    assert!(rendered.starts_with("docker run --rm -i"));
    assert!(rendered.contains("-v /work/repo:/work/repo "));
    assert!(rendered.contains("-v /b:/b:ro"));
    assert!(rendered.contains("--network=none"));
    assert!(rendered.contains("-v /extra:/extra:ro"));
    assert!(rendered.ends_with("ubuntu:24.04 python3 /b/drivers/x.py"));

    let open = Confine {
        image: "img".into(),
        network: true,
        mounts: vec![],
    };
    let rendered = confined_command(&command, Some(&open), workdir, bundle).join(" ");
    assert!(
        !rendered.contains("--network=none"),
        "granted network is not cut"
    );
}
