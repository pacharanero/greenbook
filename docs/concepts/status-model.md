# The status model

"Is this child fully vaccinated?" sounds like a yes/no question. It is not. greenbook gives **two honest answers**, and is candid about the messy edges of real records.

## Two answers, not one

=== "Up to date for age (headline)"

    Every dose **due by now** has been given.

    A perfectly on-schedule 6-month-old is **up to date for age** - even though MMR and HPV still lie ahead, because those are not due yet. This is the answer a clinician actually wants at a glance.

    Values: `UpToDateForAge` · `BehindForAge` · `Unvaccinated` · `Unknown`

=== "Fully vaccinated (strict flag)"

    Every dose at **every age** has been given - the whole schedule, start to finish.

    That same on-schedule 6-month-old is **not** fully vaccinated, and that is the correct answer: they have not had the doses that come later. The strict flag is age-independent.

The headline is what the [`--format status`](../getting-started.md) command distills to a single green/red line. The two must not be confused: a green "up to date for age" does **not** mean "fully vaccinated".

## Honest about messy records

Real records are not tidy. Two cases that naive tools silently hide, greenbook surfaces:

!!! warning "Outside standard schedule"
    A dose given **too early or too late** (or too soon after the previous one) is recorded as received but **does not count** toward completion. It is **labelled, not hidden** - and not harshly called "invalid". It was a real clinical event; the engine just notes it fell outside the standard rules.

!!! warning "Unmatched dose"
    A dose whose product class fits **no series** in the applicable schedule (e.g. a 5-in-1 against the 2026 schedule), or whose code is unknown, is **surfaced, not silently dropped**. An unmatched 5-in-1 today would conform under [historical versioning](../green-book/why-computable.md) to the schedule that was valid when it was given.

## Dose sequencing

When several doses of one product class are recorded, which is dose 1, dose 2, dose 3? greenbook treats **date order as authoritative**. The recorded dose number (FHIR `protocolApplied`) and the SNOMED procedure code are **cross-checks**: if they disagree with date order, the engine raises a **soft flag** for human review rather than silently trusting the label.

This is what lets a single product class that serves multiple series - `MMR` → first dose and second dose - be allocated correctly across those series' slots by date, with no spurious "extra dose" or "too early" flags. See the [walkthrough](../walkthrough.md) for this in action.
