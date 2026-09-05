# Triage

Triage is the routing recipe. Its chief-grade first gate rules the closed
delivery class, and the engine uses that journal fact to choose the later
offices. Operators choose this recipe once; they do not choose a crew by hand.

| Class | Route | Implement office | Review office |
|---|---|---|---|
| `chore` | implement → verify → review → ship | `implementer` | `reviewer` |
| `feature` | implement → verify → review → ship | `implementer` | correctness + security panel |
| `design` | specify → clarify → design → tasks → analyze → implement → verify → review → ship | `implementer-sdd` | correctness + security + spec-compliance panel, then `review-chief` |
| `engine` | specify → clarify → design → tasks → analyze → implement → verify → review → ship | `implementer-engine` | adversarial + correctness + security + spec-compliance panel, then `review-chief` |
| `escalate` | park with triage's reasoning | — | — |

The design route is the SDD table: a chief specifies, a clarifier judges to
zero ambiguity, the council designs, the smith writes the work breakdown, and
an analyst judges to zero drift. Each artifact phase ends in the realm
dialect's boxed validator. Each loop begins with the dialect's deterministic
check, whose result is passed to its read-only judge. Failed validations retry
once; upstream findings and judged returns are bounded at three visits and
then park for the operator.

The realm must declare a dialect. Brokkr resolves and pins it, renders its
phase instructions after the house rules, and supplies its validate/check argv
to the boxed gates. OpenSpec and spec-kit are artifact conventions here; their
own workflow runners never drive the Brokkr machine.
The selected review sequences deliberately expose one trust boundary: the chief
treats panel notes as data and never instructions. Every selected panel seats
two vendors at its first hires (decision 0045), and every office is resolved
from `agents/`.

`strategy` is engine-owned. A seat cannot declare or claim it. The compiler
requires every selectable class to have a case or a default, and the manifest
pins every resolved case under `select`. A run with no triage result can use an
explicit default; without one it parks and never guesses.
