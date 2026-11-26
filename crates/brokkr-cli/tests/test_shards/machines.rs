//! The machine layouts (operator rulings 2026-11-01 to 2026-11-23): each
//! system's machines, and the coverage gate's, run their shards alone, one
//! after another, a machine named in no layout or in no known form is
//! refused, and ci.yml's test and coverage jobs run exactly the layouts'
//! machines.

use super::*;

/// Each machine of each system's layout, and the shards it runs, by their
/// place in [`NAMES`]: every shard sits on exactly one machine of each.
const MACHINES: [(&str, &[usize]); 13] = [
    ("linux/1", &[0]),
    ("linux/2", &[10, 14]),
    ("linux/3", &[8, 9]),
    ("linux/4", &[6, 7]),
    ("linux/5", &[2, 15]),
    ("linux/6", &[5, 12]),
    ("linux/7", &[3, 4, 13]),
    ("linux/8", &[1, 11]),
    ("macos/1", &[0, 4, 10, 15]),
    ("macos/2", &[1, 3]),
    ("macos/3", &[5, 7]),
    ("macos/4", &[2, 8]),
    ("macos/5", &[6, 9]),
];

/// The exact coverage gate's machines, Linux's alone, and the shards each
/// measures, by their place in [`NAMES`]: every shard but the doctests,
/// which cargo-llvm-cov never runs, sits on exactly one.
const COVERAGE_MACHINES: [(&str, &[usize]); 8] = [
    ("linux/1", &[14]),
    ("linux/2", &[2]),
    ("linux/3", &[0]),
    ("linux/4", &[6, 9]),
    ("linux/5", &[1, 8]),
    ("linux/6", &[5, 15]),
    ("linux/7", &[7, 11, 12]),
    ("linux/8", &[3, 4, 13]),
];

/// A run whose BROKKR_TEST_MACHINE names a machine builds every binary and
/// runs that machine's shards alone, one after another in plan order, and
/// no shard's command inherits the variable: the test mode by the test
/// layout, the coverage mode by the coverage layout.
#[test]
fn a_machine_runs_only_its_shards_one_after_another_in_plan_order() {
    let real = metadata();
    let [(test, test_prefix, test_build), (coverage, coverage_prefix, coverage_build)] = MODES;
    for (args, prefix, build, layout, planned) in [
        (test, test_prefix, test_build, &MACHINES[..], NAMES.len()),
        (
            coverage,
            coverage_prefix,
            coverage_build,
            &COVERAGE_MACHINES[..],
            NAMES.len() - 1,
        ),
    ] {
        for &(machine, indices) in layout {
            let shards: Vec<&str> = indices.iter().map(|&at| SHARDS[at]).collect();
            let scratch = Scratch::new(&real, &script());
            let run = scratch.run(args, &[("BROKKR_TEST_MACHINE", machine)]);
            assert_eq!(run.status.code(), Some(0), "{machine} {args:?}: {run:?}");
            assert_eq!(
                scratch.events(),
                one_after_another(prefix, &shards),
                "{machine} {args:?}"
            );
            assert_eq!(
                scratch.calls(),
                build_then(prefix, build, &shards),
                "{machine} {args:?}"
            );
            assert_eq!(
                scratch.lines("machine"),
                vec!["unset"; shards.len()],
                "{machine} {args:?}: a shard inherited the machine"
            );
            let stdout = String::from_utf8_lossy(&run.stdout);
            assert!(
                stdout.contains(&format!(
                    "test shards: machine {machine} runs {} of the {planned} shards\n",
                    shards.len(),
                )),
                "{stdout}"
            );
        }
    }
}

/// Every shard a system compiles sits on exactly one of its machines: the
/// broker's enabled parts on Linux's alone, in both of its layouts, and
/// the doctests on no coverage machine.
#[test]
fn each_layout_places_every_shard_its_system_runs_once() {
    for (layout, system, never) in [
        (&MACHINES[..], "linux", &[][..]),
        (&MACHINES[..], "macos", &LINUX_ONLY[..]),
        (&COVERAGE_MACHINES[..], "linux", &[DOCTESTS][..]),
    ] {
        let mut placed: Vec<usize> = layout
            .iter()
            .filter(|(machine, _)| machine.starts_with(system))
            .flat_map(|(_, indices)| indices.iter().copied())
            .collect();
        placed.sort_unstable();
        let compiled: Vec<usize> = (0..NAMES.len()).filter(|at| !never.contains(at)).collect();
        assert_eq!(
            placed, compiled,
            "{system} runs every shard it compiles once"
        );
    }
    assert!(COVERAGE_MACHINES
        .iter()
        .all(|(machine, _)| machine.starts_with("linux/")));
}

