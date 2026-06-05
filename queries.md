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

## 2. "Fully vaccinated" vs "fully vaccinated for age" — RESOLVED

**Resolved 2026-06-05.** Two complementary determinations, reported together: **up-to-date for age** is the headline status (`OverallStatus = UpToDateForAge | BehindForAge | Unvaccinated | Unknown`), and a strict **`fully_vaccinated`** flag (every applicable series `Complete`, age-independent) is retained alongside it. A "fully immunised for life stage" status was considered and rejected as an unstable moving target. The four consumer needs below are captured: needs 1-2 are served by the headline status plus the per-series gap breakdown; needs 3 (predicted future schedule) and 4 (record-error detection) are tracked on the [roadmap](spec/roadmap.md) M3. Folded into [spec/standard.md](spec/standard.md) §"Overall status". Original write-up kept below.

---

The current `OverallStatus::FullyVaccinated` requires every applicable series to be `Complete`. A perfectly-on-schedule 6-month-old reports as `PartiallyVaccinated` because their MenB dose 3, PCV dose 2, MMR, HPV, etc. are not yet due. Clinically you would want to say "this child is up-to-date".

Options:

- **A.** Add a second status: `UpToDateForAge` (or similar), where every dose whose `earliest_age` is on or before `evaluated_at` has been received.
- **B.** Change the meaning of `FullyVaccinated` to "up-to-date for age" and add a new `FullyImmunisedForLifeStage` for "all doses across all stages received".
- **C.** Keep the current strict definition and add per-series "due/not-yet-due" annotations so consumers can compute the clinically useful answer themselves.

A is probably the smallest change. C is most data-faithful but pushes work to consumers.

> New vaccines are always being added, including for older adults (e.g. shingles, pneumococcal for 65+), so the "fully immunised for life stage" status is a moving target that would require maintenance. The "up-to-date for age" status is more stable and clinically useful, so I propose we use that. Fully vaccinated may be a clinical shorthand for it, but it isn't precise enough for us here.

> Thinking about the needs of a consumer of this tool: They will want to know 
> 1. Is this child correctly vaccinated for their age (ie are there gaps)? (eg a clinician seeing the child with an illness in ED)
> 2. What are the specific gaps (ie which series/doses are missing)? (eg a nurse arranging catchup planning)
> 3. What would be their 'predicted future vaccination schedule' (assuming no future change to the schedule) -  eg a parent might be interested
4. Are there errors (such as duplicates) in the record of vaccination schedule - parent or patient might point this out.

## 3. Suppressing irrelevant invalid doses in the report — RESOLVED

**Resolved 2026-06-02.** Falls out of #1: with product-class conformance matching, a dose is never re-matched into a series it doesn't belong to, so the spurious INVALID entries no longer arise and no suppression is needed. See [docs/adr/0001](./docs/adr/0001-product-class-conformance-vs-antigen-coverage.md). Original write-up kept below.

---

Related to #1 — when a dose is re-matched into a series it doesn't belong to and gets marked invalid, we currently print every such case. For a healthy 6-month-old this means the 6-in-1 doses appear three times in the report, twice as INVALID under booster series they were never intended for.

If we adopt query #1 option A or B, this disappears naturally. If we keep the current matching, we should probably suppress the noise — e.g., only report doses against the series the evaluator believes they belong to.

## 4. Sex restriction on HPV (`male_born_on_or_after`) — RESOLVED (spec); implementation pending

**Resolved 2026-06-05.** The rule is settled in the spec: check `population` and `male_born_on_or_after`, and when `Patient.gender` is `other`/`unknown` treat the patient as eligible and attach an uncertainty flag ([spec/standard.md](spec/standard.md) §"Eligibility check"). The code does not yet enforce it; that is roadmap M2. Original write-up kept below.

---

The schedule has it; the POC eligibility check ignores it (treats `population = "all"` as universally eligible regardless of male DOB cohort). For a 6-month-old this doesn't matter because no HPV doses have been given. Worth implementing before any test fixture reaches age 11+.

The spec's amalgamated answer says: when `Patient.gender` is `other`/`unknown`, evaluate as eligible and flag uncertainty. That's clear; I just haven't wired it up yet.

## 5. `latest_age` semantics for catch-up — RESOLVED

**Resolved 2026-06-05.** A dose that breaks an age or interval rule - too early *or* too late - is recorded as received but labelled **"outside standard schedule"** (preferred over "invalid", since the dose is a real clinical event) and does not count toward series completion. For hard cutoffs like rotavirus this non-counting is clinically required; for marginal cases it is a flag for human review. Folded into [spec/standard.md](spec/standard.md) §"Dose validity". Original write-up kept below.

---

The Rotavirus series has hard `latest_age` cutoffs (14w 6d for dose 1, 23w 6d for dose 2). The current implementation marks any dose given after `latest_age` as invalid. Is that what we want, or should a late-but-given dose still count as "received" (even if not "valid for course completion")? FHIR records the event happened; the schedule says it shouldn't have. I've gone with "given but not valid" but it's worth confirming.

> For other instances of this type of vaccinated-but-too-late scenario, we have used "outside of standard schedule" or similar. It can also happen too early as well as too late. 

## 6. Crate name and binary name — RESOLVED

**Resolved 2026-06-05.** `greenbook` for both crate and binary; verified free on crates.io. If the CLI is later split into its own crate the binary can be renamed then. Original write-up kept below.

---

Both set to `greenbook` per your check. If you want a different binary name (e.g. `gb` or `greenbook-cli`) say so before there are downstream callers.

> greenbook all the way unless already taken on crates.io. It's short, descriptive, and memorable. If we later want to split the CLI into a separate crate we can rename the binary then, but for now it's simpler to keep them the same.

## 7. Schedule directory layout — RESOLVED

**Resolved 2026-06-05.** Flat layout while only one jurisdiction has data: `schedules/uk-<YYYY-MM-DD>.toml`, product map `products/uk-snomed-dm.toml`. It splits into per-country subdirectories when a second jurisdiction is added, with no format change. The jurisdiction code is **`UK`** (ISO 3166 exceptionally-reserved) rather than the primary alpha-2 `GB`, because the Green Book is UK-wide and "Great Britain" reads as excluding Northern Ireland. Folded into [spec/standard.md](spec/standard.md) §"Directory structure". Original write-up kept below.

---

I went with `schedules/gb/<YYYY-MM-DD>.toml` matching the spec. If you'd like the *valid_from* date to be inferable from the filename without parsing the file, that's already true. If you'd rather have e.g. `schedules/gb/current.toml` as a stable symlink/copy of the latest, that's a small addition.

> I'd avoid nesting folders by jurisdiction until we have more than one jurisdiction. The spec is designed to be global from the outset, but the directory structure can be adapted when we have more than one schedule to manage. For now, `schedules/uk-<YYYY-MM-DD>.toml` is clear and simple.
