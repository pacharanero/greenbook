# The domain model

The engine's correctness rests on a small, deliberate vocabulary. Getting these terms - and the distinctions between them - right is most of the work. (The full glossary lives in [`spec/ubiquitous-language.md`](https://github.com/pacharanero/greenbook/blob/main/spec/ubiquitous-language.md).)

## Schedule → Series → Dose

The schedule is a nested structure, authored series-by-series:

- **Schedule** - a versioned set of recommended immunisations for one **jurisdiction**, effective from a date.
- **Series** - one vaccination programme within it ("6-in-1 primary", "MMR second dose"): an ordered set of doses.
- **Dose** - one *expected* administration, with a target / earliest / latest age and a minimum interval from the previous dose.

It is authored **series-centric** (the natural way to reason about a programme). The familiar **age-centric** table - "at 8 weeks, give X, Y, Z" - is *rendered* from it for publication, not authored directly.

## Products and antigens

- **Antigen** - a **disease target** that vaccination protects against: diphtheria, Hib, measles.
- **Product** - a specific **marketed vaccine**, identified by a SNOMED code: e.g. Infanrix Hexa.

One product covers one or more antigens. The **product map** records that bridge:

> **Infanrix Hexa** (6-in-1) → diphtheria · tetanus · pertussis · polio · Hib · hepatitis B

## Product class - the conformance key

The Green Book does not name brands. It names a **product class**: "6-in-1", "MMR", "Td/IPV".

- A product class **groups interchangeable products** - Infanrix Hexa and Vaxelis are both `6-in-1`.
- It is the **conformance key**: how a recorded dose is matched to a series.
- One class can serve **several series** - `MMR` maps to both the first-dose and second-dose series; the ages tell them apart.

!!! tip "The distinction that prevents the central bug"
    **A series is the programme unit. A product class is the conformance key. Don't conflate them.** Matching doses to series by *antigen overlap* instead of *product class* is the original bug - it drags a 6-in-1 primary dose into a later booster series because they share Hib. See [conformance vs coverage](conformance-vs-coverage.md).

## How they fit together

```
Schedule (jurisdiction, valid_from)
└── Series  "6-in-1 primary"      ── conforms to product class: 6-in-1
    ├── Dose 1   target 8w,  earliest 6w
    ├── Dose 2   target 12w, min interval 4w
    └── Dose 3   target 16w, min interval 4w

Product map
└── Infanrix Hexa (SNOMED ...)  ── class: 6-in-1  ── antigens: diphtheria, tetanus, pertussis, polio, Hib, hepatitis B
```

A recorded dose of Infanrix Hexa is class `6-in-1`, so it conforms to the `6-in-1 primary` series. The Hib protection it confers is real - but that is the separate **coverage** question, not conformance.
