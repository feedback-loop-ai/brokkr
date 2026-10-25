//! Codex's isolation options in the final-command grammar (decision 0065
//! slice two, U9c1; SI2), as the controller measured them on codex-cli
//! 0.162.1: `--ignore-user-config`, `--disable apps`, `--disable plugins`
//! and the engine's permission profile read as the sandbox class it
//! renders, bound to the seat's workspace root and the secrets store's
//! handed path (operator ruling 4 of 2026-10-23), on cold, replacement and
//! rejoin commands, in every spelling the grammar folds; each other toggle,
//! profile and selector, a profile with no store handed, and a profile
//! beside `-s` or `sandbox_mode`, refusing with its own cause. The Codex
//! part of the hands' transport test moved here from the parent suite,
//! which sits at its line baseline, when the profile came to carry the
//! class.

use super::*;
use crate::native_controls::read::{expressed, Unread};
use grammar::{Bound, Unisolated};

/// The hands' missing class, as the composer names it.
pub(super) const UNCLASSED: &str = "sandbox class ('--sandbox' or the engine's permission profile)";

/// The secrets store's path the fixtures hand beside the workdir `/w`.
const STORE: &str = "/store";

/// The paths every fixture profile is bound to.
const BOUND: Bound<'static> = Bound {
    workdir: "/w",
    store: Some(STORE),
};

/// The isolation options in their measured order.
const ISOLATION: [&str; 5] = [
    "--ignore-user-config",
    "--disable",
    "apps",
    "--disable",
    "plugins",
];

/// The selector of the profile [`definition`] names.
const SELECTOR: &str = "default_permissions=\"brokkr\"";

/// The read-only class's filesystem entries: the root read, the store
/// denied.
const READ_ONLY: &str = r#"":root"="read","/store"="deny""#;
/// The workspace-write class's: beside them, the workspace root written.
const WORKSPACE_WRITE: &str = r#"":root"="read","/w"="write","/store"="deny""#;

/// The profile `brokkr` over `entries`.
fn definition(entries: &str) -> String {
    format!("permissions.brokkr={{filesystem={{{entries}}}}}")
}

/// `assignment` as a profile, selected.
fn paired(assignment: &str) -> Vec<String> {
    argv(&["-c", assignment, "-c", SELECTOR])
}

/// The isolation options and the profile over `entries`, selected.
fn rendered(entries: &str) -> Vec<String> {
    [argv(&ISOLATION), paired(&definition(entries))].concat()
}

/// The three isolation effects and the class, in command order.
fn isolated(class: &'static str) -> Vec<Expressed> {
    vec![
        Expressed::Isolation(Isolation::UserConfig),
        Expressed::Isolation(Isolation::Apps),
        Expressed::Isolation(Isolation::Plugins),
        Expressed::Class(class),
    ]
}

/// A cold command and a rejoin around `options`, after the binary, each
/// beside the argument number its first option stands at.
fn shapes(options: &[String]) -> [(Vec<String>, usize); 2] {
    let cold = [argv(&["exec", "--json", "-C", "/w"]), options.to_vec()];
    let rejoin = [
        argv(&["exec", "resume", "--json"]),
        options.to_vec(),
        argv(&[CODEX_THREAD, "-"]),
    ];
    [(cold.concat(), 5), (rejoin.concat(), 4)]
}

/// `command`, after its binary, placed at its fixed positions.
fn placed(command: &[String]) -> grammar::Command {
    let parsed = grammar::parse_final("codex", command).expect("modelled");
    parsed.expect("a placed command").command
}

/// The first isolation fault `command` reads with bound to `bound`, in the
/// order the final reading meets them: the profile read whole, then each
/// toggle.
fn fault(command: &[String], bound: Bound<'_>) -> Option<Unisolated> {
    let command = placed(command);
    let toggles = command.nodes.iter().filter_map(grammar::isolation);
    let toggled = toggles.filter_map(Result::err).next();
    grammar::profile(&command, bound).err().or(toggled)
}

