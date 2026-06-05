# greenbook

A Rust library and CLI that evaluates a patient's FHIR vaccination history against a computable, versioned representation of the NHS routine childhood immunisation schedule (the [Green Book](https://www.gov.uk/government/publications/immunisation-schedule-the-green-book-chapter-11)), reporting whether they are up-to-date for their age, which doses are missing, and (strictly) whether every dose the schedule defines has been given - per series and in aggregate.

For the UK, the childhood vaccination schedule is currently published as human-readable PDF files that serve as the **primary source of truth**. As of 2026 there is no computable version of the schedule. This prototype is an attempt to explore a proof-of-concept for a **computable schedule format** and **evaluation engine**, with the long-term goal of creating a versioned, computable Green Book that can be maintained in parallel with the human-readable version and eventually replace it as the upstream primary publication and 'source of truth', with downstream PDFs being generated from the computable version rather than the other way around.

### What is the long-term potential of getting this right?

The long-term goal is that vaccination and public health experts could eventually author schedule changes **directly** in a computable format, and all downstream publications - PDFs, websites, and of course clinical digital tools - would be generated from this one 'upstream' and trusted computable source. It would replace the current 'digital paper' workflow where a PDF is published from a Word document and clinical code has to be written as a (potentially inaccurate) reverse-engineered derivative of it.

See [spec/](./spec/) for the full specification, [docs/testing.md](./docs/testing.md) for a walkthrough of the POC, and [queries.md](./queries.md) for open design questions.

---

## Problem Statement

### Why this is non-trivial

To determine whether a person is "fully vaccinated" you need to know:

1. What schedule applied when they were born (and what was in scope by the time each dose was due)
2. What they actually received
3. Whether what they received satisfies the schedule that applied to them

The NHS childhood schedule has changed substantially and repeatedly. A child born in 1998 has a genuinely different "complete" schedule than one born in 2010 or 2020. Any system that applies today's schedule to all patients will over- or under-flag.

Examples of schedule changes:

- Hib introduced 1992
- MenC introduced 1999
- PCV (pneumococcal) introduced 2006; schedule changed again in 2020 (3+0 to 2+1)
- Rotavirus introduced 2013
- MenB introduced 2015
- HPV introduced 2008 for girls only; extended to boys 2019
- 5-in-1 replaced by 6-in-1 in 2017 (adding hepatitis B)
- Flu extended incrementally to primary school age from ~2013

### Historical versioning is the hard part

This is a **valid-time** data problem. The schedule is not a single document but a series of versioned snapshots, each with a `valid_from` date. Evaluating a patient born in 2003 requires the schedule as it stood in 2003, not today's schedule.

No open-source project anywhere in the world has solved this for a national schedule in a principled, versioned way. The closest existing work is the US CDC's **[CDSi](https://www.cdc.gov/vaccines/programs/iis/interop-proj/cds.html)** (Clinical Decision Support for Immunization) specification and the **[ICE](https://github.com/cdsframework/ice)** (Immunization Calculation Engine) open-source implementation - but both are US-only (ACIP schedule) and both are current-schedule-forward systems, not historically aware.

**The computable, versioned Green Book does not exist. This project creates it.**

### Scope for v1

Start with the current schedule only. Build the evaluation engine and file format correctly from the outset so that historical versioning is an additive extension, not a rewrite. The data management and format design choices made now determine whether the historical problem is tractable later.

---

## Prior Art

### ICE - Immunization Calculation Engine

- GitHub: [`cdsframework/ice`](https://github.com/cdsframework/ice)
- Maintained by NYC Dept of Health and HLN Consulting
- Free and open source ([LGPL-3.0](https://www.gnu.org/licenses/lgpl-3.0.html))
- Docker image available; actively maintained as of 2026
- ACIP (US) schedule only; current-schedule-forward
- Architecture is worth studying: separates schedule data (per-antigen configuration files) from the evaluation engine ([Drools](https://www.drools.org/) rules)
- Recognised as a Digital Public Goods Standard by the [DPGA](https://digitalpublicgoods.net/) (2021)

### CDC CDSi

- Published by [CDC](https://www.cdc.gov/vaccines/programs/iis/interop-proj/cds.html) as an implementation-neutral specification
- Logic Specification (PDF) plus Supporting Data (XML/Excel per antigen)
- Versioned document (currently v4.6) updated after each [ACIP](https://www.cdc.gov/vaccines/acip/index.html) recommendation
- Useful as a design pattern reference; not directly applicable to UK

### UK NHS - what exists

- [Green Book Chapter 11](https://www.gov.uk/government/publications/immunisation-schedule-the-green-book-chapter-11): the authoritative UK schedule, published as versioned PDFs on [GOV.UK](https://www.gov.uk/). Change history visible in page revision notes but not structured data.
- [`NHSDigital/immunisation-fhir-api`](https://github.com/NHSDigital/immunisation-fhir-api): NHS Digital's FHIR R4 Immunisation API (Python, AWS Lambda backend). Handles CRUD for vaccination records. No schedule evaluation logic.
- [`NHSDigital/FHIR-R4-UKCORE-STAGING-MAIN`](https://github.com/NHSDigital/FHIR-R4-UKCORE-STAGING-MAIN): UK Core FHIR profiles including `UKCore-Immunization`.
- No open-source computable UK schedule exists.
