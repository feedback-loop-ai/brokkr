//! Issue #365, asks 1 and 6: a sample a living doc shows is what the
//! command it shows prints, and the reforging ladder's tables are the
//! policy files they describe.
//!
//! Every fenced block in a living doc, read by `doc_truth`'s one reader,
//! that holds a `$ brokkr` prompt line is either [`SAMPLED`] or in a doc
//! [`ALLOWED`] names with its reason; a prompt in neither is refused, and
//! so is an allowance that covers nothing. A sampled block holds one
//! prompt, its first line, and every line after it is what the built
//! binary prints for that argv: run where its row says ([`Place`]), on the
//! stream it names ([`Stream`]), exiting as it expects ([`Exit`]). A
//! command that reads its input is fed the text its row names
//! ([`Sample::stdin`]) and the stream closes; any other sample's stdin
//! stays the null stream it has always had. A sample that reads an
//! earlier command's output names that command in [`Place::DrivenAfter`]
//! or [`Place::FolderAfter`], and the earlier argv runs first and must
//! exit zero. A prompt that names the run's file writes `<run>` where the
//! driven run's id goes, and the sampler fills it in. A
//! fixture's root reads in the doc as [`HOME`], the guides' convention.
//! `…` is the one wildcard, as trycmd reads it: a
//! line that is `…` alone stands for any number of whole lines, and a `…`
//! inside a line for any run of characters within it. Everything else
//! matches byte for byte, tabs included.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use serde::Deserialize;
use serde_json::Value;

use crate::bootstrap_bench::{can_box_the_gates, scaffold_with, Scaffold};
use crate::brokkr_folder::repository;
use crate::doc_truth::{readable_docs, Block, Doc};
use crate::init_stacks::{brokkr_with, repository_of};
use crate::machine_proof::copy_dir;
use crate::workspace_root::{read, workspace};

/// What opens a prompt line, after its indent and its blockquote.
const PROMPT: &str = "$ brokkr";

/// The wildcard: any run of whole lines, or of characters within one.
const ELIDED: &str = "…";

/// Docs whose `$ brokkr` prompts are shown but not sampled, one row per
/// doc, never per line, each with its reason. An allowance that covers
/// nothing fails, so none outlives its prompts.
const ALLOWED: [(&str, &str); 4] = [
    (
        "docs/guides/quickstart.md",
        "its recipe-shelf blocks (recipes list, and Try another recipe's variant add, list and \
         run) wait on #722, which changes what recipes list prints, and need a recipe source as \
         their fixture",
    ),
    (
        "docs/guides/adopting-a-node-repo.md",
        "its realms block reads a map the library route has the reader write by hand, and the \
         realm's head commit",
    ),
    (
        "docs/guides/read-surfaces.md",
        "its transcript blocks need a real model's session, its muninn block a Muninn seat \
         and a parked fleet, its operator supersede block and the inspect after it a \
         historical run's residual findings, and watch and ui are live views",
    ),
    (
        "docs/guides/provider-adapters.md",
        "doctor's provider lines are the operator's own providers and versions",
    ),
];

/// An earlier command a sample runs first: its argv, and what its stdin
/// receives, as [`Sample::stdin`] gives the sample's own command.
#[derive(Clone, Copy)]
struct Step {
    /// The argv, before `<run>` is filled.
    argv: &'static [&'static str],
    /// The stdin, or `None` for the null stream a sample gets by default.
    stdin: Option<&'static str>,
}

/// `argv` as an earlier command reading no stdin.
const fn step(argv: &'static [&'static str]) -> Step {
    Step { argv, stdin: None }
}

/// Where a sample's command runs.
#[derive(Clone, Copy)]
enum Place {
    /// The workspace root: this checkout, a recipe library.
    Workspace,
    /// The root of a fresh `git init` copy of the named init-stacks
    /// fixture, with no `.brokkr/` folder yet.
    Fixture(&'static str),
    /// The [`Place::Fixture`] after `init .`, run as `init_stacks` runs
    /// it: a fresh folder, as the quickstart's Step 2 leaves one.
    Folder(&'static str),
    /// The [`Place::Folder`] after each earlier [`Step`], in order, each
    /// run at the folder's root and required to exit zero. An earlier
    /// command that fails refuses the sample with its stderr, and the
    /// sample's own argv never runs.
    FolderAfter(&'static str, &'static [Step]),
    /// The root of a fresh `git init` repository holding a copy of each
    /// named file or directory of this checkout at the same path: a
    /// recipe library assembled by hand, as flow 3's long form builds one.
    Library(&'static [&'static str]),
    /// The root of the bootstrap bench's [`Scaffold`]: `init`'s folder,
    /// committed, over a `Makefile` whose targets pass, with the `claude`
    /// CLI stubbed through the adapter's own `BROKKR_CLAUDE_BIN`, so the
    /// scaffold's verify and ship gates run for real and its model seats
    /// answer at once, writing no code.
    Scaffold,
    /// The [`Place::Scaffold`] after [`RUN`], the guides' run, reached
    /// `done`.
    Driven,
    /// The [`Place::Driven`] after each earlier [`Step`], in order, each
    /// run at the scaffold's root and required to exit zero. An earlier
    /// command that fails refuses the sample with its stderr, and the
    /// sample's own argv never runs. A prompt that names the run's file
    /// writes `<run>` where the driven run's id goes; the sampler fills
    /// it in before the argv runs.
    DrivenAfter(&'static [Step]),
}

impl Place {
    /// Whether a sample here runs on the bootstrap bench's scaffold, whose
    /// gates must be boxed. Each place answers by name, so a place added
    /// later must say.
    fn on_the_scaffold(self) -> bool {
        match self {
            Place::Workspace
            | Place::Fixture(_)
            | Place::Folder(_)
            | Place::FolderAfter(_, _)
            | Place::Library(_) => false,
            Place::Scaffold | Place::Driven | Place::DrivenAfter(_) => true,
        }
    }
}

/// The run the guides drive, as Step 3 of the quickstart starts it.
const RUN: [&str; 3] = ["run", "--feature", "prefix selectors for the read surfaces"];

/// Which stream a sample's block shows.
#[derive(Clone, Copy)]
enum Stream {
    Stdout,
    /// `init` prints everything it says here.
    Stderr,
}

/// The exit a sample expects.
#[derive(Clone, Copy)]
enum Exit {
    /// Zero: anything else is a failure, with its stderr.
    Success,
    /// Whatever it is. The row says why the exit is not the sample's.
    Any,
    /// Non-zero: the sample shows a refusal, so a command that succeeds
    /// is not it.
    Refused,
}

/// A sampled block: the doc, the prompt line that opens the block, the
/// argv the prompt runs, and where it runs, what it shows and how it
/// exits.
#[derive(Clone, Copy)]
struct Sample {
    doc: &'static str,
    prompt: &'static str,
    argv: &'static [&'static str],
    place: Place,
    stream: Stream,
    exit: Exit,
    /// What the command's stdin receives, or `None` for the null stream
    /// [`brokkr_with`] gives it: `Command::output` closes a child's
    /// stdin at once. `Some` is written whole, then the stream closes.
    stdin: Option<&'static str>,
}

/// How a fixture's root reads in a doc.
const HOME: &str = "/home/you/my-app";

/// How this checkout reads in a prompt: where the guides clone Brokkr.
const CHECKOUT: &str = "~/src/brokkr";

/// The install the Node guides show, from a checkout at [`CHECKOUT`].
const ADD_NODE: [&str; 5] = [
    "recipes",
    "add",
    "~/src/brokkr/recipes/node",
    "--name",
    "node",
];

/// The adapter flow 3's library route copies first, and then Brokkr's
/// agent library, then its capability definitions.
const ADAPTER: &str = "adapters/claude.json";

/// The `export` both chained file samples read.
const EXPORT: [&str; 5] = ["export", "--run", "latest", "--out", "./out"];

/// The fake value the secrets guide's `set` reads from stdin: never a
/// real secret, and newline-terminated as a shell heredoc would give it.
const SECRET: &str = "example-not-a-secret\n";

/// The `secrets set` the guide's `list` and `remove` read, the fake value
/// arriving on stdin, never argv.
const SET: Step = Step {
    argv: &["secrets", "set", "GITHUB_TOKEN"],
    stdin: Some(SECRET),
};

/// A sample of the journal guide's evidence at a driven place, on
/// `stream`; every one succeeds.
const fn journal(
    prompt: &'static str,
    argv: &'static [&'static str],
    place: Place,
    stream: Stream,
) -> Sample {
    Sample {
        doc: "docs/guides/journal-and-verification.md",
        prompt,
        argv,
        place,
        stream,
        exit: Exit::Success,
        stdin: None,
    }
}

