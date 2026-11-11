//! The machine layouts (operator rulings 2026-11-01 to 2026-11-11): each
//! system's machines run their shards alone, one after another, a machine
//! named in no layout or in no known form is refused, and ci.yml's test job
//! runs exactly the layouts' machines.

use super::*;

/// Each machine of each system's layout, and the shards it runs, by their
/// place in [`NAMES`]: every shard sits on exactly one machine of each.
const MACHINES: [(&str, &[usize]); 9] = [
    ("linux/1", &[0, 1]),
    ("linux/2", &[2, 3, 4, 7]),
    ("linux/3", &[5, 6]),
    ("linux/4", &[8]),
    ("linux/5", &[9]),
    ("macos/1", &[0, 7, 9]),
    ("macos/2", &[1, 3]),
    ("macos/3", &[2, 5]),
    ("macos/4", &[4, 6]),
];

/// A run whose BROKKR_TEST_MACHINE names a machine builds every binary and
/// runs that machine's shards alone, one after another in plan order, and
/// no shard's command inherits the variable.
#[test]
fn a_machine_runs_only_its_shards_one_after_another_in_plan_order() {
    let real = metadata();
    for (machine, indices) in MACHINES {
        let shards: Vec<&str> = indices.iter().map(|&at| SHARDS[at]).collect();
        let scratch = Scratch::new(&real, &script());
        let run = scratch.run(&["test"], &[("BROKKR_TEST_MACHINE", machine)]);
        assert_eq!(run.status.code(), Some(0), "{machine}: {run:?}");
        assert_eq!(
            scratch.events(),
            one_after_another(TEST, &shards),
            "{machine}"
        );
        let calls: Vec<String> = ["--no-run"]
            .iter()
            .chain(&shards)
            .map(|shard| format!("{TEST} {shard}"))
            .collect();
        assert_eq!(scratch.calls(), calls, "{machine}");
        assert_eq!(
            scratch.lines("machine"),
            vec!["unset"; shards.len()],
            "{machine}: a shard inherited the machine"
        );
        let stdout = String::from_utf8_lossy(&run.stdout);
        assert!(
            stdout.contains(&format!(
                "test shards: machine {machine} runs {} of the {} shards\n",
                shards.len(),
                NAMES.len()
            )),
            "{stdout}"
        );
    }
    for (system, never) in [("linux", None), ("macos", Some(LINUX_ONLY))] {
        let mut placed: Vec<usize> = MACHINES
            .iter()
            .filter(|(machine, _)| machine.starts_with(system))
            .flat_map(|(_, indices)| indices.iter().copied())
            .collect();
        placed.sort_unstable();
        let compiled: Vec<usize> = (0..NAMES.len()).filter(|&at| Some(at) != never).collect();
        assert_eq!(
            placed, compiled,
            "{system} runs every shard it compiles once"
        );
    }
}

/// A machine named in a form the layouts do not use, or one no shard runs
/// on, is refused before anything is built or run.
#[test]
fn a_machine_no_shard_runs_on_or_one_not_named_so_is_refused() {
    let real = metadata();
    for (machine, refusal) in [
        (
            "linux/6",
            "test shards: BROKKR_TEST_MACHINE names linux/6, which runs no shard\n",
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

/// A layout that leaves a machine empty, or names a machine by anything
/// but a number from 1, is refused by name in every mode, before anything
/// is built or run.
#[test]
fn a_layout_with_an_empty_machine_or_an_unnumbered_one_is_refused() {
    let real = metadata();
    let table = script();
    let line = "skipped, alone\" 5 1 test:it";
    assert_eq!(table.matches(line).count(), 1, "the last shard's line");
    for (planted, refusal) in [
        (
            "skipped, alone\" 6 1 test:it",
            "test shards: the linux layout leaves machine 5 with no shard\n",
        ),
        (
            "skipped, alone\" 5 x test:it",
            "test shards: capability_broker, its enabled suite skipped, alone names the macos \
             machine 'x', which is not a number from 1\n",
        ),
    ] {
        let scratch = Scratch::new(&real, &table.replace(line, planted));
        for args in [
            &["test"][..],
            &["plan", "test"][..],
            &["machines", "test"][..],
        ] {
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

/// The test job runs exactly the layouts' machines: ci.yml's
/// `engine-machines` matrix names each machine the script lays out, in
/// order, each on its own system's runner, and the script prints each
/// machine's shards as [`MACHINES`] holds them.
#[test]
fn the_test_jobs_machines_are_the_layouts() {
    let run = Command::new("bash")
        .args([SCRIPT, "machines", "test"])
        .current_dir(workspace())
        .env_remove("BROKKR_TEST_MACHINE")
        .output()
        .expect("the machines print");
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    let expected: String = MACHINES
        .iter()
        .map(|(machine, indices)| {
            let names: Vec<&str> = indices.iter().map(|&at| NAMES[at]).collect();
            format!("{machine}: {}\n", names.join(" | "))
        })
        .collect();
    assert_eq!(String::from_utf8_lossy(&run.stdout), expected);

    let workflow =
        std::fs::read_to_string(workspace().join(".github/workflows/ci.yml")).expect("ci.yml");
    let body = crate::workflow::jobs(&workflow)
        .into_iter()
        .find_map(|(id, body)| (id == "engine-machines").then_some(body))
        .expect("ci.yml's engine-machines job");
    let lines: Vec<&str> = body.lines().map(str::trim).collect();
    let listed: Vec<(&str, &str)> = lines
        .windows(2)
        .filter_map(|pair| {
            let os = pair[0].strip_prefix("- os: ")?;
            Some((os, pair[1].strip_prefix("machine: ")?))
        })
        .collect();
    let laid_out: Vec<(&str, &str)> = MACHINES
        .iter()
        .map(|(machine, _)| match machine.starts_with("linux/") {
            true => ("ubuntu-latest", *machine),
            false => ("macos-latest", *machine),
        })
        .collect();
    assert_eq!(listed, laid_out);
}

/// A run on a macOS host, naming no machine, runs every shard but the one
/// macOS never compiles, which its layout marks `-`; a Linux host runs them
/// all.
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
                .filter(|&(at, _)| at != LINUX_ONLY)
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