/// 45.3: the measured set and each class's profile read as the isolation
/// effects and that class, cold and rejoined, whichever of the grammar's
/// spellings writes them; the two tables are capability tables.
#[test]
fn the_measured_options_and_profile_read_as_the_class_they_render() {
    for (entries, class) in [
        (READ_ONLY, SandboxIntent::ReadOnly),
        (WORKSPACE_WRITE, SandboxIntent::WorkspaceWrite),
    ] {
        let (profile, selector) = (definition(entries), SELECTOR);
        let joined = vec![format!("-c={profile}"), format!("-c{selector}")];
        let long = [
            argv(&["--config", &profile]),
            vec![format!("--config={selector}")],
        ]
        .concat();
        let toggles = argv(&[
            "--ignore-user-config",
            "--disable=apps",
            "--disable=plugins",
        ]);
        for options in [
            rendered(entries),
            [argv(&ISOLATION), joined].concat(),
            [argv(&ISOLATION), long].concat(),
            [toggles, paired(&profile)].concat(),
        ] {
            for (command, _) in shapes(&options) {
                assert_eq!(grammar::profile(&placed(&command), BOUND), Ok(Some(class)));
                let effects = read_state(&placed(&command), BOUND).map(|state| state.effects);
                assert_eq!(effects, Ok(isolated(class.word())), "{command:?}");
            }
        }
    }
    // An assignment the profile cannot read is its reader's to refuse.
    let unread = [rendered(READ_ONLY), argv(&["-c", "unassigned"])].concat();
    let [(command, _), _] = shapes(&unread);
    assert_eq!(
        grammar::profile(&placed(&command), BOUND),
        Ok(Some(SandboxIntent::ReadOnly))
    );
    assert_eq!(
        read_state(&placed(&command), BOUND).map(|_| ()),
        Err(
            "carries '--config' (argument 14), whose value is not a KEY=VALUE configuration \
             assignment"
                .to_string()
        )
    );
    for (assignment, table) in [
        (definition(READ_ONLY), "permissions"),
        (SELECTOR.to_string(), "default_permissions"),
    ] {
        let setting = grammar::setting(&assignment);
        assert_eq!(setting, Ok(grammar::Setting::Capability(table)));
    }
}

/// [`codex_final`]'s plan at an inline site of `class`, whose adapter
/// fragment is the profile over `entries` beside the isolation options.
fn profiled_site(class: SandboxIntent, entries: &str) -> Sealed {
    let mut sealed = codex_final(false, true);
    sealed.expected.local.sandbox = class;
    sealed.sandbox = rendered(entries);
    sealed
}

/// The cold command [`profiled_site`] is served as, its binary first.
fn served_cold(entries: &str) -> Vec<String> {
    [
        argv(&CODEX_LEAD),
        argv(&["-c", "model_reasoning_effort=\"high\"", "--model", "gpt"]),
        argv(&["--image", "resume"]),
        rendered(entries),
        argv(&["-c", "web_search=\"disabled\""]),
    ]
    .concat()
}

/// [`Sealed::check`] of `command`, the store's path handed beside `/w`.
fn stored(sealed: &Sealed, command: Vec<String>) -> Result<Checked, Refusal> {
    let serving = Serving {
        store: Some(STORE),
        ..Serving::default()
    };
    sealed.check(command, serving)
}

/// 45.3: a cold command — and so the cold replacement of a rejoin the
/// launch declines — checks whole under each class's profile where the
/// store's path is handed, and refuses where none is; one that lost an
/// isolation option its plan composes refuses before any rebuild, and a
/// profile of another class than the site's is two classes.
#[test]
fn a_profiled_cold_command_checks_and_one_that_lost_an_option_refuses() {
    for (class, entries) in [
        (SandboxIntent::ReadOnly, READ_ONLY),
        (SandboxIntent::WorkspaceWrite, WORKSPACE_WRITE),
    ] {
        let (sealed, cold) = (profiled_site(class, entries), served_cold(entries));
        let checked = stored(&sealed, cold.clone()).map(Checked::into_argv);
        assert_eq!(checked, Ok(cold.clone()));
        let cause = format!("cannot be read: it {}", Unisolated::Unhanded(16));
        let unhanded = Err(final_refusal("codex", &cause));
        assert_eq!(sealed.cold(cold.clone()), unhanded);
        for (dropped, option) in [
            (11..12, "the isolation option '--ignore-user-config'"),
            (12..14, "the isolation option '--disable apps'"),
            (14..16, "the isolation option '--disable plugins'"),
        ] {
            let mut lost = cold.clone();
            lost.drain(dropped);
            assert_eq!(stored(&sealed, lost), Err(uncarried("codex", option)));
        }
    }
    let crossed = profiled_site(SandboxIntent::WorkspaceWrite, READ_ONLY);
    assert_eq!(
        stored(&crossed, served_cold(READ_ONLY)),
        Err(final_refusal(
            "codex",
            "is sealed with a 'workspace-write' sandbox class in its typed local declaration and \
             a 'read-only' one in its local sandbox fragment, and one launch runs one class"
        ))
    );
}

/// [`codex_hands`]'s plan whose measured fragment is the profile over
/// `entries` beside the isolation options, then the server's bindings.
fn profiled_hands(entries: &str) -> Sealed {
    let hands = [rendered(entries), argv(&CODEX_HANDS[2..])].concat();
    codex_hands(&hands.iter().map(String::as_str).collect::<Vec<_>>())
}

