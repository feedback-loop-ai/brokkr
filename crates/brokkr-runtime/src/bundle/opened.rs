//! What one compile opens, when a site first needs it (#349, ask 6).
//!
//! Compile-time resolution depends on exactly two digested inputs: the
//! library and the adapters. Availability is UNSPECIFIED here — a compile
//! that probed PATH would give one bundle two digests and make an in-flight
//! run unresumable after an `apt install`.
//!
//! The ADAPTERS are the wider need. An agent reference resolves through
//! them, decision 0021's refusals read a driver's tier and grant out of
//! them, and a typed `tools` declaration (decision 0065 slice one, design
//! D5.2) is judged against what an adapter can represent, so the first
//! agent reference, gate-class site, declared secret binding, typed `tools`
//! or dialect site opens them strictly: a load that fails refuses the
//! compile. An inline built-in model driver opens them OPTIONALLY (decision
//! 0066 ruling 1): a failure is kept, never refused at the site, and read by
//! the capability pass and decision 0069's check after every seat. A bundle
//! with none of these compiles with no `adapters/` directory in sight.
//!
//! The LIBRARY is opened only by an agent reference, which is what keeps a
//! missing one a non-event for every bundle that inlines. Each is loaded at
//! most once, and what the load said is kept, so every later site reads the
//! same value.

use std::path::Path;

use super::{tier, CapabilityAdapters, CompileError, LibraryRead, SeatClass};
use crate::agents::{Adapters, Candidate, EgressClass, Library};
use crate::capabilities::Definitions;

/// Why a strict open needed the adapters, after the loader's own words.
const ADAPTERS_NEEDED: &str = "the adapter data is where a driver's model mapping (decision 0016) \
                               and its trust tier and binding grant (decision 0021) are \
                               declared, and this bundle names an agent, seats a gate, declares a \
                               secret binding or declares typed tools";

/// The agent library and adapters for one compilation, opened as its sites
/// need them, plus every library charter's bound read.
pub(super) struct Opened<'a> {
    library_root: &'a Path,
    adapters_root: &'a Path,
    /// What a loaded library's every agent is linted against.
    definitions: &'a Definitions,
    /// Decision 0036 ruling 4: the egress class a seat's resolved route
    /// must MEET before that seat may declare secret bindings. The
    /// operator rules it into the bundle; absence is `Contracted`, which
    /// is exactly what `binding_grant: true` meant, so every bundle on
    /// disk keeps the behaviour it has.
    pub(super) egress_minimum: EgressClass,
    /// The offices a provisional model may hold ([`super::RealmLaw`]).
    pub(super) provisional_offices: Vec<String>,
    /// `None` until a site opens them; then the load, or the loader's words.
    adapters: Option<Result<Adapters, String>>,
    /// `None` until an agent reference opens it.
    library: Option<Result<Library, LibraryFault>>,
    /// Every library charter's bound read, held until the bundle is sealed
    /// (rebuild unit 18-fix-b return, council F2).
    pub(super) reads: Vec<LibraryRead>,
    /// Sites the provisional tier reached before any site opened the
    /// adapters, judged by [`Opened::settle`].
    waiting: Vec<Waiting>,
}

/// Why the library a compile opened is refused: its load, or decision 0065
/// ruling 1's lint of every agent in it.
enum LibraryFault {
    Unloadable(String),
    Undefined(String),
}

impl LibraryFault {
    fn refusal(&self) -> CompileError {
        match self {
            LibraryFault::Unloadable(words) => CompileError::Invalid(words.clone()),
            LibraryFault::Undefined(problems) => CompileError::Capability(problems.clone()),
        }
    }
}

/// One site the tier waits to judge: its label, its written command, its
/// resolved chain and its class.
struct Waiting {
    what: String,
    command: Vec<String>,
    candidates: Vec<Candidate>,
    class: SeatClass,
}

