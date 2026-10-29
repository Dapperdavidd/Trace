# ADR 0003: Score weights and thresholds are unvalidated defaults

Status: accepted, 2026-10-28

## Context
WQS, WCS, TCQ, Trading DNA, discovery thresholds and Top Movers filters all need numeric weights and cutoffs. No labelled historical data exists yet to validate any of them.

## Decision
- Every score carries a version string and its weights are public constants in code.
- Defaults are labelled `unvalidated` in API output and docs. They are rules, not probabilities.
- Missing inputs lower confidence or yield `insufficient_history`. They never default to a neutral or good value.
- Changing a weight means a new score version. Old results keep the version that produced them.
- Calibration against real outcomes is a later milestone and needs real fixtures and outcome history.

## Consequences
Scores are explainable and reproducible. The UI must show the version and the confidence label, not a bare number.