/// Flow 3's `recipes add` at `place`, exiting as `exit` says: the same
/// install onto a folder's shelf, then into a library as it is built.
const fn added(place: Place, exit: Exit) -> Sample {
    Sample {
        doc: "docs/guides/adopting-a-node-repo.md",
        prompt: "$ brokkr recipes add ~/src/brokkr/recipes/node --name node",
        argv: &ADD_NODE,
        place,
        stream: Stream::Stderr,
        exit,
        stdin: None,
    }
}

const SAMPLED: [Sample; 33] = [
    Sample {
        doc: "docs/guides/agent-library.md",
        prompt: "$ brokkr agents list",
        argv: &["agents", "list"],
        place: Place::Workspace,
        stream: Stream::Stdout,
        exit: Exit::Success,
        stdin: None,
    },
    Sample {
        doc: "docs/guides/recipe-authoring.md",
        prompt: "$ brokkr recipes list",
        argv: &["recipes", "list"],
        place: Place::Workspace,
        stream: Stream::Stdout,
        exit: Exit::Success,
        stdin: None,
    },
    Sample {
        doc: "docs/guides/recipe-authoring.md",
        prompt: "$ brokkr compile --bundle recipes/night-shift",
        argv: &["compile", "--bundle", "recipes/night-shift"],
        place: Place::Workspace,
        stream: Stream::Stdout,
        exit: Exit::Success,
        stdin: None,
    },
    Sample {
        doc: "docs/guides/quickstart.md",
        prompt: "$ brokkr --version",
        argv: &["--version"],
        place: Place::Workspace,
        stream: Stream::Stdout,
        exit: Exit::Success,
        stdin: None,
    },
    // Doctor exits non-zero on a host missing a tool it reports, which
    // is the host's, not the sample's: the sample shows what every host
    // prints and elides the rest.
    Sample {
        doc: "docs/guides/quickstart.md",
        prompt: "$ brokkr doctor",
        argv: &["doctor"],
        place: Place::Fixture("rust"),
        stream: Stream::Stdout,
        exit: Exit::Any,
        stdin: None,
    },
    Sample {
        doc: "docs/guides/quickstart.md",
        prompt: "$ brokkr init .",
        argv: &["init", "."],
        place: Place::Fixture("rust"),
        stream: Stream::Stderr,
        exit: Exit::Success,
        stdin: None,
    },
    // As the quickstart's doctor: the host decides the exit.
    Sample {
        doc: "docs/guides/versioning.md",
        prompt: "$ brokkr doctor",
        argv: &["doctor"],
        place: Place::Workspace,
        stream: Stream::Stdout,
        exit: Exit::Any,
        stdin: None,
    },
    // The guides' run itself, so not at the place it has driven.
    Sample {
        doc: "docs/guides/quickstart.md",
        prompt: "$ brokkr run --feature \"prefix selectors for the read surfaces\"",
        argv: &RUN,
        place: Place::Scaffold,
        stream: Stream::Stderr,
        exit: Exit::Success,
        stdin: None,
    },
    Sample {
        doc: "docs/guides/quickstart.md",
        prompt: "$ brokkr inspect --run latest",
        argv: &["inspect", "--run", "latest"],
        place: Place::Driven,
        stream: Stream::Stdout,
        exit: Exit::Success,
        stdin: None,
    },
    Sample {
        doc: "docs/guides/quickstart.md",
        prompt: "$ brokkr costs --run latest",
        argv: &["costs", "--run", "latest"],
        place: Place::Driven,
        stream: Stream::Stdout,
        exit: Exit::Success,
        stdin: None,
    },
    Sample {
        doc: "docs/guides/read-surfaces.md",
        prompt: "$ brokkr realms",
        argv: &["realms"],
        place: Place::Workspace,
        stream: Stream::Stdout,
        exit: Exit::Success,
        stdin: None,
    },
    Sample {
        doc: "docs/guides/read-surfaces.md",
        prompt: "$ brokkr runs",
        argv: &["runs"],
        place: Place::Driven,
        stream: Stream::Stdout,
        exit: Exit::Success,
        stdin: None,
    },
    Sample {
        doc: "docs/guides/read-surfaces.md",
        prompt: "$ brokkr inspect --run latest",
        argv: &["inspect", "--run", "latest"],
        place: Place::Driven,
        stream: Stream::Stdout,
        exit: Exit::Success,
        stdin: None,
    },
    Sample {
        doc: "docs/guides/read-surfaces.md",
        prompt: "$ brokkr seats --run latest",
        argv: &["seats", "--run", "latest"],
        place: Place::Driven,
        stream: Stream::Stdout,
        exit: Exit::Success,
        stdin: None,
    },
    // Decision 0087 ruling 8, slice 2 (#717): Brokkr's portable recipes
    // install onto a fresh folder's shelf.
    Sample {
        doc: "docs/guides/quickstart.md",
        prompt: "$ brokkr recipes add ~/src/brokkr/recipes/fast --name fast",
        argv: &[
            "recipes",
            "add",
            "~/src/brokkr/recipes/fast",
            "--name",
            "fast",
        ],
        place: Place::Folder("rust"),
        stream: Stream::Stderr,
        exit: Exit::Success,
        stdin: None,
    },
    Sample {
        doc: "docs/guides/cards/node.md",
        prompt: "$ brokkr recipes add ~/src/brokkr/recipes/node --name node",
        argv: &ADD_NODE,
        place: Place::Folder("node-npm"),
        stream: Stream::Stderr,
        exit: Exit::Success,
        stdin: None,
    },
    // Flow 3's long form runs the same install four times, in the guide's
    // order: onto a folder's shelf, then into a library as it is built.
    added(Place::Folder("node-npm"), Exit::Success),
    added(Place::Library(&[ADAPTER]), Exit::Refused),
    added(Place::Library(&[ADAPTER, "agents"]), Exit::Refused),
    added(
        Place::Library(&[ADAPTER, "agents", "capabilities"]),
        Exit::Success,
    ),
    // #365 slice 5: the journal guide's evidence, each command at a
    // driven run. The chained samples name the commands they read, so
    // `verify-run` and `import` run after the `export` that writes their
    // file, `anchor --check` after the `anchor` it checks, and
    // `keep-refs delete` after the `plant` it removes. The run's id ends
    // in a fresh uuid suffix every drive, so a prompt that names its file
    // writes `<run>`, which the sampler fills in.
    journal(
        "$ brokkr anchor --run latest",
        &["anchor", "--run", "latest"],
        Place::Driven,
        Stream::Stderr,
    ),
    journal(
        "$ brokkr anchor --run latest --check",
        &["anchor", "--run", "latest", "--check"],
        Place::DrivenAfter(&[step(&["anchor", "--run", "latest"])]),
        Stream::Stdout,
    ),
    journal(
        "$ brokkr export --run latest --out ./out --redact",
        &["export", "--run", "latest", "--out", "./out", "--redact"],
        Place::Driven,
        Stream::Stderr,
    ),
    journal(
        "$ brokkr export --run latest --out ./out",
        &["export", "--run", "latest", "--out", "./out"],
        Place::Driven,
        Stream::Stderr,
    ),
    journal(
        "$ brokkr verify-run ./out/<run>.ndjson",
        &["verify-run", "./out/<run>.ndjson"],
        Place::DrivenAfter(&[step(&EXPORT)]),
        Stream::Stdout,
    ),
    journal(
        "$ brokkr replay --run latest",
        &["replay", "--run", "latest"],
        Place::Driven,
        Stream::Stdout,
    ),
    journal(
        "$ brokkr import --from ./out/<run>.ndjson --db ./canonical/forge.db",
        &[
            "import",
            "--from",
            "./out/<run>.ndjson",
            "--db",
            "./canonical/forge.db",
        ],
        Place::DrivenAfter(&[step(&EXPORT)]),
        Stream::Stdout,
    ),
    journal(
        "$ brokkr keep-refs list",
        &["keep-refs", "list"],
        Place::Driven,
        Stream::Stdout,
    ),
    journal(
        "$ brokkr keep-refs plant --run latest",
        &["keep-refs", "plant", "--run", "latest"],
        Place::Driven,
        Stream::Stderr,
    ),
    journal(
        "$ brokkr keep-refs delete --run latest",
        &["keep-refs", "delete", "--run", "latest"],
        Place::DrivenAfter(&[step(&["keep-refs", "plant", "--run", "latest"])]),
        Stream::Stderr,
    ),
    // #365 slice 6: the secrets guide, in a fresh folder. `set` reads its
    // fake value from stdin, never argv; `list` and `remove` each run
    // after the `set` whose store entry they read.
    Sample {
        doc: "docs/guides/secrets.md",
        prompt: "$ brokkr secrets set GITHUB_TOKEN",
        argv: SET.argv,
        place: Place::Folder("rust"),
        stream: Stream::Stderr,
        exit: Exit::Success,
        stdin: Some(SECRET),
    },
    Sample {
        doc: "docs/guides/secrets.md",
        prompt: "$ brokkr secrets list",
        argv: &["secrets", "list"],
        place: Place::FolderAfter("rust", &[SET]),
        stream: Stream::Stdout,
        exit: Exit::Success,
        stdin: None,
    },
    Sample {
        doc: "docs/guides/secrets.md",
        prompt: "$ brokkr secrets remove GITHUB_TOKEN",
        argv: &["secrets", "remove", "GITHUB_TOKEN"],
        place: Place::FolderAfter("rust", &[SET]),
        stream: Stream::Stderr,
        exit: Exit::Success,
        stdin: None,
    },
];

