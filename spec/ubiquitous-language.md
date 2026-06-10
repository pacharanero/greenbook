# Ubiquitous Language

The shared vocabulary of greenbook. Terms are **bold** on first use elsewhere in the docs. Where the team has used several words for one concept, the canonical term is given and the rejected ones listed as aliases to avoid.

## The schedule

| Term | Definition | Aliases to avoid |
| --- | --- | --- |
| **Schedule** | A versioned, computable set of recommended immunisations for one jurisdiction, effective from a given date. | programme set |
| **Schedule version** | A single dated snapshot of a schedule, stored as one file named for its `valid_from` date. | edition, revision |
| **Series** | One vaccination programme within a schedule (e.g. 6-in-1 primary, MMR second dose), comprising an ordered set of doses. | programme, course |
| **Dose** | A single *expected* administration within a series, carrying target / earliest / latest age and a minimum interval. | appointment, jab |
| **Eligibility** | The predicate deciding whether a patient is in scope for a series (population, sex, birth cohort). | applicability |
| **Jurisdiction** | The country and authority whose schedule this is (e.g. UK / UKHSA). The UK code is the ISO 3166 exceptionally-reserved `UK`, used over the primary alpha-2 `GB` so the UK-wide scope (incl. Northern Ireland) is unambiguous. | region, locale |

## Products and antigens

| Term | Definition | Aliases to avoid |
| --- | --- | --- |
| **Antigen** | A disease target that vaccination protects against (diphtheria, Hib, measles). | disease, pathogen, vaccine target |
| **Product** | A specific marketed vaccine identified by a SNOMED code (e.g. Infanrix Hexa). | vaccine, brand, jab |
| **Product class** | The conformance unit the Green Book names (e.g. "6-in-1", "MMR", "Td/IPV"), grouping interchangeable products. | vaccine type, category |
| **Product map** | The file mapping each product code to its product class and the antigens it covers. | product table, vaccine mapping |
| **Antigen registry** | The set of antigens a schedule defines, referenced by its series. | antigen list |

## Evaluation

| Term | Definition | Aliases to avoid |
| --- | --- | --- |
| **Conformance** | Whether a patient received the doses the Green Book named, at valid ages and intervals; matched by **product class**. | compliance, adherence |
| **Coverage** | Which diseases a patient is protected against, computed across the **antigens** of every product received. | protection, immunity |
| **Matching** | Assigning a recorded dose to a series by equality of **product class**. | antigen overlap (this *was* the matching rule; it is now coverage-only) |
| **Recorded dose** | A vaccination event present in the patient's record. | administered dose, given dose |
| **Valid dose** | A recorded dose that falls within the standard schedule for its assigned series (on or after `earliest_age`, interval met, not past `latest_age`). Counts toward completion. | counted dose |
| **Outside standard schedule** | A recorded dose that was given but breaks an age or interval rule - too early *or* too late. Recorded as received; does not count toward completion. Preferred over "invalid", because the dose is a real clinical event. | invalid dose |
| **Unmatched dose** | A recorded dose whose product class fits no series in the applicable schedule, or whose code is unknown. | orphan dose |
| **Dose sequencing** | Determining which dose number a recorded dose represents. **Date order is authoritative**; the recorded dose number and procedure code are cross-checks that **flag** disagreement, not override. | dose numbering |
| **Procedure code** | The SNOMED *procedure* code (UKCore-VaccinationProcedure extension) recorded at administration, which can name the dose ("...second dose..."). Distinct from the dm+d **product** code. Human-entered. | vaccination procedure |
| **Duplicate dose / echo** | The same physical vaccination recorded twice from different systems, often with different dates. Detected by a shared **procedure code**; the earliest is kept, the rest reported as duplicates, not counted. | repeat dose |
| **Flag** | A soft cross-check warning on a recorded dose (e.g. recorded dose number disagrees with date order). Does *not* affect validity; surfaced for human review. | error, warning |
| **Programme** | All series sharing one **product class** (e.g. the two `MMR` series), evaluated together so the class's doses are allocated across the series' slots by date - one dose per slot. | course |
| **Due / not-yet-due** | Whether an expected dose's age has been reached by the evaluation date. The split that makes "up-to-date for age" computable. | overdue |
| **Series completion status** | A series' standing: `Complete`, `Partial`, `None`, or `NotApplicable`. | series result |
| **Up-to-date for age** | The **headline** status: every dose *due by the evaluation date* has been received and is valid; not-yet-due doses are not held against the patient. | on track |
| **Fully vaccinated** | A strict, age-independent flag: every applicable series is `Complete` (every dose at every age received and valid). Distinct from up-to-date-for-age; reported as an explicit flag, not the headline term. | fully immunised for life stage |
| **Overall status** | The headline age-relative determination: `UpToDateForAge`, `BehindForAge`, `Unvaccinated`, or `Unknown` - reported alongside the strict `fully_vaccinated` flag. | summary |

