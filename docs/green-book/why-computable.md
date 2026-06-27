# Why a computable version is necessary

If the PDF works for clinicians, why build a computable schedule at all? Because the hard questions are not about *reading* the schedule - they are about *evaluating a patient against it*, and that is something only a machine can do at scale, correctly, and repeatably.

## The schedule is not one document - it is a series of versions

The NHS childhood schedule has changed substantially and repeatedly:

| Year | Change |
|------|--------|
| 1992 | Hib introduced |
| 1999 | MenC introduced |
| 2006 | PCV introduced |
| 2013 | Rotavirus introduced |
| 2015 | MenB introduced |
| 2017 | 5-in-1 → 6-in-1 |
| 2019 | HPV extended to boys |
| 2020 | PCV 3+0 → 2+1 |

A child born in **1998** has a genuinely different "complete" schedule than one born in **2010** or **2020**. Any system that applies *today's* schedule to *all* patients will over- or under-flag.

## A failure mode that exists in the wild

This is not hypothetical. SystmOne Online's patient-facing childhood vaccination view has been observed presenting a naive grid of the **current** national routine schedule projected onto a patient's historical record, with due dates filled across antigens and age bands that were not all part of the schedule for that birth cohort. It is an understandable implementation shortcut - a static current-schedule table is much easier to build than a valid-time evaluator - but it is exactly the failure mode greenbook is designed to avoid.

The hard part is not drawing a table. The hard part is answering, for a real person and a real evaluation date: which schedule version applied, which products existed then, which doses were due by then, and whether the recorded history conforms to that historical schedule.

## This is a valid-time problem

The schedule is not a single document but a series of **versioned snapshots**, each with a `valid_from` date. Evaluating a patient born in 2003 requires the schedule **as it stood in 2003**, not today's schedule.

greenbook models this directly: **one file per schedule version**, dated by its `valid_from`, plus `evaluate-auto`, which builds a patient-specific effective schedule by selecting each expected dose from the version in force when that dose first became due. The first curated historical slices are now in `rules/`, with more Green Book revisions being added incrementally.

## The vision: invert the pipeline

Today the data flows one way - from a Word document, to a PDF, to hand-written code. What if the **computable schedule were the source of truth**, and everything else were generated from it?

```
Experts author the computable Schedule
        │
        ├──→  the published PDF / age-centric table   (rendered)
        ├──→  GOV.UK and other websites               (rendered)
        └──→  clinical decision support in every system (one trusted source)
```

Vaccination and public-health experts would author schedule changes **directly** in a computable format. Every downstream publication - PDFs, websites, clinical tools - would be **rendered from the same trusted data**, instead of each vendor reverse-engineering a PDF and quietly diverging.

That is the long-term goal. The rest of this site shows the [domain model](../concepts/domain-model.md) that makes it possible, and the [engine](../getting-started.md) that proves it can be evaluated.
