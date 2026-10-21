# Design — another build's reservation (#772)

Decision 0090 carries the rulings. This design carries what the code does
and the evidence behind it.

## D1. What fails, and why

`inventory::run` read each run directory's `reservation.json` through
`root::read::<Reservation>`. `Reservation` and everything beneath it carry
`deny_unknown_fields`, and `Reach::Boxed.sealed` is `Box<hands::Sealed>`
since U8a2c. From U8a2a's first build until then, `sealed` was a list of
`{path, identity: [dev, ino] | null}`. Any parse failure became
`Unknown::Participant(<path>)` before any lease or reach comparison, so a
disjoint run in another realm refused this one.

The reproduction is the mutation in task 4.1: with the view disabled,
every foreign test fails, along with the widened-foreign assertion in
`an_unknown_participant_refuses_before_anything_is_sealed`.

## D2. The view

`inventory/standing.rs` holds `Standing`: `repo`, `run`, `artifacts`,
`journal`, `attempts` (each an `Attempt` of `protects`, `writers`,
`cleanup`, `serves`), `settled`, and `whole: Option<Reservation>`, never
deserialized. A writer is `Writing { site, reach: Reaching }`, and
`Reaching` is `Boxed { writable, sources }` or `Unbounded`. None of these
types denies unknown fields: another build's record is not closed to this
one (decision 0071 ruling 3 asks for `deny_unknown_fields` only where the
contract is closed).

`protects` and `cleanup` reuse the strict vocabularies, `Protects` and
`Witnessed`, through one generic, `Tolerated<T>`, which is `Known(T)` or
`Unrecognised(IgnoredAny)`, untagged. One home per vocabulary (ruling 5).
Each reader matches exhaustively: `Attempt::protects` reads `Unrecognised`
as evidence, and `Attempt::proved` reads it as unproven.

`Reaching` has no catch-all arm. A reach of a kind this build does not
know gives it no writable roots to compare, so the file refuses by name.
Reading it as unbounded would have tolerated it where nothing is at stake.
Refusing is the stricter reading, and the commission lists an
unrecognised reach among the unreadable cases.

`From<Reservation> for Standing` and `From<&Writer> for Writing` are the one
conversion. The comparison (`exclusion.rs`), the sources' comparison
(`sources.rs`) and recovery (`settlement.rs`) consume only `Standing` and
`Writing`. The incoming claim's writers are converted once where they are
compared. The `writers`, `protects` and `serves` accessors move from
`Reservation` to `Standing`, and `Reservation` keeps none.

## D3. Which record is read how

`inventory(root, at, own, budget)` takes this claim's `[repo, run]`.
`worktree` marks the run directory whose names equal it, and
`standing::standing(dir, at, own)` reads it:

1. the bytes, once, within MB3's request bound (`root::bytes`, split out
   of `root::read`);
2. whole as `Reservation` where they parse, which gives `whole: Some`;
3. this run's own: refused as `Participant` otherwise;
4. else the `format` probe, refused as `Formatted(path, n)` for any `n` but
   1;
5. else the view, refused as `Participant` where it does not parse.

`joined` and `extended_with` take this run's record from `whole`, and
inventory guarantees it there for this run's own directory.

## D4. Recovery

`recovered` certifies each standing record's proven attempts as before.
A record with `whole` is settled, sealed and re-derived. A record without
it is `released` in memory, with nothing written (decision 0090 ruling 3,
option b of the framing). Option a, leaving it standing, was not taken.
It fails closed too, but a dead older-build attempt on the same worktree
would then hold the worktree until that build concluded it, though the
durable facts already prove it over.

## D5. Format

Decision 0090 ruling 4. The probe is a struct of one optional field, so a
record of any shape names its format before the view is asked to read it.
A format the view does not know therefore refuses by its number, never as
a parse failure.