/// Whether a line of a fenced block is a `$ brokkr` prompt.
fn is_prompt(line: &str) -> bool {
    line.trim_start().starts_with(PROMPT)
}

/// Each prompt in `docs` that neither opens a block a `sampled` row names
/// nor stands in a doc `allowed` names, and each allowance that covered
/// nothing.
fn unsampled(docs: &[Doc], sampled: &[Sample], allowed: &[(&str, &str)]) -> Vec<String> {
    let mut found = Vec::new();
    let mut used = vec![false; allowed.len()];
    for doc in docs {
        let allowance = allowed.iter().position(|(path, _)| *path == doc.path);
        for block in doc.blocks.iter().filter(|block| block.fenced) {
            for (at, line) in block.text.lines().enumerate() {
                let opens = |sample: &Sample| sample.doc == doc.path && sample.prompt == line;
                if !is_prompt(line) || (at == 0 && sampled.iter().any(opens)) {
                    continue;
                }
                match allowance {
                    Some(index) => used[index] = true,
                    None => found.push(format!(
                        "{}:{}: `{}` is neither sampled nor in an allowed doc",
                        doc.path,
                        block.first + at,
                        line.trim()
                    )),
                }
            }
        }
    }
    found.extend(
        allowed
            .iter()
            .zip(&used)
            .filter(|(_, used)| !**used)
            .map(|((path, _), _)| {
                format!("{path}: allowed, but covers nothing: no `{PROMPT}` prompt is left")
            }),
    );
    found
}

/// Whether one output line fits one sample line: equal, or each `…` in
/// the sample standing for any run of characters within the line.
fn line_fits(sample: &str, line: &str) -> bool {
    let mut pieces = sample.split(ELIDED);
    let Some(mut rest) = pieces.next().and_then(|first| line.strip_prefix(first)) else {
        return false;
    };
    let pieces: Vec<&str> = pieces.collect();
    let Some((last, middle)) = pieces.split_last() else {
        return rest.is_empty();
    };
    for piece in middle {
        match rest.find(piece) {
            Some(at) => rest = &rest[at + piece.len()..],
            None => return false,
        }
    }
    rest.ends_with(last)
}

/// The sample line no reading of `expected` gets past, or `None` when it
/// matches `actual` whole: the deepest line any alignment reached, and
/// `expected.len()` when the output runs on past the sample's end.
fn unmatched(expected: &[&str], actual: &[&str]) -> Option<usize> {
    let mut deepest = 0;
    let mut failed = BTreeSet::new();
    (!walk(expected, actual, (0, 0), &mut deepest, &mut failed)).then_some(deepest)
}

