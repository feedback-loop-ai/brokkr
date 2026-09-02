# The name

This engine was renamed to **Brokkr** — in the myth, the dwarf whose
whole task was to work the bellows and not stop; Loki, as a biting fly,
made him flinch once, and Mjölnir's handle came out short. Steadiness
under distraction, and the cost of one lapse, is this engine's core loop
told as a story a thousand years old. The old name was also the most
collided word in software, and so was never findable.

**"Forge" survives as the verb.** Slices are forged, runs are forged,
Brokkr forges. The proper noun retired from the marquee, not from the
vocabulary — and the mechanism keeps its plain names, so a new operator
can still guess what a command does with no glossary: `.forge/`,
`forge.db`, `refs/forge/`, the wire protocols.

**The binary is `brokkr`, and now the only one.** The `forge` shim that
rode along for one release is gone, and the crates are `brokkr-*`.
Environment override names carry one-release legacy fallbacks documented
in the [versioning guide](../guides/versioning.md), and the `{forge}`
token in bundle argv still answers to `{brokkr}` for the same window.

[Decision 0019](../decisions/0019-brokkr.md) is the ruling, with the
reasoning and the five laws that bound it. [The Edda](edda.md)
is the lore layer those laws govern: commentary, never specification —
if it burned, the constitution would still be whole.

The [sagas](sagas/) are this repository's own stories under those same
laws.