impl<'a> Opened<'a> {
    /// Nothing opened yet.
    pub(super) fn new(
        (library_root, adapters_root): (&'a Path, &'a Path),
        definitions: &'a Definitions,
        egress_minimum: EgressClass,
        provisional_offices: Vec<String>,
    ) -> Opened<'a> {
        Opened {
            library_root,
            adapters_root,
            definitions,
            egress_minimum,
            provisional_offices,
            adapters: None,
            library: None,
            reads: Vec::new(),
            waiting: Vec::new(),
        }
    }

    /// The adapters, opened strictly: a load that fails, now or at an
    /// earlier optional read, refuses with the loader's words and why.
    pub(super) fn adapters(&mut self) -> Result<&Adapters, CompileError> {
        strict(&mut self.adapters, self.adapters_root)
    }

    /// Decision 0066 ruling 1's optional read, for an inline built-in model
    /// driver: the adapters are opened, and the `Result` the load gave is
    /// kept rather than refused here.
    ///
    /// Readers with opposite needs. The effortless-route exemption (decision
    /// 0035 addendum 2026-09-11), the inline resume assessment and the inline
    /// discovery notice (decision 0069) take the `Ok` and read an absent or
    /// unloadable root as none of them, so the strict rule stands. The
    /// capability pass is MANDATORY: the same data says how a harness's native
    /// search is switched off, and an error swallowed here used to reach it as
    /// "nothing declared" and compile a Codex seat with no denial. It takes the
    /// error too, and refuses the seat with the loader's own words. A PRESENT
    /// root that does not load is refused after that pass in any case
    /// (decision 0069): a malformed discovery notice must not pass itself off
    /// as an adapter that declares none.
    pub(super) fn read_for_inline_driver(&mut self) {
        open(&mut self.adapters, self.adapters_root);
    }

    /// The adapters some site opened and that loaded, or none.
    pub(super) fn loaded(&self) -> Option<&Adapters> {
        self.adapters.as_ref()?.as_ref().ok()
    }

    /// The loader's words, where an optional read failed.
    pub(super) fn unloadable(&self) -> Option<&str> {
        self.adapters.as_ref()?.as_ref().err().map(String::as_str)
    }

    /// What the capability pass resolves against: the loaded adapters, or
    /// why there are none.
    pub(super) fn capability_adapters(&self) -> CapabilityAdapters<'_> {
        match &self.adapters {
            Some(Ok(adapters)) => CapabilityAdapters::loaded(adapters),
            unloaded => CapabilityAdapters::unloaded(unloaded.as_ref()),
        }
    }

    /// The library and the adapters an agent reference resolves through.
    /// Decision 0065 ruling 1 (CQ2; design D3): the library is linted whole
    /// when it is opened, before the reference that opened it is resolved
    /// or subtracts anything — every agent in it, seated or not, asks only
    /// for what the operator defined. The capability walk resolves seated
    /// references alone, so without this a valid seated worker hides an
    /// unseated office's undefined request. A library no site opens is not
    /// loaded to be linted. A library that loads is refused for adapters
    /// that do not before the lint speaks.
    pub(super) fn agent_roots(&mut self) -> Result<(&Library, &Adapters), CompileError> {
        let opened = match self.library.take() {
            Some(opened) => opened,
            None => {
                let loaded = Library::load(self.library_root)
                    .map_err(|error| LibraryFault::Unloadable(error.to_string()));
                if loaded.is_ok() {
                    self.adapters()?;
                }
                loaded.and_then(|library| linted(self.definitions, library))
            }
        };
        let library = self
            .library
            .insert(opened)
            .as_ref()
            .map_err(LibraryFault::refusal)?;
        Ok((library, strict(&mut self.adapters, self.adapters_root)?))
    }

    /// The library an agent reference opened, or none.
    pub(super) fn library(&self) -> Option<&Library> {
        self.library.as_ref()?.as_ref().ok()
    }

    /// What ruling 1's lint consulted, pinned beside the seats' own: every
    /// capability name an agent of the opened library asks for.
    pub(super) fn library_asks(&self) -> Vec<String> {
        self.library()
            .into_iter()
            .flat_map(Library::agents)
            .flat_map(|agent| agent.capabilities.keys().cloned())
            .collect()
    }

    /// The adapters the provisional tier judges `what` by now, and the
    /// offices a provisional model may hold. A site the tier reaches
    /// before any site opened the adapters waits for [`Opened::settle`],
    /// because a later site may yet open them; one reached after an
    /// optional read failed is judged by nothing, as the failure leaves
    /// nothing declared.
    pub(super) fn for_tier(
        &mut self,
        what: &str,
        command: &[String],
        candidates: &[Candidate],
        class: SeatClass,
    ) -> Option<(&Adapters, &[String])> {
        match &self.adapters {
            None => {
                self.waiting.push(Waiting {
                    what: what.to_string(),
                    command: command.to_vec(),
                    candidates: candidates.to_vec(),
                    class,
                });
                None
            }
            Some(loaded) => Some((loaded.as_ref().ok()?, &self.provisional_offices)),
        }
    }

    /// Once every seat is parsed: each site that waited is judged by the
    /// adapters the compile opened, in the order the sites were reached,
    /// exactly as it would have been had they been open when it was.
    pub(super) fn settle(&mut self) -> Result<(), CompileError> {
        let waiting = std::mem::take(&mut self.waiting);
        let Some(adapters) = self.loaded() else {
            return Ok(());
        };
        for site in &waiting {
            tier::judge(
                (&site.what, &site.command),
                &site.candidates,
                site.class,
                adapters,
                &self.provisional_offices,
            )?;
        }
        Ok(())
    }
}

/// The adapters, opened on the first call; later calls read what it said.
fn open<'s>(
    slot: &'s mut Option<Result<Adapters, String>>,
    root: &Path,
) -> &'s Result<Adapters, String> {
    slot.get_or_insert_with(|| Adapters::load(root).map_err(|error| error.to_string()))
}

/// [`open`], refusing a load that failed.
fn strict<'s>(
    slot: &'s mut Option<Result<Adapters, String>>,
    root: &Path,
) -> Result<&'s Adapters, CompileError> {
    open(slot, root)
        .as_ref()
        .map_err(|words| CompileError::Invalid(format!("{words}; {ADAPTERS_NEEDED}")))
}

/// The library, once every agent in it asks only for what is defined.
fn linted(definitions: &Definitions, library: Library) -> Result<Library, LibraryFault> {
    let problems = definitions.lint(&library);
    match problems.is_empty() {
        true => Ok(library),
        false => Err(LibraryFault::Undefined(problems.join("; "))),
    }
}