## Patient input

| Term | Definition | Aliases to avoid |
| --- | --- | --- |
| **Patient** | The individual being evaluated; source of date of birth and sex. | subject |
| **Immunisation** | A single vaccination event in the record (en-GB spelling; FHIR's resource is `Immunization`). | shot |
| **Vaccination record** | A patient together with their immunisations, parsed from a FHIR bundle. | patient record, history |
| **FHIR bundle** | The input document carrying one Patient and zero or more Immunizations. | input file |

## Publication and lifecycle (some deferred)

| Term | Definition | Aliases to avoid |
| --- | --- | --- |
| **Green Book** | The authoritative UK immunisation schedule publication (Chapter 11). | the guidance |
| **Source of truth** | The authoritative upstream artefact; the project's goal is for the computable schedule to be this, with the PDF generated *from* it. | master copy |
| **Render** | Pivoting a schedule from series-centric (authoring) to age-centric (publication) form. | export |
| **Historical versioning** | Selecting the schedule version that applied at a point in time (valid-time evaluation). | back-dating |
| **Catch-up schedule** | The alternative schedule for a patient presenting late. | late schedule |
| **At-risk rules** | Higher-priority series overriding the primary schedule for special clinical groups. | special cases |

## Relationships

- A **Schedule** contains many **Series**; each **Series** has an ordered set of **Doses**.
- A **Series** references exactly one **Product class** (for conformance) and one or more **Antigens** (for coverage).
- A **Product class** groups one or more **Products**; a **Product** covers one or more **Antigens**.
- A **Product class** may serve more than one **Series** — e.g. `MMR` maps to both the MMR first-dose and MMR second-dose series.
- A **Recorded dose** is matched to at most one **Series** by **Product class**; if none matches, it is an **Unmatched dose**.
- **Conformance** is determined per **Series**; **Coverage** is determined per **Antigen**.
- A **Schedule version** is chosen by its `valid_from` relative to the **Patient**'s date of birth (historical) or to the evaluation date.

## Example dialogue

> **Dev:** "A child got one Infanrix Hexa dose. That product covers Hib, so does it count toward the Hib/MenC booster **series**?"

> **Clinical informaticist:** "No. For **conformance** we match by **product class**, not **antigen**. Infanrix Hexa is class `6-in-1`, so it only counts toward the `6in1-primary` series. The Hib/MenC booster is class `Hib/MenC` — only a Menitorix dose conforms there."

> **Dev:** "But the child *is* protected against Hib after that dose?"

> **Clinical informaticist:** "Right — that's **coverage**, a separate question answered over **antigens**. Hib coverage is satisfied; Hib/MenC booster conformance is not. Two questions, two answers."

> **Dev:** "And if they were given a 5-in-1 dose, which has no class in the 2026 schedule?"

> **Clinical informaticist:** "Against that **schedule version** it's an **unmatched dose** — it conforms to nothing here. Under **historical versioning** it would conform to the schedule that was valid when it was given."

> **Dev:** "Last one — `MMR` is one **product class** but there are two MMR **series**, first and second dose?"

> **Clinical informaticist:** "Yes. One class can serve several series; the **doses** and their ages distinguish them. Series is the programme unit; product class is the conformance key — don't conflate them."

## Flagged ambiguities

- **"coverage"** was originally used broadly ("comparing coverage… two different levels"). It is now reserved strictly for the **antigen / disease** view. The schedule-adherence sense is **Conformance**. See [conformance vs coverage](./conformance-vs-coverage.md).
- **"matching"** meant *antigen overlap* in the POC; it now means **product-class** matching for conformance. Antigen overlap is retained but only feeds **Coverage** — do not call it "matching".
- **"fully vaccinated"** was overloaded: "all applicable series `Complete`" versus "up-to-date for age". **Resolved**: the two senses are reported separately. **Up-to-date for age** is the headline age-relative status (`OverallStatus`); **Fully vaccinated** is retained as a strict, age-independent flag. A "fully immunised for life stage" status was considered and rejected as an unstable moving target. The loose clinical phrase "fully vaccinated" is therefore never the headline term here.
- **"dose"** spans *expected* dose (defined in a series) and *recorded* dose (administered, in the record). Qualify which is meant; use **Recorded dose** / **Valid dose** for the administered side.
- **Series vs product class** are 1:1 in the current GB data, which invites conflation. They are distinct: a series is the authoring/programme unit; a product class is the conformance key. `MMR` (one class, two series) is the canonical disambiguator.
- **"Immunisation" vs "Immunization"** — en-GB is the domain spelling and used in our own types; the American `Immunization` is a FHIR wire-format resource name only.
