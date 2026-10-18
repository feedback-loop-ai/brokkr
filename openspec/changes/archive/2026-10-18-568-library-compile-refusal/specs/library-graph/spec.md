## ADDED Requirements

### Requirement: A recipe that does not compile is drawn with its refusal

`brokkr graph` SHALL compile every recipe the realm resolves by name.

A recipe that resolves and does not compile against this Brokkr SHALL NOT fail the verb. Its view's `table` SHALL be `refused`, carrying:
- Brokkr's words naming the recipe;
- the compiler's reason, without the `bundle: ` words that wrap it;
- the bundle directory it resolves to, named in the realm's folder when it lies there.

The view SHALL then draw only the rules its runs fired, as for a table not found. Every view SHALL carry `does_not_compile`, the realm's recipes that do not compile, sorted. A failure to compile that is not the recipe's own refusal SHALL still fail the verb. The graph's wire version SHALL be 2.

#### Scenario: An adapter written for an older Brokkr

- **WHEN** the realm's `claude` adapter carries `binding_grant`, a key this Brokkr does not know, and `--recipe starter` is asked
- **THEN** the verb prints a view whose `table` is `refused` with the text "starter doesn't compile against this Brokkr", the reason "adapter 'claude' (<path>) has unknown key 'binding_grant'; known keys: …", and the bundle `.brokkr`, and `does_not_compile` is `["starter"]`

#### Scenario: A realm's dialect that cannot be read

- **WHEN** the realm's map names a dialect that does not exist
- **THEN** the verb fails, and names no recipe as not compiling