/// The cold command [`profiled_hands`] is served as, its binary first,
/// the hands' profile written over `written`.
fn hands_cold(written: &str) -> Vec<String> {
    [
        argv(&CODEX_LEAD),
        argv(&["--model", "gpt", "--image", "resume"]),
        rendered(written),
        argv(&["-c", "mcp_servers.brokkr.command=\"/bin/brokkr\"", "-c"]),
        vec![format!("mcp_servers.brokkr.args={SERVER_ARGUMENTS}")],
        argv(&[
            "-c",
            "mcp_servers.brokkr.default_tools_approval_mode=\"approve\"",
            "-c",
            "web_search=\"disabled\"",
        ]),
    ]
    .concat()
}

/// R1 of U9c1's second review (SI2): the final check recomposes the box's
/// hands judged bound to the paths its serving hands, so a profiled hands
/// command — cold, and so the cold replacement of the rejoin the driver
/// declines for the hands' bindings — checks whole where the serving hands
/// the workdir its profile writes and the store it denies. Where it hands
/// no store, another workdir or another store, and where the hands'
/// measured profile is bound to none of them, it refuses.
#[test]
fn profiled_hands_check_whole_against_the_paths_the_serving_hands() {
    let check = |sealed: &Sealed, command: &[String], workdir: &'static str, store| {
        let serving = Serving {
            workdir,
            store,
            ..Serving::default()
        };
        sealed
            .check(command.to_vec(), serving)
            .map(Checked::into_argv)
    };
    let unread = |fault: Unisolated| {
        let cause = format!("cannot be read: it {fault}");
        Err(final_refusal("codex", &cause))
    };
    for entries in [READ_ONLY, WORKSPACE_WRITE] {
        let (boxed, cold) = (profiled_hands(entries), hands_cold(entries));
        assert_eq!(check(&boxed, &cold, "/w", Some(STORE)), Ok(cold.clone()));
        let unhanded = unread(Unisolated::Unhanded(14));
        assert_eq!(check(&boxed, &cold, "/w", None), unhanded);
        let moved = unread(Unisolated::Unstored(14));
        assert_eq!(check(&boxed, &cold, "/w", Some("/moved")), moved);
    }
    let (boxed, cold) = (profiled_hands(WORKSPACE_WRITE), hands_cold(WORKSPACE_WRITE));
    let elsewhere = unread(Unisolated::Unrooted(14));
    assert_eq!(check(&boxed, &cold, "/elsewhere", Some(STORE)), elsewhere);
    // The hostile profile, writing `/` and denying a path that is not the
    // store, as the hands' measured fragment: served as written it is read
    // as the profile it is, and under a bound command the sealed hands are
    // read bound too, and refuse before anything is composed.
    let hostile = r#"":root"="read","/"="write","/nonexistent"="deny""#;
    let boxed = profiled_hands(hostile);
    let unrooted = unread(Unisolated::Unrooted(14));
    assert_eq!(
        check(&boxed, &hands_cold(hostile), "/w", Some(STORE)),
        unrooted
    );
    let bound = hands_cold(WORKSPACE_WRITE);
    let unsealed = Err(final_refusal(
        "codex",
        "is sealed with a boundary that cannot be read",
    ));
    assert_eq!(check(&boxed, &bound, "/w", Some(STORE)), unsealed);
}

/// Each case of the refusal tables: the options, the index of the one the
/// fault stands at among them, and the fault.
type Case = (Vec<String>, usize, fn(usize) -> Unisolated);

/// Every case refuses with its fault at its argument, cold and rejoined,
/// bound to `bound`, and the final reading refuses in that fault's words.
fn refuses_bound(bound: Bound<'_>, cases: Vec<Case>) {
    for (options, at, fault_at) in cases {
        for (command, lead) in shapes(&options) {
            let expected = fault_at(lead + at);
            assert_eq!(fault(&command, bound), Some(expected), "{command:?}");
            let read = read_state(&placed(&command), bound).map(|_| ());
            assert_eq!(read, Err(expected.to_string()), "{command:?}");
        }
    }
}

/// [`refuses_bound`] under the fixtures' [`BOUND`].
fn refuses(cases: Vec<Case>) {
    refuses_bound(BOUND, cases);
}