/// Whether `expected[i..]` matches `actual[j..]`, recording the deepest
/// sample line reached and each pair already known to fail.
fn walk(
    expected: &[&str],
    actual: &[&str],
    (i, j): (usize, usize),
    deepest: &mut usize,
    failed: &mut BTreeSet<(usize, usize)>,
) -> bool {
    *deepest = (*deepest).max(i);
    if failed.contains(&(i, j)) {
        return false;
    }
    let held = match expected.get(i) {
        None => j == actual.len(),
        Some(&ELIDED) => {
            (j..=actual.len()).any(|k| walk(expected, actual, (i + 1, k), deepest, failed))
        }
        Some(line) => {
            actual.get(j).is_some_and(|got| line_fits(line, got))
                && walk(expected, actual, (i + 1, j + 1), deepest, failed)
        }
    };
    if !held {
        failed.insert((i, j));
    }
    held
}

/// What a sample's stream showed, or why its exit is not the one it
/// expects.
type Ran = Result<String, String>;

/// What one command left: its exit code, both streams, and the fixture
/// root it ran in, canonical first and then as given, where it ran in
/// one.
struct Printed {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    roots: Vec<String>,
}

/// A fixture's root, canonical first and then as given.
fn roots_of(root: &Path) -> Vec<String> {
    let text = |path: &Path| path.to_str().unwrap().to_string();
    vec![text(&root.canonicalize().unwrap()), text(root)]
}

/// `command` under the config home `config` and no `COLUMNS`, so no
/// host's configuration (its `host.json`) or terminal width reaches a
/// sample, or `contributing`'s recipe listing (#726).
pub(crate) fn hermetic(command: &mut Command, config: &Path) {
    command.env("XDG_CONFIG_HOME", config).env_remove("COLUMNS");
}

/// [`RUN`] driven to `done` in `scaffold`: anything else is no place for
/// a sample, and fails with the run's stderr.
fn drive(scaffold: &Scaffold, configure: &dyn Fn(&mut Command)) {
    let (code, _, stderr) = brokkr_with(&RUN, &scaffold.root, configure);
    assert_eq!(
        code,
        Some(0),
        "the guides' run did not reach done: {stderr}"
    );
}

/// A run's id, as `inspect --run latest --json` prints it.
#[derive(Deserialize)]
struct Inspected {
    summary: RunSummary,
}

#[derive(Deserialize)]
struct RunSummary {
    run_id: String,
}

/// The id of the run [`drive`] left at `latest` in `scaffold`.
fn driven_run_id(scaffold: &Scaffold, configure: &dyn Fn(&mut Command)) -> String {
    let (code, stdout, stderr) = brokkr_with(
        &["inspect", "--run", "latest", "--json"],
        &scaffold.root,
        configure,
    );
    assert_eq!(
        code,
        Some(0),
        "inspect did not read the run a sample chains after: {stderr}"
    );
    let inspected: Inspected = serde_json::from_str(&stdout)
        .unwrap_or_else(|error| panic!("inspect printed no run id: {error}\n{stdout}"));
    inspected.summary.run_id
}

/// `arg` with the run's placeholder `<run>` filled by `run`.
fn filled(arg: &str, run: &str) -> String {
    arg.replace("<run>", run)
}

/// The built binary run with a sample's argv where the sample says, an
/// `init` with a `claude` first on PATH, as `init_stacks` runs it. Every
/// command it spawns is [`hermetic`], and its stdin is the sample's.
fn printed(sample: &Sample) -> Printed {
    let config = tempfile::tempdir().unwrap();
    let hermetic = |command: &mut Command| hermetic(command, config.path());
    let words: Vec<String> = sample.argv.iter().map(|arg| expanded(arg)).collect();
    let argv: Vec<&str> = words.iter().map(String::as_str).collect();
    let run = |cwd: &Path, roots: Vec<String>, configure: &dyn Fn(&mut Command), argv: &[&str]| {
        ran(argv, cwd, roots, configure, sample.stdin)
    };
    match sample.place {
        Place::Workspace => run(&workspace(), Vec::new(), &hermetic, &argv),
        Place::Fixture(fixture) => {
            let repo = repository_of(fixture);
            run(repo.path(), roots_of(repo.path()), &hermetic, &argv)
        }
        Place::Folder(fixture) => {
            let repo = folder(fixture, &hermetic);
            run(repo.path(), roots_of(repo.path()), &hermetic, &argv)
        }
        Place::FolderAfter(fixture, earlier) => {
            let repo = folder(fixture, &hermetic);
            chain(earlier, repo.path(), sample, &hermetic, None);
            run(repo.path(), roots_of(repo.path()), &hermetic, &argv)
        }
        Place::Library(paths) => {
            let repo = repository();
            for path in paths {
                copied(&workspace().join(path), &repo.path().join(path));
            }
            run(repo.path(), roots_of(repo.path()), &hermetic, &argv)
        }
        Place::Scaffold => {
            let before = Before {
                drive: false,
                chain: &[],
                run_id: false,
            };
            scaffolded(sample, &words, &hermetic, before)
        }
        Place::Driven => {
            let before = Before {
                drive: true,
                chain: &[],
                run_id: words.iter().any(|arg| arg.contains("<run>")),
            };
            scaffolded(sample, &words, &hermetic, before)
        }
        Place::DrivenAfter(chain) => {
            let before = Before {
                drive: true,
                chain,
                run_id: true,
            };
            scaffolded(sample, &words, &hermetic, before)
        }
    }
}

/// [`brokkr_with`] with `stdin` written whole to the child and closed, or
/// `None` leaving the null stream `Command::output` gives it; `roots` are
/// the fixture roots a sample's text reads as [`HOME`].
fn ran(
    argv: &[&str],
    cwd: &Path,
    roots: Vec<String>,
    configure: &dyn Fn(&mut Command),
    stdin: Option<&str>,
) -> Printed {
    let (code, stdout, stderr) = match stdin {
        None => brokkr_with(argv, cwd, configure),
        Some(input) => {
            let file = tempfile::NamedTempFile::new().unwrap();
            std::fs::write(file.path(), input).unwrap();
            let path = file.path().to_path_buf();
            brokkr_with(argv, cwd, |command| {
                configure(command);
                command.stdin(std::fs::File::open(&path).unwrap());
            })
        }
    };
    Printed {
        code,
        stdout,
        stderr,
        roots,
    }
}

/// The fresh [`Place::Folder`] `fixture` names: `init .` run in a fresh
/// `git init` copy of the fixture, which leaves the folder.
fn folder(fixture: &str, hermetic: &dyn Fn(&mut Command)) -> tempfile::TempDir {
    let repo = repository_of(fixture);
    let (code, _, stderr) = brokkr_with(&["init", "."], repo.path(), hermetic);
    assert_eq!(code, Some(0), "{fixture}: init refused: {stderr}");
    repo
}

