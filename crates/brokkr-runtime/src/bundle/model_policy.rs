//! Decision 0021's two compile-time refusals: every model-bearing site
//! pins its model (decision 0031 ruling 2), and its effort (decision 0035
//! ruling 5), read by one walker over each written driver command
//! (decision 0040).

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use super::{CompileError, DriverDigests, Seat, SeatBody, StepBody};
use crate::agents::{route_is_effortless, Adapter, Adapters, HandsNotice};
use crate::argv::Dispatch;

/// The model-bearing built-in named by one written driver command
/// ([`command_parts`]). Custom drivers own their own contract; exec has no
/// model to pin.
pub(crate) fn built_in_model_driver(command: &[String]) -> Option<&'static str> {
    let Dispatch::Driver {
        kind: dispatched, ..
    } = Dispatch::of(command)
    else {
        return None;
    };
    ["claude", "lanetally", "codex", "dsh"]
        .into_iter()
        .find(|kind| dispatched == *kind)
}

/// What one inline command's argv says about one pin — the model pin on
/// `--model`, the effort pin on `--effort` (decision 0035). THREE states,
/// because decision 0031 and decision 0036 ruling 2 read the same walk
/// for different facts and only 0031 can collapse the last two: 0031
/// ruling 2 asks whether there IS a readable pin, while 0036 asks WHERE
/// the material goes — and "this argv names no model" and "this argv
/// names a model I cannot read" are different answers to that second
/// question. Kept apart here so the two rulings cannot drift over what
/// "the pinned model" means, and so the route resolver never has to
/// guess which kind of silence it was handed. One walker for both axes,
/// because both are the same rule: named once, concretely, never as a
/// second flag that could outrank the first and never as something that
/// reads as a flag itself.
pub(super) enum ModelPin {
    /// No such flag anywhere in the argv: `exec`, and every driver that
    /// takes its destination from its own profile.
    Absent,
    /// One readable concrete value; for a model, its `<route>/` prefix
    /// is the route.
    Concrete(String),
    /// The flag is present and this compiler cannot read it as one
    /// concrete value: flag-shaped, empty, over-long, outside the id
    /// alphabet, dangling at the end of the argv, or pinned twice. It
    /// carries the flags the read used to reach that answer, because
    /// decision 0040 ruling 3 says a refusal names them and no constant
    /// stands in for a flag the read did not use.
    Unreadable(Vec<String>),
}

/// Decision 0040 ruling 2: a flag is SHORT when it is one dash and one
/// character, which is the shape the getopt convention lets a value
/// attach itself to (`-mspark/x`). Every other flag — the long `--model`
/// this engine composes, and anything else an adapter may declare — has
/// exactly the two spellings `FLAG VALUE` and `FLAG=VALUE`, and a longer
/// word beginning with it is a different flag of the same family rather
/// than an illegible spelling of this one.
pub(crate) fn short_flag(flag: &str) -> bool {
    let mut characters = flag.chars();
    characters.next() == Some('-')
        && matches!(characters.next(), Some(c) if c != '-')
        && characters.next().is_none()
}

/// The flag every adapter's route pin is read on beside its own
/// declaration (decision 0040 ruling 1), and the flag this engine
/// composes for the four model-bearing built-ins (decisions 0031, 0016).
pub(super) const MODEL_FLAG: &str = "--model";

/// One walker, one flag, and the spellings that flag's own SHAPE has
/// (decision 0040 ruling 2) — not the spellings its caller asked for.
/// The shape is a property of the string, so 0031's reader and 0036's
/// reader can no longer disagree about what a word means: a long
/// neighbour is walked past by both, and a short flag's attached value
/// is a pin to both.
fn command_pin(command: &[String], flag: &str, limit: usize) -> ModelPin {
    let illegible = || ModelPin::Unreadable(vec![flag.to_string()]);
    let concrete = |value: &str| {
        !value.is_empty()
            && !value.starts_with('-')
            && value.chars().count() <= limit
            && value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '/'))
    };
    let attached = format!("{flag}=");
    let mut pin = None;
    let mut parts = command.iter().map(String::as_str);
    while let Some(part) = parts.next() {
        if part == flag {
            match parts.next().filter(|value| concrete(value)) {
                Some(value) if pin.replace(value).is_none() => {}
                _ => return illegible(),
            }
        } else if let Some(value) = part.strip_prefix(&attached) {
            if !concrete(value) || pin.replace(value).is_some() {
                return illegible();
            }
        } else if let Some(value) = part.strip_prefix(flag) {
            // A word that BEGINS with the flag and is neither of the
            // two spellings above. Decision 0040 ruling 2 decides it by
            // the flag's shape rather than by who is asking:
            //
            // - a SHORT flag carries its value attached, by the getopt
            //   convention every CLI this fleet has met honours, so the
            //   remainder IS the pin. A remainder that is not one
            //   concrete model id is illegible, which costs
            //   `-march=native` under `-m` a refusal naming the flag
            //   and nothing else;
            // - a LONG flag has no such form, so a longer word is an
            //   unrelated flag of the same family and every reader
            //   walks past it. `--model-fallback` is not a way of
            //   writing `--model`, for 0036 as much as for 0031.
            if short_flag(flag) && (!concrete(value) || pin.replace(value).is_some()) {
                return illegible();
            }
        }
    }
    match pin {
        Some(model) => ModelPin::Concrete(model.to_string()),
        None => ModelPin::Absent,
    }
}

