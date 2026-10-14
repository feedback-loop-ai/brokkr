# operator-console-resume Specification

## Purpose
Let the console resume a run under the recipe its start recorded, with the
run view naming that recipe, so resuming needs no setting and a state the
console cannot prepare is answered as unsent.

## Requirements

### Requirement: The run view names the recipe the run started under

`RunView` SHALL carry `recipe`: the `bundle_name` recorded in the run's `run/started` manifest, or none when the first event is not a start or names none. `inspect --json` SHALL emit it, and the view's wire version SHALL be 20.

#### Scenario: A run started from a recipe

- **WHEN** a run's `run/started` manifest records `bundle_name` `triage`
- **THEN** its view's `recipe` is `triage`

#### Scenario: A journal with no start

- **WHEN** a run's first event is not `run/started`
- **THEN** its view's `recipe` is none

### Requirement: The console resumes under the run's own recipe

The console SHALL resume, after a retry or on Resume, with `--recipe <recipe>` from the run's view. `BROKKR_CONSOLE_RESUME_BUNDLE` and then `BROKKR_CONSOLE_RESUME_RECIPE` SHALL override it. With no override and no recipe in the view, it SHALL send nothing and say why.

#### Scenario: No settings

- **WHEN** neither setting is set and the view names recipe `landing`
- **THEN** resume runs `brokkr resume --recipe landing`

#### Scenario: A state folder the console cannot prepare

- **WHEN** the console's state folder cannot be made or is not private
- **THEN** the command answers "not sent: unstarted" with the reason, and no process starts

### Requirement: Analytics absences are typed

`ModelShare.model` and `RuleCount.rule_id` SHALL be absent (`null`) where no model or rule is recorded, not a placeholder string. Their text fields SHALL keep the words a reader sees.

#### Scenario: A seat that reports no model

- **WHEN** a seat's records name no model
- **THEN** its share's `model` is `null` and its text says "not reported"

## Provenance

- `2026-10-14-568-operator-console-slice-two` — folded 2026-10-14
