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

## This is a valid-time problem

The schedule is not a single document but a series of **versioned snapshots**, each with a `valid_from` date. Evaluating a patient born in 2003 requires the schedule **as it stood in 2003**, not today's schedule.

No open-source project anywhere in the world has solved this for a national schedule in a principled, versioned way. greenbook's design - **one file per schedule version**, dated by its `valid_from` - makes historical evaluation an *additive* extension rather than a rewrite: build it correctly for the current schedule first, then add older versions as files.

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