/// The model pin as decision 0036 ruling 2 reads it: which route the
/// material goes to, or which kind of silence the argv holds — on one
/// of the two flags decision 0040 ruling 1 has it read. On `--model`,
/// the flag this engine composes for the four model-bearing built-ins,
/// it is also decision 0031 ruling 2's read, which asks only whether one
/// concrete id is stated; a neighbouring `--model…` flag is a different
/// flag to it, not an illegible spelling of this one, because `--model`
/// is long.
pub(super) fn route_pin(command: &[String], flag: &str) -> ModelPin {
    command_pin(command, flag, 80)
}

/// Decision 0040 ruling 1's read, and the whole of it: an inline site's
/// route, taken from the flag the adapter DECLARES (`model_flag`, since
/// decision 0016) and from `--model`, the one flag every model CLI this
/// fleet has met accepts. Where the two are the same string that is one
/// read, as it has always been.
///
/// The join is the ruling: a concrete pin on either flag names the
/// route, because the material goes where the argv says on whichever
/// flag the provider honours; two concrete pins naming DIFFERENT ids are
/// illegible, refused naming both, because the material can only go one
/// place and the argv says two; and a read that could not be made out on
/// either flag stays illegible, since a seat binding a secret does not
/// get the benefit of a doubt the machine genuinely has.
///
/// An adapter declaring `model_flag: "unsupported"` still has `--model`
/// read. A provider that cannot be told a model has no route to name, so
/// a pin found there is a site telling a binary something the adapter
/// says it cannot hear: illegible, never the adapter's own class.
pub(super) fn inline_route_pin(command: &[String], adapter: Option<&Adapter>) -> ModelPin {
    let declared = adapter.and_then(|adapter| adapter.model_flag.as_deref());
    let untellable = matches!(adapter, Some(adapter) if adapter.model_flag.is_none());
    let mut readings = Vec::new();
    if let Some(flag) = declared.filter(|flag| *flag != MODEL_FLAG) {
        readings.push(route_pin(command, flag));
    }
    readings.push(match (route_pin(command, MODEL_FLAG), untellable) {
        (ModelPin::Concrete(_), true) => ModelPin::Unreadable(vec![MODEL_FLAG.to_string()]),
        (pin, _) => pin,
    });

    let illegible: Vec<String> = readings
        .iter()
        .filter_map(|pin| match pin {
            ModelPin::Unreadable(flags) => Some(flags.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    if !illegible.is_empty() {
        return ModelPin::Unreadable(illegible);
    }
    let mut named: Vec<&String> = readings
        .iter()
        .filter_map(|pin| match pin {
            ModelPin::Concrete(model) => Some(model),
            _ => None,
        })
        .collect();
    named.dedup();
    match named.as_slice() {
        [] => ModelPin::Absent,
        [model] => ModelPin::Concrete((*model).clone()),
        // Two flags, two destinations, one body of material: the argv
        // cannot be obeyed, so it is not read.
        _ => ModelPin::Unreadable(
            declared
                .into_iter()
                .chain(Some(MODEL_FLAG))
                .map(str::to_string)
                .collect(),
        ),
    }
}

pub(super) fn command_pins_model(command: &[String]) -> bool {
    matches!(route_pin(command, MODEL_FLAG), ModelPin::Concrete(_))
}

/// Decision 0031 ruling 2: a model-bearing built-in with no concrete `--model`.
pub(super) fn model_unpinned(command: &[String]) -> bool {
    built_in_model_driver(command).is_some() && !command_pins_model(command)
}

/// Issue #373: the dsh driver admits one spelling of its model pin, the
/// separate `--model <id>`, and refuses every other word beginning with
/// `--model` before it spawns — the joined `--model=<id>` this compiler
/// reads as a pin (decision 0040 ruling 2) and a long neighbour such as
/// `--model-fallback` that it walks past alike. A seat compile admitted
/// would then park at spawn, after its run and journal exist, so compile
/// refuses what spawn will.
fn dsh_refuses_model_word(command: &[String]) -> bool {
    command
        .iter()
        .any(|part| part != MODEL_FLAG && part.starts_with(MODEL_FLAG))
}

/// The effort pin bound, matching `seat-record/v2`'s own: a level is one
/// bounded word, never a path and never a sentence.
fn command_pins_effort(command: &[String]) -> bool {
    matches!(command_pin(command, "--effort", 40), ModelPin::Concrete(_))
}

/// Every driver-bearing invocation site that states one of the two pins
/// and not the other, or neither.
#[derive(Default)]
struct Unpinned {
    model: Vec<String>,
    effort: Vec<String>,
    /// Issue #373: dsh sites carrying a `--model…` word the dsh driver
    /// refuses at spawn.
    dsh_model: Vec<String>,
    /// Decision 0035 addendum 2026-09-11: per site, the adapter digest
    /// whose effortless listing exempted it from the effort pin. The
    /// declaration that authorised the exemption rides the bundle's
    /// identity beside decision 0021's — un-listing a route moves the
    /// digest of every bundle it excused.
    witnessed: BTreeMap<String, DriverDigests>,
    /// Proposed decision 0056 ruling 5: per inline driver-bearing site,
    /// the resume assessment the adapter it names declares, carried to
    /// [`Bundle::inline_resume`] for the driver's private start context.
    resume: Map<String, Value>,
    /// The declaration each of those assessments was read from. Kept
    /// apart from `witnessed` because an exemption is a different fact
    /// from a consumed assessment, but merged into the manifest's
    /// `drivers` pin with it: an edit to the resume block must move the
    /// bundle identity `pinned_bundle_holds` compares, or a changed
    /// assessment would reuse a root the old one opened.
    resume_witness: BTreeMap<String, DriverDigests>,
    /// Decision 0069: per inline driver-bearing site, the
    /// discovery notice its adapter declares. The declaration is already
    /// witnessed in `resume_witness`, which pins every adapter an inline
    /// built-in consults, assessment or not.
    hands_notice: BTreeMap<String, HandsNotice>,
}

/// Decision 0035 addendum 2026-09-11: a seat whose concrete lane
/// resolves through its adapter to an effortless route needs no effort
/// pin — and the adapter digest that says so is witnessed beside the
/// exemption. Only a readable concrete id with a route prefix can
/// claim it: a bare id keeps the adapter default's standing, and an
/// unreadable pin is refused on the model axis first.
pub(super) fn effort_exempt(
    what: &str,
    command: &[String],
    adapters: Option<&Adapters>,
    witnessed: &mut BTreeMap<String, DriverDigests>,
) -> bool {
    let adapters = match adapters {
        Some(adapters) => adapters,
        None => return false,
    };
    let kind = match built_in_model_driver(command) {
        Some(kind) => kind,
        None => return false,
    };
    let adapter = match adapters.adapter(kind) {
        Some(adapter) => adapter,
        None => return false,
    };
    match inline_route_pin(command, Some(adapter)) {
        ModelPin::Concrete(id) if route_is_effortless(adapter, &id) => {
            let mut authorised = Map::new();
            authorised.insert(kind.to_string(), Value::String(adapter.digest.clone()));
            witnessed.insert(what.to_string(), authorised);
            true
        }
        _ => false,
    }
}

/// Every driver-bearing site of one PARSED body with the command it wrote,
/// under its label, by an exhaustive match over the typed forms: a form
/// this walk is not taught does not compile, so none is skipped silently.
/// An agent reference writes none; the resolver pins its chain.
fn written_sites<'a>(what: &str, body: &'a SeatBody) -> Vec<(String, &'a [String])> {
    match body {
        SeatBody::Invocation(invocation) => invocation.written_sites(what),
        SeatBody::Sequence { steps } => steps
            .iter()
            .flat_map(|step| {
                let what = format!("{what}:{}", step.name);
                match &step.body {
                    StepBody::Invocation(invocation) => invocation.written_sites(&what),
                    StepBody::Dialect { .. } => Vec::new(),
                }
            })
            .collect(),
        SeatBody::Select { cases, default } => cases
            .iter()
            .map(|(case, body)| (case.as_str(), body))
            .chain(default.as_deref().map(|body| ("default", body)))
            .flat_map(|(case, body)| written_sites(&format!("{what}:{case}"), body))
            .collect(),
    }
}

/// One site's written command: an inline built-in states both pins.
fn collect_unpinned(what: &str, argv: &[String], adapters: Option<&Adapters>, out: &mut Unpinned) {
    // Both built-in lists are the same list: exec has neither a model to
    // pin nor an effort to pin, and a custom driver owns its own
    // contract. So a site is asked for both pins or for neither, and a
    // seat missing both is named in both halves of one refusal.
    if let Some(kind) = built_in_model_driver(argv) {
        if model_unpinned(argv) {
            out.model.push(what.to_string());
        }
        if !command_pins_effort(argv) && !effort_exempt(what, argv, adapters, &mut out.witnessed) {
            out.effort.push(what.to_string());
        }
        if kind == "dsh" && dsh_refuses_model_word(argv) {
            out.dsh_model.push(what.to_string());
        }
        // The adapter a built-in model driver names answers for this
        // INLINE site as it does for an agent-resolved one: its measured
        // resume assessment travels to the engine so the driver's gate
        // can judge an offer at this site. Unmeasured stays unmeasured —
        // an adapter with no resume block contributes the null the gate
        // reads as `unsupported-resume` — and a missing or malformed
        // adapters root contributes nothing at all.
        if let Some(adapter) = adapters.and_then(|adapters| adapters.adapter(kind)) {
            out.resume.insert(what.to_string(), adapter.resume.value());
            // The DECLARATION the assessment was read from is pinned
            // beside every other adapter a site consulted: the engine
            // consumes this resume block to decide whether that site
            // rejoins, so an edit to it must move the bundle identity
            // that `pinned_bundle_holds` compares and the instance key
            // the offer is stamped with. Otherwise a changed
            // restrictions assessment would enable a different rejoin
            // under the old, still-eligible root. Recorded beside the
            // exemption map rather than inside it because the two are
            // different facts; both ride the manifest's `drivers` pin.
            let mut authorised = Map::new();
            authorised.insert(kind.to_string(), Value::String(adapter.digest.clone()));
            out.resume_witness.insert(what.to_string(), authorised);
            // Its discovery notice (decision 0069) is read from
            // that same witnessed declaration, and kept apart from the
            // assessment: reading a notice qualifies no resume.
            if let Some(notice) = &adapter.hands_notice {
                out.hands_notice.insert(what.to_string(), notice.clone());
            }
        }
    }
}

fn labels(sites: &[String]) -> String {
    sites
        .iter()
        .map(|site| format!("'{site}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What `enforce_model_pins` returns: the adapter digests whose effortless
/// listings exempted inline seats, the resume assessment each inline
/// built-in model driver's adapter declares for the engine, the
/// declaration each of those assessments was read from, and the discovery
/// notice each declares. Four maps rather than one because they answer
/// different questions from the same walk — the third rides the
/// manifest's `drivers` pin beside the first, while the second travels to
/// the driver's private start context and the fourth to the engine's
/// per-attempt notice.
type PinWitness = (
    BTreeMap<String, DriverDigests>,
    Map<String, Value>,
    BTreeMap<String, DriverDigests>,
    BTreeMap<String, HandsNotice>,
);

/// One refusal names the complete repair set, on BOTH axes. A model pin
/// without an effort pin is half a hire (decision 0035 ruling 5), so the
/// two clauses stand beside each other rather than the first hiding the
/// second behind a second compile. Returns the adapter digests whose
/// effortless listings exempted inline seats, for the manifest, beside
/// the resume assessment each inline built-in model driver's adapter
/// declares, for the engine's private start context, beside the digest of
/// the declaration each assessment was read from, for the manifest.
pub(super) fn enforce_model_pins(
    seats: &BTreeMap<String, Seat>,
    adapters: Option<&Adapters>,
) -> Result<PinWitness, CompileError> {
    let mut unpinned = Unpinned::default();
    for (phase, seat) in seats {
        for (what, argv) in written_sites(phase, &seat.body) {
            collect_unpinned(&what, argv, adapters, &mut unpinned);
        }
    }
    let mut refusals = Vec::new();
    if !unpinned.model.is_empty() {
        refusals.push(format!(
            "seats {} do not pin a model; add '--model <concrete-model-id>' to each \
             driver.command (decision 0031 ruling 2)",
            labels(&unpinned.model)
        ));
    }
    if !unpinned.effort.is_empty() {
        refusals.push(format!(
            "seats {} do not pin an effort; add '--effort <level>' — one of the \
             levels that driver's adapter declares — to each driver.command \
             (decision 0035 ruling 5)",
            labels(&unpinned.effort)
        ));
    }
    if !unpinned.dsh_model.is_empty() {
        refusals.push(format!(
            "dsh seats {} carry a '--model…' word the dsh driver refuses at spawn; \
             write the pin as '--model <concrete-model-id>' and no other '--model…' \
             flag (issue #373)",
            labels(&unpinned.dsh_model)
        ));
    }
    if refusals.is_empty() {
        return Ok((
            unpinned.witnessed,
            unpinned.resume,
            unpinned.resume_witness,
            unpinned.hands_notice,
        ));
    }
    Err(CompileError::Invalid(refusals.join("; ")))
}