/// Run each earlier [`Step`] at `cwd`, in order, each required to exit
/// zero: a failing earlier command refuses `sample` with its stderr, and
/// the sample's own argv never runs. `<run>` in an earlier argv is filled
/// from `run`, when a place read one.
fn chain(
    earlier: &[Step],
    cwd: &Path,
    sample: &Sample,
    configure: &dyn Fn(&mut Command),
    run: Option<&str>,
) {
    for step in earlier {
        let argv: Vec<String> = step
            .argv
            .iter()
            .map(|arg| match run {
                Some(run) => filled(&expanded(arg), run),
                None => expanded(arg),
            })
            .collect();
        let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
        let done = ran(&argv, cwd, Vec::new(), configure, step.stdin);
        assert_eq!(
            done.code,
            Some(0),
            "{}: the chain's `{}` did not exit zero, so the sample never runs: {}",
            sample.prompt,
            spelled(&argv),
            done.stderr
        );
    }
}

/// What a place on the scaffold does before a sample's own argv runs,
/// each choice stated by the place itself: whether [`RUN`] is driven
/// first, the earlier [`Step`]s run after it, in order, and whether the
/// driven run's id is read to fill `<run>`.
struct Before {
    drive: bool,
    chain: &'static [Step],
    run_id: bool,
}

/// A sample at one of the scaffold's places, after what `before` says,
/// run at the scaffold's root with the `claude` CLI stubbed. Each earlier
/// [`Step`] must exit zero, or the sample refuses with that command's
/// stderr and its own argv never runs.
fn scaffolded(
    sample: &Sample,
    words: &[String],
    hermetic: &dyn Fn(&mut Command),
    before: Before,
) -> Printed {
    let scaffold = scaffold_with(hermetic);
    let stubbed = |command: &mut Command| {
        hermetic(command);
        command
            .env("PATH", &scaffold.path)
            .env("BROKKR_CLAUDE_BIN", &scaffold.stub);
    };
    if before.drive {
        drive(&scaffold, &stubbed);
    }
    let id = before.run_id.then(|| driven_run_id(&scaffold, &stubbed));
    chain(
        before.chain,
        &scaffold.root,
        sample,
        &stubbed,
        id.as_deref(),
    );
    let fill = |arg: &str| match &id {
        Some(id) => filled(arg, id),
        None => arg.to_string(),
    };
    let argv: Vec<String> = words.iter().map(|arg| fill(arg)).collect();
    let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
    ran(
        &argv,
        &scaffold.root,
        roots_of(&scaffold.root),
        &stubbed,
        sample.stdin,
    )
}

/// `arg` as a shell passes it: a path under [`CHECKOUT`] read as the same
/// path under this checkout.
fn expanded(arg: &str) -> String {
    match arg.strip_prefix(CHECKOUT) {
        Some(rest) if rest.starts_with('/') => format!("{}{rest}", workspace().display()),
        _ => arg.to_string(),
    }
}

/// `from`, a file or a directory, copied to `to` with its parents.
fn copied(from: &Path, to: &Path) {
    if from.is_dir() {
        copy_dir(from, to);
    } else {
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(from, to).unwrap();
    }
}

/// Whether a character carries a path name on past where it stands.
fn continues(c: char) -> bool {
    c.is_alphanumeric() || "._-~".contains(c)
}

/// `text` with each occurrence of `root` that is the whole root, never
/// the head or the tail of a longer path, read as [`HOME`].
fn placed(text: &str, root: &str) -> String {
    let mut out = String::new();
    let mut from = 0;
    for (at, _) in text.match_indices(root) {
        let end = at + root.len();
        let after_a_path = text[..at].ends_with(|c: char| c == '/' || continues(c));
        if after_a_path || text[end..].starts_with(continues) {
            continue;
        }
        out.push_str(&text[from..at]);
        out.push_str(HOME);
        from = end;
    }
    out.push_str(&text[from..]);
    out
}

/// The stream a sample shows, its fixture roots read as [`HOME`], when
/// its command exited as the sample expects.
fn shown(sample: &Sample, printed: Printed) -> Ran {
    let held = match sample.exit {
        Exit::Success => printed.code == Some(0),
        Exit::Any => true,
        Exit::Refused => printed.code.is_some_and(|code| code != 0),
    };
    if !held {
        let code = printed
            .code
            .map_or("by a signal".to_string(), |code| code.to_string());
        return Err(format!("exited {code}: {}", printed.stderr));
    }
    let text = match sample.stream {
        Stream::Stdout => printed.stdout,
        Stream::Stderr => printed.stderr,
    };
    Ok(printed
        .roots
        .iter()
        .fold(text, |text, root| placed(&text, root)))
}

/// The prompt that runs `argv`: each argument as it is, and one that
/// holds a space in double quotes, as a shell reads it.
fn spelled(argv: &[&str]) -> String {
    let words: Vec<String> = argv
        .iter()
        .map(|arg| match arg.contains(' ') {
            true => format!("\"{arg}\""),
            false => arg.to_string(),
        })
        .collect();
    format!("{PROMPT} {}", words.join(" "))
}

/// Each sample in `sampled` that is not what `run` prints for its argv,
/// or whose block is not one command with its output.
fn sample_failures(
    docs: &[Doc],
    sampled: &[Sample],
    run: impl Fn(&Sample) -> Printed,
) -> Vec<String> {
    let mut found = Vec::new();
    for (index, sample) in sampled.iter().enumerate() {
        let spelled = spelled(sample.argv);
        if sample.prompt != spelled {
            found.push(format!(
                "{}: the sample `{}` runs `{spelled}`",
                sample.doc, sample.prompt
            ));
            continue;
        }
        let blocks: Vec<&Block> = docs
            .iter()
            .filter(|doc| doc.path == sample.doc)
            .flat_map(|doc| &doc.blocks)
            .filter(|block| block.fenced && block.text.lines().next() == Some(sample.prompt))
            .collect();
        // Rows naming one doc's same prompt are its blocks that open with
        // it, in order, one each.
        let twin = |other: &&Sample| other.doc == sample.doc && other.prompt == sample.prompt;
        let twins = sampled.iter().filter(twin).count();
        let nth = sampled[..index].iter().filter(twin).count();
        match blocks.len() == twins {
            true => found.extend(sample_failure(sample, blocks[nth], &run)),
            false => found.push(format!(
                "{}: {} fenced blocks open with `{}`; a sample is exactly one",
                sample.doc,
                blocks.len(),
                sample.prompt
            )),
        }
    }
    found
}

