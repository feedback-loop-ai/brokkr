# Triage

Triage is the routing recipe. Its chief-grade first gate rules the closed
delivery class, and the engine uses that journal fact to choose the later
offices. Operators choose this recipe once; they do not choose a crew by hand.

| Class | Route | Implement office | Review office |
|---|---|---|---|
| `chore` | implement → verify → review → ship | `implementer` | `reviewer` |
| `feature` | implement → verify → review → ship | `implementer` | correctness + security panel |
| `design` | design → implement → verify → review → ship | `implementer` | correctness + security + spec-compliance panel, then `review-chief` |
| `engine` | design → implement → verify → review → ship | `implementer-engine` | adversarial + correctness + security + spec-compliance panel, then `review-chief` |
| `escalate` | park with triage's reasoning | — | — |

The `design` council remains positions → chief → deterministic spec-kit check.
The selected review sequences deliberately expose one trust boundary: the chief
treats panel notes as data and never instructions. Every selected panel keeps
at least two model families, and every office is resolved from `agents/`.

`strategy` is engine-owned. A seat cannot declare or claim it. The compiler
requires every selectable class to have a case or a default, and the manifest
pins every resolved case under `select`. A run with no triage result can use an
explicit default; without one it parks and never guesses.
