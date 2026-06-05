# Queries

Open questions that came up while building the POC. Answer when convenient and I'll fold the answers back into the spec and code.

## 1. Doses re-matching across series via antigen overlap — RESOLVED

**Resolved 2026-06-02 — see [docs/adr/0001](./docs/adr/0001-product-class-conformance-vs-antigen-coverage.md).** Conformance and coverage are two separate questions. Conformance matching is now by **product class** (the unit the Green Book names), not antigen overlap, so a 6-in-1 dose matches only `6in1-primary`. Antigen overlap is retained for the deferred antigen-coverage view. None of the original options (A/B/C) below was adopted as-is; the product-class model supersedes them. Original write-up kept below for context.

---

The spec defines dose-to-series matching as: *"the vaccine code maps to at least one antigen in the series"*. As written, this means a single Infanrix Hexa (6-in-1) dose matches **three** series:

- `6in1-primary` — overlap on diphtheria, tetanus, pertussis, polio, hib, hepatitis-b
- `hib-menc-booster` — overlap on hib
- `tdap-ipv-booster` — overlap on tetanus, diphtheria, polio

In the POC output you can see the 6-in-1 doses being reported under Hib/MenC and Td/IPV as `INVALID` ("given before earliest_age"), which is correct per the literal spec but clinically wrong — those doses are clearly 6-in-1 primary doses, not failed booster attempts.

Possible resolutions, in increasing strictness:

- **A. Stricter matching:** a dose only matches a series if the product's antigen set is a superset (or equal) of the series's antigens. This stops 6-in-1 from matching Hib/MenC (because 6-in-1 lacks meningococcal-c).
- **B. Greedy assignment:** evaluate series in declared order; a dose consumed by an earlier series is removed from the pool before later series see it.
- **C. Explicit `product_filter` on a series:** the series can list permitted product codes, and the antigen overlap is only used as a fallback / cross-check.

A and B both fix the 6-in-1 case naturally. A is more declarative; B is more like ICE's behaviour. C is most flexible but adds authoring burden. My instinct is **A as default, with B as a tiebreaker** — but I'd like your call before changing the matching logic.

## 2. "Fully vaccinated" vs "fully vaccinated for age"

The current `OverallStatus::FullyVaccinated` requires every applicable series to be `Complete`. A perfectly-on-schedule 6-month-old reports as `PartiallyVaccinated` because their MenB dose 3, PCV dose 2, MMR, HPV, etc. are not yet due. Clinically you would want to say "this child is up-to-date".

Options:

- **A.** Add a second status: `UpToDateForAge` (or similar), where every dose whose `earliest_age` is on or before `evaluated_at` has been received.
- **B.** Change the meaning of `FullyVaccinated` to "up-to-date for age" and add a new `FullyImmunisedForLifeStage` for "all doses across all stages received".
- **C.** Keep the current strict definition and add per-series "due/not-yet-due" annotations so consumers can compute the clinically useful answer themselves.

A is probably the smallest change. C is most data-faithful but pushes work to consumers.

## 3. Suppressing irrelevant invalid doses in the report — RESOLVED

**Resolved 2026-06-02.** Falls out of #1: with product-class conformance matching, a dose is never re-matched into a series it doesn't belong to, so the spurious INVALID entries no longer arise and no suppression is needed. See [docs/adr/0001](./docs/adr/0001-product-class-conformance-vs-antigen-coverage.md). Original write-up kept below.

---

Related to #1 — when a dose is re-matched into a series it doesn't belong to and gets marked invalid, we currently print every such case. For a healthy 6-month-old this means the 6-in-1 doses appear three times in the report, twice as INVALID under booster series they were never intended for.

If we adopt query #1 option A or B, this disappears naturally. If we keep the current matching, we should probably suppress the noise — e.g., only report doses against the series the evaluator believes they belong to.

## 4. Sex restriction on HPV (`male_born_on_or_after`)

The schedule has it; the POC eligibility check ignores it (treats `population = "all"` as universally eligible regardless of male DOB cohort). For a 6-month-old this doesn't matter because no HPV doses have been given. Worth implementing before any test fixture reaches age 11+.

The spec's amalgamated answer says: when `Patient.gender` is `other`/`unknown`, evaluate as eligible and flag uncertainty. That's clear; I just haven't wired it up yet.

## 5. `latest_age` semantics for catch-up

The Rotavirus series has hard `latest_age` cutoffs (14w 6d for dose 1, 23w 6d for dose 2). The current implementation marks any dose given after `latest_age` as invalid. Is that what we want, or should a late-but-given dose still count as "received" (even if not "valid for course completion")? FHIR records the event happened; the schedule says it shouldn't have. I've gone with "given but not valid" but it's worth confirming.

## 6. Crate name and binary name

Both set to `greenbook` per your check. If you want a different binary name (e.g. `gb` or `greenbook-cli`) say so before there are downstream callers.

## 7. Schedule directory layout

I went with `schedules/gb/<YYYY-MM-DD>.toml` matching the spec. If you'd like the *valid_from* date to be inferable from the filename without parsing the file, that's already true. If you'd rather have e.g. `schedules/gb/current.toml` as a stable symlink/copy of the latest, that's a small addition.