/// Why one sampled block is not what `run` prints, if it is not.
fn sample_failure(
    sample: &Sample,
    block: &Block,
    run: impl Fn(&Sample) -> Printed,
) -> Option<String> {
    let at = |index: usize| format!("{}:{}", sample.doc, block.first + index);
    let expected: Vec<&str> = block.text.lines().skip(1).collect();
    if let Some(index) = expected
        .iter()
        .position(|line| line.trim_start().starts_with("$ "))
    {
        return Some(format!(
            "{}: `{}` is a second `$ ` line; a sample runs one command",
            at(index + 1),
            expected[index].trim()
        ));
    }
    if expected
        .iter()
        .all(|line| line.trim().is_empty() || *line == ELIDED)
    {
        return Some(format!(
            "{}: `{}` shows no output but `{ELIDED}`",
            at(0),
            sample.prompt
        ));
    }
    let actual = match shown(sample, run(sample)) {
        Ok(actual) => actual,
        Err(why) => return Some(format!("{}: `{}` {why}", at(0), sample.prompt)),
    };
    let index = unmatched(&expected, &actual.lines().collect::<Vec<_>>())?;
    Some(match expected.get(index) {
        Some(line) => format!(
            "{}: `{line}` is not what `{}` printed:\n{actual}",
            at(index + 1),
            sample.prompt
        ),
        None => format!(
            "{}: the sample ends before what `{}` printed:\n{actual}",
            at(index),
            sample.prompt
        ),
    })
}

#[test]
fn every_brokkr_prompt_in_a_living_doc_is_sampled_or_allowed() {
    assert_eq!(
        unsampled(&readable_docs(), &SAMPLED, &ALLOWED),
        Vec::<String>::new(),
        "a living doc shows a `$ brokkr` prompt: sample it in SAMPLED, or allow its doc with a reason"
    );
}

/// The samples a host runs: all of them where the scaffold's gates can
/// be `boxed`, and else all but those at the scaffold's places.
fn runnable(boxed: bool) -> Vec<Sample> {
    SAMPLED
        .iter()
        .filter(|sample| boxed || !sample.place.on_the_scaffold())
        .copied()
        .collect()
}

/// Where the scaffold's gates cannot be boxed, the samples at its places
/// are skipped as `bootstrap_bench` skips: said on stderr, and refused
/// where `BROKKR_REQUIRE_BOUNDARY_EVIDENCE` asks for the box.
#[test]
fn every_sample_is_what_the_binary_prints() {
    let boxed = can_box_the_gates();
    if !boxed {
        brokkr_protocol::hands::skip_boundary_proof(
            brokkr_protocol::hands::boundary_evidence_required(),
            "no namespace can be built here for the scaffold's boxed gates, \
             so the samples at its places are not run",
        );
    }
    assert_eq!(
        sample_failures(&readable_docs(), &runnable(boxed), printed),
        Vec::<String>::new(),
        "a sampled block is not what its command prints: paste the output, eliding with `…`"
    );
}