/// A machine named in a form the layouts do not use, or one no shard runs
/// on, is refused before anything is built or run.
#[test]
fn a_machine_no_shard_runs_on_or_one_not_named_so_is_refused() {
    let real = metadata();
    for (machine, refusal) in [
        (
            "linux/9",
            "test shards: BROKKR_TEST_MACHINE names linux/9, which runs no shard\n",
        ),
        (
            "macos/6",
            "test shards: BROKKR_TEST_MACHINE names macos/6, which runs no shard\n",
        ),
        (
            "windows/1",
            "test shards: BROKKR_TEST_MACHINE 'windows/1' is not linux/<n> or macos/<n>\n",
        ),
        (
            "linux/0",
            "test shards: BROKKR_TEST_MACHINE 'linux/0' is not linux/<n> or macos/<n>\n",
        ),
        (
            "linux/1 ",
            "test shards: BROKKR_TEST_MACHINE 'linux/1 ' is not linux/<n> or macos/<n>\n",
        ),
    ] {
        let scratch = Scratch::new(&real, &script());
        let run = scratch.run(&["test"], &[("BROKKR_TEST_MACHINE", machine)]);
        assert_eq!(run.status.code(), Some(1), "{machine:?}: {run:?}");
        assert_eq!(String::from_utf8_lossy(&run.stderr), refusal, "{machine:?}");
        assert_eq!(scratch.calls(), Vec::<String>::new(), "{machine:?}");
    }
}

/// The modes that read the test layout, and those that read the coverage
/// layout, each before anything is built or run.
const TEST_MODES: [&[&str]; 3] = [&["test"], &["plan", "test"], &["machines", "test"]];
const COVERAGE_MODES: [&[&str]; 3] = [
    &["coverage", "nightly-2026-09-05"],
    &["plan", "coverage"],
    &["machines", "coverage"],
];

/// A layout that leaves a machine empty, or names a machine by anything
/// but a number from 1, is refused by name in every mode that reads it,
/// before anything is built or run: a test machine in the test modes, a
/// coverage machine in the coverage modes.
#[test]
fn a_layout_with_an_empty_machine_or_an_unnumbered_one_is_refused() {
    let real = metadata();
    let table = script();
    // The libraries' rest is linux/1's one shard in the test layout, the
    // broker's enabled rest coverage's linux/1's, and the broker's skipped
    // suite's line names a macOS machine.
    let libraries = "skipped' 1 3 1 lib";
    let rest = "the rest of it, alone\" 2 1 - test:it";
    let line = "skipped, alone\" 5 6 1 test:it";
    for (line, planted, refusal, modes) in [
        (
            libraries,
            "skipped' 9 3 1 lib",
            "test shards: the linux layout leaves machine 1 with no shard\n",
            TEST_MODES,
        ),
        (
            rest,
            "the rest of it, alone\" 2 9 - test:it",
            "test shards: the linux layout leaves machine 1 with no shard\n",
            COVERAGE_MODES,
        ),
        (
            line,
            "skipped, alone\" 5 6 x test:it",
            "test shards: capability_broker, its enabled suite skipped, alone names the macos \
             machine 'x', which is not a number from 1\n",
            TEST_MODES,
        ),
    ] {
        assert_eq!(table.matches(line).count(), 1, "{line}");
        let scratch = Scratch::new(&real, &table.replace(line, planted));
        for args in modes {
            let run = scratch.run(args, &[]);
            assert_eq!(run.status.code(), Some(1), "{planted} {args:?}: {run:?}");
            assert_eq!(
                String::from_utf8_lossy(&run.stderr),
                refusal,
                "{planted} {args:?}"
            );
        }
        assert_eq!(scratch.calls(), Vec::<String>::new(), "{planted}");
    }
}

/// What `machines <mode>` prints for `layout`: each machine and the names
/// of its shards.
fn printed(layout: &[(&str, &[usize])]) -> String {
    layout
        .iter()
        .map(|(machine, indices)| {
            let names: Vec<&str> = indices.iter().map(|&at| NAMES[at]).collect();
            format!("{machine}: {}\n", names.join(" | "))
        })
        .collect()
}

