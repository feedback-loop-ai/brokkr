//! `pinned_refusal.rs`'s own unit tests: each check of the pinned-script
//! admission names its own variant, and every variant reads as the text
//! the operator has always seen (#353).

use super::*;
use crate::bundle::{pinned_key, pinned_script};

/// A declaring layer holding one pinned script, `scripts/check.sh`.
struct Layer {
    dir: tempfile::TempDir,
}

impl Layer {
    fn new() -> Layer {
        let layer = Layer {
            dir: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir_all(layer.path().join("scripts")).unwrap();
        std::fs::write(layer.path().join("scripts/check.sh"), "#!/bin/sh\ntrue\n").unwrap();
        layer
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }
}

/// A `{brokkr} driver exec` dispatch whose argv ends in `tail`.
fn exec(tail: &[&str]) -> Vec<String> {
    ["engine", "driver", "exec"]
        .iter()
        .chain(tail)
        .map(|part| part.to_string())
        .collect()
}

#[test]
fn each_dispatch_check_refuses_in_its_own_variant() {
    let layer = Layer::new();
    let dir = layer.path();
    let bare = exec(&["./scripts/check.sh"]);
    assert_eq!(
        pinned_script(dir, &bare),
        Err(PinnedScriptRefusal::NoSeparator)
    );
    let option = exec(&["--", "bash", "-c", "./scripts/check.sh"]);
    let refused = pinned_script(dir, &option);
    assert_eq!(
        refused,
        Err(PinnedScriptRefusal::OptionToken { token: "-c" })
    );
    let spelled = exec(&["--", "/bin/sh", "./scripts/check.sh"]);
    let refused = pinned_script(dir, &spelled);
    assert_eq!(
        refused,
        Err(PinnedScriptRefusal::Spelled { token: "/bin/sh" })
    );
    let interpreters = exec(&["--", "bash", "true"]);
    let after = &interpreters[4..];
    let refused = pinned_script(dir, &interpreters);
    assert_eq!(refused, Err(PinnedScriptRefusal::NoScript { after }));
}

#[test]
fn each_lookup_check_refuses_in_its_own_variant() {
    let layer = Layer::new();
    let dir = layer.path();
    let startup = PinnedScriptRefusal::Startup {
        token: "./scripts/run*.sh",
        component: "run*.sh",
        character: '*',
    };
    let refused = pinned_key(dir, "./scripts/run*.sh", "scripts/run*.sh");
    assert_eq!(refused, Err(startup));
    let refused = pinned_key(dir, "./scripts/../check.sh", "scripts/../check.sh");
    let token = "./scripts/../check.sh";
    assert_eq!(refused, Err(PinnedScriptRefusal::NotPlain { token }));
    let unpinned = PinnedScriptRefusal::Unpinned {
        token: "./.git/hooks/run.sh",
        relative: ".git/hooks/run.sh",
        holds: "git's own metadata, which no bundle pins",
    };
    let refused = pinned_key(dir, "./.git/hooks/run.sh", ".git/hooks/run.sh");
    assert_eq!(refused, Err(unpinned));
    let refused = pinned_key(dir, "./scripts/missing.sh", "scripts/missing.sh");
    let token = "./scripts/missing.sh";
    assert_eq!(refused, Err(PinnedScriptRefusal::NotAFile { token, dir }));
}

#[test]
fn a_lookup_git_cannot_answer_carries_git_s_refusal() {
    let layer = Layer::new();
    let file = layer.path().join("scripts/check.sh");
    // What this host's spawn says of git started in a file, asked of the
    // host directly rather than of the code under test.
    let detail = std::process::Command::new("git")
        .current_dir(&file)
        .output()
        .unwrap_err()
        .to_string();
    let refused = pinned_key(&file, "./x/run.sh", "x/run.sh");
    let bundle = file.clone();
    let git = SkipError::Git { bundle, detail };
    assert_eq!(refused, Err(PinnedScriptRefusal::Git(git)));
}

#[test]
fn every_pinned_script_refusal_reads_as_it_always_has() {
    let after = ["bash".to_string(), "true".to_string()];
    let skip = SkipError::Git {
        bundle: "/layer".into(),
        detail: "spawn failed".to_string(),
    };
    let cases = [
        (
            PinnedScriptRefusal::NoSeparator,
            "the dispatch carries no `--` after `exec`",
        ),
        (
            PinnedScriptRefusal::OptionToken { token: "-c" },
            "'-c' is an option token before the script; interpreters take no options here \
             (`bash -c` would run text, not pinned bytes)",
        ),
        (
            PinnedScriptRefusal::Spelled { token: "/bin/sh" },
            "'/bin/sh' is spelled as a path that is not `./`-relative to the bundle; only a \
             `./` script the manifest walk pins is admitted unboxed",
        ),
        (
            PinnedScriptRefusal::NoScript { after: &after },
            "the command names no `./`-relative script after its interpreters (bash true)",
        ),
        (
            PinnedScriptRefusal::Startup {
                token: "./a/b*.sh",
                component: "b*.sh",
                character: '*',
            },
            "'./a/b*.sh' has component \"b*.sh\" containing refused startup character '*'; \
             rename the component to meet the pinned-script admission rule (decision 0048; \
             decision 0046 ruling 4)",
        ),
        (
            PinnedScriptRefusal::NotPlain { token: "./a//b" },
            "'./a//b' is not a plain `./`-relative path: every component is a plain name, \
             never `..`, `.`, empty, a `\\` or a drive",
        ),
        (
            PinnedScriptRefusal::Git(skip),
            "asking git which trees the bundle at /layer ignores failed (spawn failed); a \
             bundle's walk skips a git-ignored tree only when git says so, never by guessing",
        ),
        (
            PinnedScriptRefusal::Unpinned {
                token: "./target/run.sh",
                relative: "target/run.sh",
                holds: "workspace data, never bundle bytes",
            },
            "'./target/run.sh' names a path the manifest walk does not pin (target/run.sh is \
             workspace data, never bundle bytes)",
        ),
        (
            PinnedScriptRefusal::NotAFile {
                token: "./run.sh",
                dir: Path::new("/layer"),
            },
            "'./run.sh' names no regular file under the declaring layer /layer",
        ),
    ];
    for (refusal, text) in cases {
        assert_eq!(refusal.to_string(), text, "{refusal:?}");
    }
}