/// A sample in `docs/a.md`, run from the workspace root, showing stdout
/// and expecting success.
fn sample(prompt: &'static str, argv: &'static [&'static str]) -> Sample {
    Sample {
        doc: "docs/a.md",
        prompt,
        argv,
        place: Place::Workspace,
        stream: Stream::Stdout,
        exit: Exit::Success,
        stdin: None,
    }
}

/// What a command left that ran in no fixture.
fn exited(code: Option<i32>, stdout: &str, stderr: &str) -> Printed {
    Printed {
        code,
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
        roots: Vec::new(),
    }
}

#[test]
fn a_sample_shows_the_stream_it_names() {
    let on = |stream| Sample {
        stream,
        ..sample("$ brokkr init .", &["init", "."])
    };
    let printed = || exited(Some(0), "to stdout", "to stderr");
    assert_eq!(
        shown(&on(Stream::Stderr), printed()),
        Ok("to stderr".into())
    );
    assert_eq!(
        shown(&on(Stream::Stdout), printed()),
        Ok("to stdout".into())
    );
}

#[test]
fn a_non_zero_exit_is_refused_unless_the_sample_takes_any() {
    let on = |exit| Sample {
        exit,
        ..sample("$ brokkr doctor", &["doctor"])
    };
    let unhealthy = |code| exited(code, "MISSING   git", "unhealthy");
    assert_eq!(
        shown(&on(Exit::Success), unhealthy(Some(1))),
        Err("exited 1: unhealthy".into())
    );
    assert_eq!(
        shown(&on(Exit::Success), unhealthy(None)),
        Err("exited by a signal: unhealthy".into())
    );
    assert_eq!(
        shown(&on(Exit::Any), unhealthy(Some(1))),
        Ok("MISSING   git".into())
    );
}

#[test]
fn a_refusal_sample_takes_only_a_non_zero_exit() {
    let refused = Sample {
        exit: Exit::Refused,
        ..sample("$ brokkr recipes list", &["recipes", "list"])
    };
    let printed = |code| exited(code, "error: refused", "the stderr");
    assert_eq!(
        shown(&refused, printed(Some(1))),
        Ok("error: refused".into())
    );
    assert_eq!(
        shown(&refused, printed(Some(0))),
        Err("exited 0: the stderr".into())
    );
    assert_eq!(
        shown(&refused, printed(None)),
        Err("exited by a signal: the stderr".into())
    );
}

#[test]
fn a_prompts_checkout_path_runs_under_this_checkout() {
    let here = workspace().display().to_string();
    assert_eq!(
        expanded("~/src/brokkr/recipes/fast"),
        format!("{here}/recipes/fast")
    );
    assert_eq!(
        expanded("~/src/brokkr-fork/recipes"),
        "~/src/brokkr-fork/recipes"
    );
    assert_eq!(expanded("--name"), "--name");
}

#[test]
fn a_fixture_root_reads_as_home_only_where_it_is_whole() {
    assert_eq!(
        placed("none from /t/r up to /t/r — init /t/r/.brokkr", "/t/r"),
        "none from /home/you/my-app up to /home/you/my-app — init /home/you/my-app/.brokkr"
    );
    assert_eq!(
        placed("/t/rr /t/r.d /t/r-x /x/t/r a/t/r", "/t/r"),
        "/t/rr /t/r.d /t/r-x /x/t/r a/t/r"
    );
    let canonical = Printed {
        roots: vec!["/private/var/t/r".into(), "/var/t/r".into()],
        ..exited(Some(0), "/private/var/t/r, /var/t/r/x", "")
    };
    assert_eq!(
        shown(&sample("$ brokkr doctor", &["doctor"]), canonical),
        Ok("/home/you/my-app, /home/you/my-app/x".into())
    );
}

/// A doc of the given fenced blocks, each with the line its text starts
/// on.
fn fenced(path: &str, blocks: &[(usize, &str)]) -> Doc {
    Doc {
        path: path.to_string(),
        blocks: blocks
            .iter()
            .map(|&(first, text)| Block {
                first,
                text: text.to_string(),
                fenced: true,
            })
            .collect(),
    }
}

#[test]
fn a_prompt_neither_sampled_nor_allowed_is_refused() {
    let sampled = [sample("$ brokkr runs", &["runs"])];
    let docs = [
        fenced(
            "docs/a.md",
            &[
                (3, "$ brokkr runs\nrun-1"),
                (9, "out\n$ brokkr runs"),
                (14, "$ brokkr runs --json"),
            ],
        ),
        fenced("docs/b.md", &[(2, "  $ brokkr doctor\n$ brokkr realms")]),
        fenced("docs/d.md", &[(7, "out\n  $ brokkr realms")]),
        fenced("docs/c.md", &[(5, "brokkr runs\n$ cargo test")]),
    ];
    let allowed = [("docs/b.md", "the host"), ("docs/c.md", "stale")];
    assert_eq!(
        unsampled(&docs, &sampled, &allowed),
        [
            "docs/a.md:10: `$ brokkr runs` is neither sampled nor in an allowed doc",
            "docs/a.md:14: `$ brokkr runs --json` is neither sampled nor in an allowed doc",
            "docs/d.md:8: `$ brokkr realms` is neither sampled nor in an allowed doc",
            "docs/c.md: allowed, but covers nothing: no `$ brokkr` prompt is left",
        ]
    );
}

#[test]
fn the_wildcard_reads_as_trycmd_does() {
    let reads = |expected: &str, actual: &str| {
        unmatched(
            &expected.lines().collect::<Vec<_>>(),
            &actual.lines().collect::<Vec<_>>(),
        )
    };
    assert_eq!(reads("a\n…\nb", "a\nb"), None);
    assert_eq!(reads("a\n…\nb", "a\nx\ny\nb"), None);
    assert_eq!(reads("a\n…", "a\nx\ny"), None);
    assert_eq!(reads("a\t…\tc", "a\tsome digest\tc"), None);
    assert_eq!(reads("…/recipes/fast", "/home/x/recipes/fast"), None);
    assert_eq!(reads("a…b…c", "a-c-b-c"), None);
    assert_eq!(reads("xy…yz", "xyz"), Some(0));
    assert_eq!(reads("a\tb", "a    b"), Some(0));
    assert_eq!(reads("a\n…\nb\nc", "a\nb\nx\nb\nd"), Some(3));
    assert_eq!(reads("a\nb", "a\nb\nc"), Some(2));
    assert_eq!(reads("a\nb", "a"), Some(1));
}

#[test]
fn a_sample_is_one_command_and_its_output() {
    let docs = [fenced(
        "docs/a.md",
        &[
            (3, "$ brokkr runs\nrun-1\n…\nrun-9"),
            (9, "$ brokkr realms\n…\n\n…"),
            (15, "$ brokkr seats\nanalyst\n$ brokkr doctor\nok"),
            (21, "$ brokkr costs\n…"),
            (26, "$ brokkr costs\ntotal"),
            (30, "$ brokkr ledger\ntotal"),
            (34, "$ brokkr inspect\none\ntwo"),
        ],
    )];
    let sampled = [
        sample("$ brokkr runs", &["runs"]),
        sample("$ brokkr realms", &["realms"]),
        sample("$ brokkr seats", &["seats"]),
        sample("$ brokkr costs", &["costs"]),
        sample("$ brokkr doctor", &["doctor"]),
        sample("$ brokkr  runs", &["runs"]),
        sample("$ brokkr ledger", &["ledger"]),
        sample("$ brokkr inspect", &["inspect"]),
    ];
    let run = |sample: &Sample| match sample.argv {
        ["runs"] => exited(Some(0), "run-1\nrun-2\nrun-8\n", ""),
        ["ledger"] => exited(Some(1), "", "no journal"),
        _ => exited(Some(0), "one\ntwo\nthree\n", ""),
    };
    assert_eq!(
        sample_failures(&docs, &sampled, run),
        [
            "docs/a.md:6: `run-9` is not what `$ brokkr runs` printed:\nrun-1\nrun-2\nrun-8\n",
            "docs/a.md:9: `$ brokkr realms` shows no output but `…`",
            "docs/a.md:17: `$ brokkr doctor` is a second `$ ` line; a sample runs one command",
            "docs/a.md: 2 fenced blocks open with `$ brokkr costs`; a sample is exactly one",
            "docs/a.md: 0 fenced blocks open with `$ brokkr doctor`; a sample is exactly one",
            "docs/a.md: the sample `$ brokkr  runs` runs `$ brokkr runs`",
            "docs/a.md:30: `$ brokkr ledger` exited 1: no journal",
            "docs/a.md:36: the sample ends before what `$ brokkr inspect` printed:\none\ntwo\nthree\n",
        ]
    );
}

#[test]
fn rows_naming_one_prompt_are_its_blocks_in_order() {
    let docs = [fenced(
        "docs/a.md",
        &[
            (3, "$ brokkr runs\nfirst"),
            (8, "$ brokkr runs\nsecond"),
            (13, "$ brokkr seats\none"),
        ],
    )];
    let first = sample("$ brokkr runs", &["runs"]);
    let second = Sample {
        place: Place::Library(&[]),
        ..first
    };
    let run = |sample: &Sample| match sample.place {
        Place::Library(_) => exited(Some(0), "second\n", ""),
        _ => exited(Some(0), "first\n", ""),
    };
    assert_eq!(
        sample_failures(&docs, &[first, second], run),
        Vec::<String>::new()
    );
    assert_eq!(
        sample_failures(&docs, &[second, first], run),
        [
            "docs/a.md:4: `first` is not what `$ brokkr runs` printed:\nsecond\n",
            "docs/a.md:9: `second` is not what `$ brokkr runs` printed:\nfirst\n",
        ]
    );
    let seats = sample("$ brokkr seats", &["seats"]);
    assert_eq!(
        sample_failures(&docs, &[seats, seats], run),
        ["docs/a.md: 1 fenced blocks open with `$ brokkr seats`; a sample is exactly one"; 2]
    );
}

#[test]
fn a_prompt_quotes_an_argument_that_holds_a_space() {
    assert_eq!(
        spelled(&RUN),
        "$ brokkr run --feature \"prefix selectors for the read surfaces\""
    );
    assert_eq!(
        spelled(&["inspect", "--run", "latest"]),
        "$ brokkr inspect --run latest"
    );
}

#[test]
fn the_scaffolds_samples_wait_for_a_box() {
    let prompts = |samples: Vec<Sample>| samples.iter().map(|s| s.prompt).collect::<Vec<_>>();
    assert_eq!(
        prompts(runnable(true)),
        prompts(SAMPLED.to_vec()),
        "a host that boxes the gates runs every sample"
    );
    assert_eq!(
        prompts(runnable(false)),
        [
            "$ brokkr agents list",
            "$ brokkr recipes list",
            "$ brokkr compile --bundle recipes/night-shift",
            "$ brokkr --version",
            "$ brokkr doctor",
            "$ brokkr init .",
            "$ brokkr doctor",
            "$ brokkr realms",
            "$ brokkr recipes add ~/src/brokkr/recipes/fast --name fast",
            "$ brokkr recipes add ~/src/brokkr/recipes/node --name node",
            "$ brokkr recipes add ~/src/brokkr/recipes/node --name node",
            "$ brokkr recipes add ~/src/brokkr/recipes/node --name node",
            "$ brokkr recipes add ~/src/brokkr/recipes/node --name node",
            "$ brokkr recipes add ~/src/brokkr/recipes/node --name node",
            "$ brokkr secrets set GITHUB_TOKEN",
            "$ brokkr secrets list",
            "$ brokkr secrets remove GITHUB_TOKEN",
        ]
    );
}

#[test]
fn a_sample_runs_under_a_config_home_of_its_own_and_no_width() {
    let mut command = Command::new("brokkr");
    hermetic(&mut command, Path::new("/t/config"));
    let set: BTreeMap<_, _> = command.get_envs().collect();
    let name = std::ffi::OsStr::new;
    assert_eq!(
        set,
        BTreeMap::from([
            (name("COLUMNS"), None),
            (name("XDG_CONFIG_HOME"), Some(name("/t/config"))),
        ])
    );
}

/// A policy table as the ladder's tables read it. A rule key these tables
/// do not render is refused by its parser rather than left out of a row.
#[derive(Deserialize)]
struct Policy {
    phases: Vec<String>,
    rules: Vec<Rule>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    id: String,
    from: String,
    result: String,
    #[serde(default)]
    when: BTreeMap<String, Value>,
    next: Option<String>,
    #[serde(default)]
    park: bool,
    severity: Option<String>,
    reason: String,
}

/// The two tables the ladder renders, each with the recipe it is.
const POLICIES: [(&str, &str); 2] = [
    ("fast", "recipes/fast/policy.json"),
    ("triage", "recipes/triage/policy.json"),
];

fn policy(path: &str) -> Policy {
    serde_json::from_str(&read(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
}

/// Whether a condition counts entries to a phase: `visits_<phase>_gte`.
fn is_bound(key: &str) -> bool {
    key.starts_with("visits_") && key.ends_with("_gte")
}

/// A table cell, its `|` escaped.
fn cell(text: &str) -> String {
    text.replace('|', "\\|")
}

/// A rule's conditions `keep` selects, each `` `key: value` ``, or
/// `none`.
fn conditions(rule: &Rule, keep: impl Fn(&str) -> bool) -> String {
    let kept: Vec<String> = rule
        .when
        .iter()
        .filter(|(key, _)| keep(key))
        .map(|(key, value)| format!("`{key}: {value}`"))
        .collect();
    match kept.is_empty() {
        true => "none".to_string(),
        false => kept.join(", "),
    }
}

/// A Markdown table of `header` and `rows`, each cell escaped.
fn table(header: &[&str], rows: &[Vec<String>]) -> String {
    let line = |cells: Vec<String>| format!("| {} |", cells.join(" | "));
    let mut lines = vec![
        line(header.iter().map(|title| cell(title)).collect()),
        format!("|{}", "---|".repeat(header.len())),
    ];
    lines.extend(
        rows.iter()
            .map(|row| line(row.iter().map(|text| cell(text)).collect())),
    );
    lines.join("\n")
}

/// Every bounded return in `policies`: a rule whose `next` its table
/// lists before its `from`, below an exhaustion arm, a rule with its
/// `from` and `result` whose conditions count entries to a phase.
fn bounded_returns(policies: &[(&str, Policy)]) -> String {
    let mut rows = Vec::new();
    for (recipe, policy) in policies {
        let place = |phase: &str| {
            policy
                .phases
                .iter()
                .position(|listed| listed == phase)
                .unwrap_or_else(|| panic!("{recipe}: `{phase}` is not a phase"))
        };
        for (at, rule) in policy.rules.iter().enumerate() {
            let Some(next) = rule.next.as_deref() else {
                continue;
            };
            let arms: Vec<&Rule> = policy.rules[..at]
                .iter()
                .filter(|arm| arm.from == rule.from && arm.result == rule.result)
                .filter(|arm| arm.when.keys().any(|key| is_bound(key)))
                .collect();
            if place(next) >= place(&rule.from) || arms.is_empty() {
                continue;
            }
            let bounds: BTreeSet<String> =
                arms.iter().map(|arm| conditions(arm, is_bound)).collect();
            let ids: Vec<String> = arms.iter().map(|arm| format!("`{}`", arm.id)).collect();
            rows.push(vec![
                format!("`{}`", rule.id),
                format!("`{recipe}`"),
                format!("`{}` → `{next}` on `{}`", rule.from, rule.result),
                conditions(rule, |key| !is_bound(key)),
                bounds.into_iter().collect::<Vec<_>>().join(", "),
                ids.join(", "),
            ]);
        }
    }
    table(
        &[
            "Rule",
            "Recipe",
            "Return",
            "Also requires",
            "Bound",
            "Exhaustion arms",
        ],
        &rows,
    )
}

/// Review's exhaustion arms in `fast`, in table order: each rule from
/// `review` that counts implement visits, with what else it requires, its
/// outcome, and its reason verbatim.
fn review_exhaustion(fast: &Policy) -> String {
    const BOUND: &str = "visits_implement_gte";
    let rows: Vec<Vec<String>> = fast
        .rules
        .iter()
        .filter(|rule| rule.from == "review" && rule.when.contains_key(BOUND))
        .map(|rule| {
            let outcome = match (&rule.next, rule.park, &rule.severity) {
                (Some(next), false, None) => format!("`next: {next}`"),
                (Some(next), false, Some(severity)) => {
                    format!("`next: {next}`, severity `{severity}`")
                }
                (None, true, None) => "`park: true`".to_string(),
                _ => panic!("{}: an outcome the ladder cannot render", rule.id),
            };
            vec![
                format!("`{}`", rule.id),
                conditions(rule, |key| key != BOUND),
                outcome,
                rule.reason.clone(),
            ]
        })
        .collect();
    table(&["Rule", "Also requires", "Outcome", "Reason"], &rows)
}

/// The guide's text between `<!-- {name}:start -->` and
/// `<!-- {name}:end -->`, trimmed.
fn between<'a>(guide: &'a str, name: &str) -> &'a str {
    let start = format!("<!-- {name}:start -->");
    let end = format!("<!-- {name}:end -->");
    let (_, after) = guide
        .split_once(&start)
        .unwrap_or_else(|| panic!("the guide lost {start}"));
    let (inside, _) = after
        .split_once(&end)
        .unwrap_or_else(|| panic!("the guide lost {end}"));
    inside.trim()
}

const GUIDE: &str = "docs/guides/recipe-authoring.md";

#[test]
fn the_bounded_returns_table_is_the_policy_files() {
    let policies: Vec<(&str, Policy)> = POLICIES
        .iter()
        .map(|&(recipe, path)| (recipe, policy(path)))
        .collect();
    let rendered = bounded_returns(&policies);
    assert_eq!(
        between(&read(GUIDE), "bounded-returns"),
        rendered,
        "{GUIDE}'s bounded returns drifted from the policy files; the table they render:\n\n{rendered}\n"
    );
}

#[test]
fn the_review_exhaustion_table_is_fasts_policy() {
    let rendered = review_exhaustion(&policy(POLICIES[0].1));
    assert_eq!(
        between(&read(GUIDE), "review-exhaustion"),
        rendered,
        "{GUIDE}'s review exhaustion arms drifted from fast's policy; the table it renders:\n\n{rendered}\n"
    );
}
