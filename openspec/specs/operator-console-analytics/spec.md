# operator-console-analytics Specification

## Purpose
Give the operator console, and scripts, one cross-run readout of what the
runs did, derived only from what the journals record, with what no journal
records listed as such and every absence typed rather than worded.

## Requirements

### Requirement: Analytics comes from one derivation in brokkr-view

A pure aggregation in `brokkr-view` SHALL compute the metrics over the
hearths `realms.json` names, for a range of 24 hours, 7 days or 30 days, and
optionally one realm. `brokkr analytics --json` SHALL emit it. The console
SHALL only paint it.

#### Scenario: One number, two surfaces

- **WHEN** the readout and the console show the same range and realm
- **THEN** every figure is identical

### Requirement: The step-1 metrics

The aggregation SHALL provide:

- **Headline:**
  - runs shipped, and the change against the previous equal range;
  - median start → end, including parked time;
  - hours waiting on the operator (from a park to the operator's next
    command), and their share of all parked time;
  - runs stopped by a rule, with the most frequent rule id;
  - reported cost in USD, naming the harnesses that report none.
- **Charts:**
  - runs finished per day, shipped versus stopped;
  - hours and runs waiting on the operator, by the step they parked in;
  - rework: implement visits per run, runs sent back by review, and the
    share that passed review on the second try;
  - the share of step attempts by served model.
- **By realm:** shipped, stopped, median start → end, hours waiting on the
  operator, reported cost.

#### Scenario: Cost from harnesses that report none

- **WHEN** the range includes codex and dsh seats
- **THEN** reported cost covers only the reporting harnesses, and the realm row reads "not reported" when none of its seats reported

### Requirement: What is not recorded is said, not invented

The aggregation SHALL NOT produce seat utilisation, host capacity, cost-cap
stops, converted currencies, token totals or finding-dismissal categories.
The console SHALL list them under "not recorded yet".

#### Scenario: The footer

- **WHEN** the Analytics view renders
- **THEN** it ends with "not recorded yet: machine capacity over time · cost limits · costs for Codex and dsh · reasons findings were dismissed"

## Provenance

- `2026-10-13-568-operator-console-slice-one` — folded 2026-10-14
