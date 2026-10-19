use std::sync::LazyLock;

use super::*;
use crate::agents::Adapters;

/// The engine executable the crate's fixtures compile and compose with:
/// this test binary's own, as every entry point resolves it.
pub(crate) static ENGINE: LazyLock<Executable> = LazyLock::new(Executable::current);

fn strings(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

/// What every reader of the convention answers for one argv: the
/// dispatch, the model-bearing built-in, the inline driver, the exec test
/// both engine sites read, the permission template, the harness, and
/// whether an adapter declaring it may stand behind an inline `claude`
/// command (`Ok` its template, `Err` refused).
type Answers<'a> = (
    Dispatch<'a>,
    Option<&'static str>,
    Option<String>,
    bool,
    &'a [String],
    &'a str,
    Result<Option<Vec<String>>, ()>,
);

/// [`Answers`] for `argv`, the inline template read off `claude` declaring
/// it as its driver.
fn answers<'a>(claude: &crate::agents::Adapter, argv: &'a [String]) -> Answers<'a> {
    let adapter = crate::agents::Adapter {
        driver: argv.to_vec(),
        ..claude.clone()
    };
    let inline = crate::agents::inline_template(&adapter, "claude");
    (
        Dispatch::of(argv),
        crate::bundle::built_in_model_driver(argv),
        crate::bundle::dispatch_driver(argv),
        crate::engine::is_exec_dispatch(argv),
        crate::agents::permission_template(argv),
        crate::capabilities::harness_of(argv),
        inline
            .map(|segment| segment.map(|segment| segment.argv))
            .map_err(|_| ()),
    )
}

/// Each of the convention's readers answers each argv as it did before
/// one type read it (#349, ask 8): the kind behind `driver`, the tail less
/// exactly one escape, and every other argv dispatching nothing.
#[test]
fn every_reader_of_the_dispatch_convention_answers_from_one_reading() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters");
    let shipped = Adapters::load(&root).expect("the shipped adapters load");
    let adapter = shipped
        .adapter("claude")
        .expect("the shipped claude adapter");
    let answers = |argv| answers(adapter, argv);
    let exec = strings(&["{brokkr}", "driver", "exec", "--", "x"]);
    let model = strings(&["/abs/brokkr", "driver", "claude", "--", "--model", "m"]);
    let bare = strings(&["x", "driver", "claude", "--model", "m"]);
    let twice = strings(&["x", "driver", "claude", "--", "--", "y"]);
    let unescaped = strings(&["x", "driver", "codex"]);
    let (short, unmarked) = (strings(&["x", "driver"]), strings(&["x", "run", "claude"]));
    let none: &[String] = &[];
    let opaque = crate::capabilities::OPAQUE_HARNESS;
    let driver = |kind, tail| Dispatch::Driver { kind, tail };
    assert_eq!(
        answers(&exec),
        (
            driver("exec", &exec[4..]),
            None,
            Some("exec".into()),
            true,
            &exec[4..],
            "exec",
            Err(())
        )
    );
    let claude = |tail: &[String]| Ok(Some(tail.to_vec()));
    let (tail, built) = (&model[4..], Some("claude"));
    assert_eq!(
        answers(&model),
        (
            driver("claude", tail),
            built,
            Some("claude".into()),
            false,
            tail,
            "claude",
            claude(tail)
        )
    );
    let tail = &bare[3..];
    assert_eq!(
        answers(&bare),
        (
            driver("claude", tail),
            built,
            Some("claude".into()),
            false,
            tail,
            "claude",
            claude(tail)
        )
    );
    let tail = &twice[4..];
    assert_eq!(tail, ["--", "y"]);
    assert_eq!(
        answers(&twice),
        (
            driver("claude", tail),
            built,
            Some("claude".into()),
            false,
            tail,
            "claude",
            claude(tail)
        )
    );
    let codex = (
        driver("codex", none),
        Some("codex"),
        Some("codex".into()),
        false,
    );
    assert_eq!(
        answers(&unescaped),
        (codex.0, codex.1, codex.2, codex.3, none, "codex", Err(()))
    );
    for raw in [&short[..], &unmarked, none] {
        let refused = (Dispatch::Raw, None, None, false, none, opaque, Err(()));
        assert_eq!(answers(raw), refused, "{raw:?}");
    }
    assert_eq!(answers(&strings(&["x", "driver", "claude"])).6, Ok(None));
}

/// The executable as each reader spells it, readable and not: the compile's
/// `{brokkr}` has its own fallback word ([`crate::bundle::expand_command`]),
/// and the engine's spawns the empty path.
#[test]
fn an_executable_reads_as_its_path_or_the_empty_one() {
    let named = Executable(Some(PathBuf::from("/opt/brokkr-test")));
    let named = (named.path(), named.or_empty(), named.lossy());
    let opt = Path::new("/opt/brokkr-test");
    assert_eq!(named, (Some(opt), opt, "/opt/brokkr-test".to_string()));
    let unread = Executable(None);
    let unread = (unread.path(), unread.or_empty(), unread.lossy());
    assert_eq!(unread, (None, Path::new(""), String::new()));
    let current = std::env::current_exe().ok();
    assert_eq!(*ENGINE, Executable(current));
}