/// Parsed `matrix.machine` list of `id` in `workflow`.
fn matrix_machines(workflow: &str, file: &str, id: &str) -> Vec<String> {
    let wf =
        crate::workflow::Workflow::parse(workflow).unwrap_or_else(|e| panic!("{file} parses: {e}"));
    wf.require_job(file, id)
        .unwrap()
        .matrix
        .as_ref()
        .and_then(|m| m.axis_strings("machine"))
        .unwrap_or_else(|| panic!("{file}'s {id} has no matrix.machine list"))
        .into_iter()
        .map(str::to_string)
        .collect()
}

/// The test and coverage jobs run exactly the layouts' machines: ci.yml's
/// `engine-machines` matrix names each test machine the script lays out, in
/// order, each on its own system's runner, `coverage-machines` names each
/// coverage machine in order, and the script prints each machine's shards
/// as [`MACHINES`] and [`COVERAGE_MACHINES`] hold them; the coverage mode's
/// macOS machines are the test layout's, without the doctests.
#[test]
fn the_test_jobs_machines_are_the_layouts() {
    let macos: Vec<(&str, Vec<usize>)> = MACHINES
        .iter()
        .filter(|(machine, _)| machine.starts_with("macos/"))
        .map(|(machine, indices)| {
            (
                *machine,
                indices
                    .iter()
                    .copied()
                    .filter(|&at| at != DOCTESTS)
                    .collect(),
            )
        })
        .collect();
    let macos: Vec<(&str, &[usize])> = macos
        .iter()
        .map(|(machine, at)| (*machine, at.as_slice()))
        .collect();
    for (mode, expected) in [
        ("test", printed(&MACHINES)),
        (
            "coverage",
            printed(&[&COVERAGE_MACHINES[..], &macos].concat()),
        ),
    ] {
        let run = Command::new("bash")
            .args([SCRIPT, "machines", mode])
            .current_dir(workspace())
            .env_remove("BROKKR_TEST_MACHINE")
            .output()
            .expect("the machines print");
        assert_eq!(run.status.code(), Some(0), "{mode}: {run:?}");
        assert_eq!(String::from_utf8_lossy(&run.stdout), expected, "{mode}");
    }

    let workflow =
        std::fs::read_to_string(workspace().join(".github/workflows/ci.yml")).expect("ci.yml");
    let coverage: Vec<&str> = COVERAGE_MACHINES
        .iter()
        .map(|(machine, _)| *machine)
        .collect();
    assert_eq!(
        matrix_machines(&workflow, "ci.yml", "coverage-machines"),
        coverage,
        "coverage-machines lists {coverage:?}"
    );
    let linux: Vec<&str> = MACHINES
        .iter()
        .map(|(machine, _)| *machine)
        .filter(|machine| machine.starts_with("linux/"))
        .collect();
    assert_eq!(
        matrix_machines(&workflow, "ci.yml", "engine-machines"),
        linux,
        "ci.yml's engine-machines lists {linux:?}"
    );
    let macos_workflow =
        std::fs::read_to_string(workspace().join(".github/workflows/macos-test-shards.yml"))
            .expect("macos-test-shards.yml");
    let macos_machines: Vec<&str> = MACHINES
        .iter()
        .map(|(machine, _)| *machine)
        .filter(|machine| machine.starts_with("macos/"))
        .collect();
    assert_eq!(
        matrix_machines(&macos_workflow, "macos-test-shards.yml", "macos-machines"),
        macos_machines,
        "macos-test-shards.yml's macos-machines lists {macos_machines:?}"
    );
}

/// A run on a macOS host, naming no machine, runs every shard but the
/// four macOS never compiles, the broker's enabled parts, which its layout
/// marks `-`; a Linux host runs them all.
#[test]
fn a_host_runs_every_shard_its_system_compiles() {
    let real = metadata();
    for (host, shards) in [
        ("Linux", SHARDS.to_vec()),
        (
            "Darwin",
            SHARDS
                .iter()
                .enumerate()
                .filter(|(at, _)| !LINUX_ONLY.contains(at))
                .map(|(_, shard)| *shard)
                .collect(),
        ),
    ] {
        let scratch = Scratch::new(&real, &script());
        let run = scratch.run(&["test"], &[("STUB_UNAME", host)]);
        assert_eq!(run.status.code(), Some(0), "{host}: {run:?}");
        assert_eq!(scratch.events(), one_after_another(TEST, &shards), "{host}");
    }
}