/// Operator ruling 4 of 2026-10-23: the profile is never read by its shape
/// alone. A write of any path but the workspace root, a deny of any path
/// but the handed store, a second deny, an extra read, a path named twice
/// and a profile with no store handed each refuse with its own cause.
#[test]
fn a_profile_binds_its_write_to_the_workdir_and_its_deny_to_the_store() {
    let profile = |entries: &str| paired(&definition(entries));
    refuses(vec![
        (
            profile(r#"":root"="read","/"="write","/store"="deny""#),
            0,
            Unisolated::Unrooted,
        ),
        (
            profile(r#"":root"="read","/w"="write","/x"="write","/store"="deny""#),
            0,
            Unisolated::Unrooted,
        ),
        (
            profile(r#"":root"="write","/store"="deny""#),
            0,
            Unisolated::Unrooted,
        ),
        (
            profile(r#"":root"="read","/w"="write","/nonexistent"="deny""#),
            0,
            Unisolated::Unstored,
        ),
        (
            profile(r#"":root"="read","/store"="deny","/other"="deny""#),
            0,
            Unisolated::Unstored,
        ),
        (
            profile(r#"":root"="read","store"="deny""#),
            0,
            Unisolated::Unstored,
        ),
        (
            profile(r#"":root"="read","/store"="deny","/store"="deny""#),
            0,
            Unisolated::Classless,
        ),
        (
            profile(r#"":root"="read",":root"="read","/store"="deny""#),
            0,
            Unisolated::Classless,
        ),
        (
            profile(r#"":root"="read","/etc"="read","/store"="deny""#),
            0,
            Unisolated::Classless,
        ),
        (
            profile(r#""/w"="write","/store"="deny""#),
            0,
            Unisolated::Classless,
        ),
    ]);
    let unhanded = |workdir| Bound {
        workdir,
        store: None,
    };
    for bound in [unhanded("/w"), Bound::default()] {
        refuses_bound(
            bound,
            vec![
                (profile(READ_ONLY), 0, Unisolated::Unhanded),
                (profile(WORKSPACE_WRITE), 0, Unisolated::Unhanded),
            ],
        );
    }
    let elsewhere = Bound {
        workdir: "/elsewhere",
        ..BOUND
    };
    refuses_bound(
        elsewhere,
        vec![(profile(WORKSPACE_WRITE), 0, Unisolated::Unrooted)],
    );
    let moved = Bound {
        store: Some("/moved"),
        ..BOUND
    };
    refuses_bound(moved, vec![(profile(READ_ONLY), 0, Unisolated::Unstored)]);
}

/// 45.11 (U8i; operator ruling 4 of 2026-10-24): bound to a store two
/// folders inside the root, the workspace-write class renders both folders
/// written, from the root outwards, and reads back only with both: one
/// missing either refuses with its own cause, a read-only profile writing
/// them or a write of any other folder refuses as any other written path
/// does, and a folder named twice renders no class. The read-only class
/// renders none.
#[test]
fn a_profile_reads_as_workspace_write_only_with_every_folder_to_an_inside_store_written() {
    let profile = |entries: &str| paired(&definition(entries));
    let deep = Bound {
        workdir: "/w",
        store: Some("/w/a/b/store"),
    };
    let (root, deny) = (r#"":root"="read","/w"="write""#, r#""/w/a/b/store"="deny""#);
    let (outer, inner) = (r#""/w/a"="write""#, r#""/w/a/b"="write""#);
    let pinned = format!("{root},{outer},{inner},{deny}");
    let rendered = grammar::rendered("workspace-write", deep);
    assert_eq!(rendered, Some(profile(&pinned)));
    let read_only = format!(r#"":root"="read",{deny}"#);
    assert_eq!(
        grammar::rendered("read-only", deep),
        Some(profile(&read_only))
    );
    for (entries, class) in [
        (&pinned, SandboxIntent::WorkspaceWrite),
        (
            &format!("{root},{inner},{outer},{deny}"),
            SandboxIntent::WorkspaceWrite,
        ),
        (&read_only, SandboxIntent::ReadOnly),
    ] {
        for (command, _) in shapes(&profile(entries)) {
            assert_eq!(grammar::profile(&placed(&command), deep), Ok(Some(class)));
        }
    }
    let other = r#""/w/a/c"="write""#;
    refuses_bound(
        deep,
        vec![
            (profile(&format!("{root},{deny}")), 0, Unisolated::Unpinned),
            (
                profile(&format!("{root},{inner},{deny}")),
                0,
                Unisolated::Unpinned,
            ),
            (
                profile(&format!("{root},{outer},{deny}")),
                0,
                Unisolated::Unpinned,
            ),
            (
                profile(&format!(r#"":root"="read",{outer},{inner},{deny}"#)),
                0,
                Unisolated::Unrooted,
            ),
            (
                profile(&format!("{root},{outer},{inner},{other},{deny}")),
                0,
                Unisolated::Unrooted,
            ),
            (
                profile(&format!("{root},{outer},{outer},{inner},{deny}")),
                0,
                Unisolated::Classless,
            ),
        ],
    );
}

/// 45.11: only a store strictly inside the root pins a folder, and only one
/// its paths place, inside or outside, renders a workspace-write class. A
/// store directly in the root and one in a folder whose name only begins
/// with the root's render the root alone and read back with it alone. A
/// store the paths cannot place, one through a `..`, through a doubled
/// separator or relative, or beside a root that is relative or spelled
/// with a trailing separator, renders the root alone and is refused, never
/// taken to lie outside; the store that is the root names one path twice.
/// The read-only class reads back beside each.
#[test]
fn a_store_outside_the_root_pins_nothing_and_one_its_paths_cannot_place_refuses() {
    let write = Ok(Some(SandboxIntent::WorkspaceWrite));
    let unplaced = Err(Unisolated::Unpinned(5));
    for (workdir, store, read) in [
        ("/w", "/w/store", write),
        ("/w", "/w2/a/store", write),
        ("/w", "/w/../a/store", unplaced),
        ("/w", "/w//a/store", unplaced),
        ("/w", "w/a/store", unplaced),
        ("w", "w/a/store", unplaced),
        ("/w/", "/w/a/store", unplaced),
        ("/w", "/w", Err(Unisolated::Classless(5))),
    ] {
        let bound = Bound {
            workdir,
            store: Some(store),
        };
        let entries = format!(r#"":root"="read","{workdir}"="write","{store}"="deny""#);
        let alone = paired(&definition(&entries));
        let rendered = grammar::rendered("workspace-write", bound);
        assert_eq!(rendered.as_ref(), Some(&alone), "{workdir} {store}");
        let [(command, _), _] = shapes(&alone);
        assert_eq!(grammar::profile(&placed(&command), bound), read, "{store}");
        let read_only = grammar::rendered("read-only", bound).unwrap();
        let [(command, _), _] = shapes(&read_only);
        let class = grammar::profile(&placed(&command), bound);
        assert_eq!(
            class,
            Ok(Some(SandboxIntent::ReadOnly)),
            "{workdir} {store}"
        );
    }
}

/// 45.11 (U8h's canonical store, the macOS shape on Linux): a root spelled
/// through a link in a canonical temporary folder holds the store kept at
/// its canonical place, so the folder between them is rendered at the
/// store's spelling and required; the root as spelled holds a store spelled
/// through it the same way. A store that is the root's canonical form
/// cannot be placed, and refuses.
#[test]
fn a_root_spelled_through_a_link_pins_the_folder_of_the_canonical_store_inside_it() {
    let dir = tempfile::tempdir().unwrap();
    let canonical = std::fs::canonicalize(dir.path()).unwrap();
    std::fs::create_dir(canonical.join("real")).unwrap();
    std::os::unix::fs::symlink(canonical.join("real"), canonical.join("link")).unwrap();
    let text = |path: std::path::PathBuf| path.display().to_string();
    let root = text(canonical.join("link"));
    for folder in [
        text(canonical.join("real/.forge")),
        format!("{root}/.forge"),
    ] {
        let store = format!("{folder}/store");
        let bound = Bound {
            workdir: &root,
            store: Some(&store),
        };
        let written = format!(r#"":root"="read","{root}"="write""#);
        let (pin, deny) = (
            format!(r#""{folder}"="write""#),
            format!(r#""{store}"="deny""#),
        );
        let pinned = paired(&definition(&format!("{written},{pin},{deny}")));
        assert_eq!(grammar::rendered("workspace-write", bound), Some(pinned));
        let unpinned = paired(&definition(&format!("{written},{deny}")));
        refuses_bound(bound, vec![(unpinned, 0, Unisolated::Unpinned)]);
    }
    // A store that is the root's canonical form is no store inside it.
    let real = text(canonical.join("real"));
    let bound = Bound {
        workdir: &root,
        store: Some(&real),
    };
    let entries = format!(r#"":root"="read","{root}"="write","{real}"="deny""#);
    let alone = paired(&definition(&entries));
    refuses_bound(bound, vec![(alone, 0, Unisolated::Unpinned)]);
}

/// 45.3: any `--enable`, `--disable` of another feature, a dotted profile
/// key, any other permissions value, an access outside the measured three,
/// a profile that renders no class and one that does not deny the store
/// refuse.
#[test]
fn every_other_toggle_and_profile_refuses_with_its_cause() {
    let profile = |entries: &str| paired(&definition(entries));
    let quoted = definition(READ_ONLY).replacen("permissions", "\"permissions\"", 1);
    let spaced = definition(READ_ONLY).replacen('=', " =", 1);
    let padded = definition(READ_ONLY).replacen('=', "= ", 1);
    refuses(vec![
        (argv(&["--enable", "apps"]), 0, Unisolated::Enabled),
        (argv(&["--enable=plugins"]), 0, Unisolated::Enabled),
        (argv(&["--disable", "shell_tool"]), 0, Unisolated::Disabled),
        (argv(&["--disable=Apps"]), 0, Unisolated::Disabled),
        (
            paired(r#"permissions.brokkr.filesystem={}"#),
            0,
            Unisolated::Dotted,
        ),
        (
            paired(r#"permissions={brokkr={}}"#),
            0,
            Unisolated::Unprofiled,
        ),
        (paired(&quoted), 0, Unisolated::Unprofiled),
        (paired(&spaced), 0, Unisolated::Unprofiled),
        (paired(&padded), 0, Unisolated::Unprofiled),
        (
            paired(r#"permissions.brokkr={network={":root"="read"}}"#),
            0,
            Unisolated::Unprofiled,
        ),
        (
            profile(r#"":root"="read", "/store"="deny""#),
            0,
            Unisolated::Unprofiled,
        ),
        (
            profile(r#"":root"=read,"/store"="deny""#),
            0,
            Unisolated::Unprofiled,
        ),
        (
            profile(r#"":root"="read","/a\"b"="deny""#),
            0,
            Unisolated::Unprofiled,
        ),
        (
            profile(r#"":root"="read",""="deny""#),
            0,
            Unisolated::Unprofiled,
        ),
        (
            profile(r#"":root"="read","/a\b"="deny""#),
            0,
            Unisolated::Unprofiled,
        ),
        (
            profile("\":root\"=\"read\",\"/a\tb\"=\"deny\""),
            0,
            Unisolated::Unprofiled,
        ),
        (profile(r#"":root""read""#), 0, Unisolated::Unprofiled),
        (
            paired(r#"permissions.brokkr={filesystem={":root"="read"}}}"#),
            0,
            Unisolated::Unprofiled,
        ),
        (
            profile(r#"":root"="read","/store"="none""#),
            0,
            Unisolated::Access,
        ),
        (
            profile(r#""/w"="read","/store"="deny""#),
            0,
            Unisolated::Classless,
        ),
        (
            profile(r#"":root"="read","/w"="write""#),
            0,
            Unisolated::Undenied,
        ),
    ]);
}

/// 45.3: any other selector, a selector before its profile, a second
/// profile or selector, a profile nothing selects and a profile beside
/// every spelling of `--sandbox` and `sandbox_mode` refuse.
#[test]
fn every_other_selector_and_a_profile_beside_a_sandbox_refuse() {
    let defined = argv(&["-c", &definition(READ_ONLY)]);
    let selected = paired(&definition(READ_ONLY));
    let other = definition(READ_ONLY).replacen("brokkr", "other", 1);
    let beside = |sandbox: &[&str]| [selected.clone(), argv(sandbox)].concat();
    let selecting = |selector: &str| [defined.clone(), argv(&["-c", selector])].concat();
    refuses(vec![
        (
            selecting("default_permissions=\"other\""),
            2,
            Unisolated::Selector,
        ),
        (
            selecting("default_permissions=\":read-only\""),
            2,
            Unisolated::Selector,
        ),
        (
            selecting("default_permissions=brokkr"),
            2,
            Unisolated::Selector,
        ),
        (
            [argv(&["-c", SELECTOR]), defined.clone()].concat(),
            0,
            Unisolated::Selector,
        ),
        (
            [selected.clone(), argv(&["-c", &other])].concat(),
            4,
            Unisolated::Twice,
        ),
        (
            [selected.clone(), argv(&["-c", SELECTOR])].concat(),
            4,
            Unisolated::Twice,
        ),
        (defined.clone(), 0, Unisolated::Unselected),
        (beside(&["--sandbox", "read-only"]), 4, Unisolated::Beside),
        (beside(&["-s", "read-only"]), 4, Unisolated::Beside),
        (beside(&["-sread-only"]), 4, Unisolated::Beside),
        (beside(&["--sandbox=read-only"]), 4, Unisolated::Beside),
        (
            beside(&["-csandbox_mode=\"read-only\""]),
            4,
            Unisolated::Beside,
        ),
        (
            [argv(&["-c", "sandbox_mode=\"read-only\""]), selected].concat(),
            0,
            Unisolated::Beside,
        ),
    ]);
}

/// Each isolation fault's words, written out by hand: this module's one
/// pin of each, and the final check's whole rendering of one of them.
#[test]
fn each_isolation_fault_reads_in_its_own_words() {
    for (fault, words) in [
        (
            Unisolated::Enabled(3),
            "carries '--enable' (argument 3), which switches a feature on, and no feature is \
             switched on",
        ),
        (
            Unisolated::Disabled(3),
            "carries '--disable' (argument 3) of a feature other than 'apps' or 'plugins', the \
             two it was measured to disable",
        ),
        (
            Unisolated::Dotted(3),
            "carries '--config' (argument 3) under a permission profile through a dotted key, \
             where the profile is one whole table assigned at once",
        ),
        (
            Unisolated::Unprofiled(3),
            "carries '--config' (argument 3) into the 'permissions' configuration with something \
             other than one named profile of one filesystem table, written canonically",
        ),
        (
            Unisolated::Access(3),
            "carries '--config' (argument 3), a permission profile granting a filesystem access \
             other than 'read', 'write' or 'deny'",
        ),
        (
            Unisolated::Unhanded(3),
            "carries '--config' (argument 3), a permission profile, where no secrets store's path \
             is handed beside the workdir to bind it to, so no profile is admitted",
        ),
        (
            Unisolated::Classless(3),
            "carries '--config' (argument 3), a permission profile that renders no sandbox class: \
             ':root' is read once, no other path is read, and no path is named twice",
        ),
        (
            Unisolated::Unrooted(3),
            "carries '--config' (argument 3), a permission profile that writes a path other than \
             the seat's workspace root",
        ),
        (
            Unisolated::Unpinned(3),
            "carries '--config' (argument 3), a permission profile writing the workspace root \
             without every folder to the secrets store inside it, or beside a store it cannot \
             place",
        ),
        (
            Unisolated::Unstored(3),
            "carries '--config' (argument 3), a permission profile that denies a path other than \
             the workspace's secrets store",
        ),
        (
            Unisolated::Undenied(3),
            "carries '--config' (argument 3), a permission profile that does not deny the \
             workspace's secrets store, so it stays readable",
        ),
        (
            Unisolated::Selector(3),
            "carries '--config' (argument 3) into the 'default_permissions' configuration with \
             something other than the quoted name of the one profile defined before it",
        ),
        (
            Unisolated::Twice(3),
            "carries a second permission profile or selector (argument 3); a command runs one",
        ),
        (
            Unisolated::Unselected(3),
            "defines a permission profile (argument 3) that no 'default_permissions' selects",
        ),
        (
            Unisolated::Beside(3),
            "carries '--sandbox' or a 'sandbox_mode' assignment (argument 3) beside a permission \
             profile, which on codex-cli 0.162.1 it silently overrides",
        ),
    ] {
        assert_eq!(fault.to_string(), words);
    }
    let sealed = profiled_site(SandboxIntent::ReadOnly, READ_ONLY);
    let sandboxed = [served_cold(READ_ONLY), argv(&["-s", "read-only"])].concat();
    let cause = format!("cannot be read: it {}", Unisolated::Beside(22));
    let refusal = Err(final_refusal("codex", &cause));
    assert_eq!(stored(&sealed, sandboxed), refusal);
}

/// R2 of U9c1's second review (decision 0071 ruling 8): what a command's
/// state cannot be read for is typed, each variant read at its argument
/// and its words written out by hand once; an isolation fault keeps its
/// own words.
#[test]
fn each_unread_state_is_a_typed_fault_in_its_own_words() {
    let enable = Unisolated::Enabled(5);
    for (harness, options, fault, words) in [
        (
            "codex",
            argv(&["--sandbox", "nowhere"]),
            Unread::Classless("--sandbox", 5),
            "carries '--sandbox' (argument 5), whose value names no sandbox class: read-only, \
             workspace-write or danger-full-access",
        ),
        (
            "codex",
            argv(&["-s", "read-only", "-c", "sandbox_mode=\"read-only\""]),
            Unread::Reclassed(7),
            "expresses the sandbox class a second time (argument 7)",
        ),
        (
            "codex",
            argv(&["-c", "unassigned"]),
            Unread::Unbounded(5, "is not a KEY=VALUE configuration assignment"),
            "carries '--config' (argument 5), whose value is not a KEY=VALUE configuration \
             assignment",
        ),
        (
            "codex",
            argv(&["-c", "sandbox_mode=\"nowhere\""]),
            Unread::Unestablished(5, "sandbox_mode"),
            "carries '--config' (argument 5) into the 'sandbox_mode' configuration in a spelling \
             or with a value whose meaning is not established, so it is read as neither ON nor OFF",
        ),
        ("codex", argv(&["--enable", "apps"]), enable.into(), ""),
        (
            "claude",
            argv(&["--session-id", "session-1"]),
            Unread::Selector("--session-id", 1),
            "carries '--session-id' (argument 1), a session selector other than a rejoin's",
        ),
        (
            "claude",
            argv(&["--allowedTools", "Read,,Edit"]),
            Unread::Listed(
                "--allowedTools",
                1,
                "joins an empty pattern: a doubled, leading or trailing separator",
            ),
            "carries '--allowedTools' (argument 1), whose value joins an empty pattern: a \
             doubled, leading or trailing separator",
        ),
    ] {
        // Codex's options follow its cold lead; Claude's stand alone.
        let [(cold, _), _] = shapes(&options);
        let command = if harness == "codex" {
            cold
        } else {
            options.clone()
        };
        let parsed = grammar::parse_final(harness, &command).expect("modelled");
        let read = expressed(&parsed.expect("placed").command, BOUND);
        assert_eq!(read.map(|_| ()), Err(fault), "{options:?}");
        let words = if words.is_empty() {
            enable.to_string()
        } else {
            words.to_string()
        };
        assert_eq!(fault.to_string(), words);
    }
}

/// Rebuild unit 13-fix-c, R1, moved here from the parent suite: the Codex
/// hands carry the class and each of the server's three bindings, the
/// class by `--sandbox` or, since U9c1, by the engine's permission profile
/// in its place, judged bound to the handed paths; a profile that is not
/// bound to them carries none, and the composer, handed no store's path,
/// reads every profile as carrying none (operator ruling 4 of 2026-10-23).
#[test]
fn codex_hands_carry_the_class_by_sandbox_or_profile_and_three_bindings() {
    let in_box = |count: usize, controls: &Controls| Controls {
        provenance: typed(count, &[]),
        ..controls.clone()
    };
    let codex = codex_hands(&CODEX_HANDS);
    let compose = |hands: &[String]| {
        let controls = in_box(hands.len(), &codex.controls);
        compose_for_provider("codex", &codex.authored, hands, &controls).map(|_| ())
    };
    let transported = |hands: &[String], bound: Bound<'_>| {
        let hands = grammar::parse("codex", hands).expect("modelled");
        untransported("codex", &hands.expect("a parsed fragment"), bound)
    };
    let (command, args, approval) = (
        "server command binding ('mcp_servers.brokkr.command')",
        "server arguments binding ('mcp_servers.brokkr.args')",
        "server approval binding ('mcp_servers.brokkr.default_tools_approval_mode')",
    );
    let profiled = [rendered(READ_ONLY), argv(&CODEX_HANDS[2..])].concat();
    assert_eq!(compose(&argv(&CODEX_HANDS)), Ok(()));
    let unclassed = unhanded("codex", profiled.len(), UNCLASSED);
    assert_eq!(compose(&profiled), Err(unclassed));
    for (dropped, what) in [
        (0..2, UNCLASSED),
        (2..4, command),
        (4..6, args),
        (6..8, approval),
    ] {
        let mut hands = argv(&CODEX_HANDS);
        hands.drain(dropped);
        let unhanded = unhanded("codex", hands.len(), what);
        assert_eq!(compose(&hands), Err(unhanded), "{what}");
    }
    assert_eq!(transported(&profiled, BOUND), None);
    assert_eq!(transported(&profiled, Bound::default()), Some(UNCLASSED));
    let written = [rendered(WORKSPACE_WRITE), argv(&CODEX_HANDS[2..])].concat();
    assert_eq!(transported(&written, BOUND), None);
    let elsewhere = Bound {
        workdir: "/elsewhere",
        ..BOUND
    };
    let moved = Bound {
        store: Some("/moved"),
        ..BOUND
    };
    for bound in [elsewhere, moved] {
        assert_eq!(transported(&written, bound), Some(UNCLASSED), "{bound:?}");
    }
    for (dropped, what) in [
        (0..9, UNCLASSED),
        (5..9, UNCLASSED),
        (7..9, UNCLASSED),
        (9..11, command),
        (11..13, args),
        (13..15, approval),
    ] {
        let mut hands = profiled.clone();
        hands.drain(dropped);
        assert_eq!(transported(&hands, BOUND), Some(what), "{what}");
    }
    for entries in [
        r#"":root"="read""#,
        r#"":root"="read","/nonexistent"="deny""#,
        r#"":root"="read","/"="write","/store"="deny""#,
    ] {
        let unbound = [paired(&definition(entries)), argv(&CODEX_HANDS[2..])].concat();
        assert_eq!(transported(&unbound, BOUND), Some(UNCLASSED), "{entries}");
    }
}

/// SI2: authored isolation options and profiles keep slice one's refusal;
/// only the engine composes them.
#[test]
fn authored_isolation_options_and_profiles_are_refused() {
    let profile = definition(READ_ONLY);
    for (written, option) in [
        (argv(&["--ignore-user-config"]), "--ignore-user-config"),
        (argv(&["--disable", "apps"]), "--disable"),
        (argv(&["--enable=apps"]), "--enable"),
        (argv(&["-c", &profile]), "--config"),
        (vec![format!("-c{SELECTOR}")], "--config"),
    ] {
        let refusal = authored_refused("codex", option, 1, BEARS);
        assert_eq!(
            authored_refusal("codex", &written),
            Err(refusal),
            "{written:?}"
        );
    }
}

/// The feature-toggle shorthand builds what `--enable` and `--disable`
/// are: one value, split or equals-joined, repeatable, and a control of the
/// power it names. The tables call it only in constant evaluation, so this
/// is its one run.
#[test]
fn the_feature_shorthand_builds_a_repeatable_control_of_its_power() {
    use grammar::{feature, Arity, Power};
    let toggle = feature("--w", Power::Mcp);
    assert_eq!(toggle.canonical, "--w");
    assert!(toggle.aliases.is_empty());
    assert_eq!(toggle.arity, Arity::One);
    assert!(toggle.equals && !toggle.attached && toggle.repeat);
    assert_eq!(toggle.effect, Effect::Control(Power::Mcp));
}
