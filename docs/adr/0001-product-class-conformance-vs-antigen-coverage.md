# 1. Product-class conformance vs antigen coverage

Date: 2026-06-02

## Status

Accepted.

## Context

Determining a patient's vaccination status against the Green Book turns out to be two different questions, not one:

1. **Schedule conformance** — "did this child receive the doses the Green Book asked for, at valid ages and intervals?" The Green Book asks for *named products* at each appointment ("the 6-in-1 at 8, 12 and 16 weeks").
2. **Antigen / disease coverage** — "what is this child actually protected against?" This is a property of the antigens delivered, aggregated across every product received, regardless of which programme each came from.

The original POC matched a dose to a series by **antigen overlap**: a dose matched a series if the product covered at least one of the series' antigens. This conflated the two questions and used the *coverage* rule to answer the *conformance* question. Because antigens recur across products, it produced clinically wrong results: a single 6-in-1 dose (which contains Hib, tetanus, diphtheria and polio) matched not only `6in1-primary` but also the `hib-menc-booster` and `tdap-ipv-booster` series, where it was then flagged INVALID for being "given before earliest_age". See the original write-up in [queries.md](../../queries.md) §1.

The spec ([spec/standard.md](../../spec/standard.md)) separately relies on antigen-level reasoning to handle products changing over time — a 5-in-1 Pediacel dose covers diphtheria/tetanus/pertussis/polio/Hib but not hepatitis B, even against a schedule that now expects 6-in-1. That rationale is sound, but it is a *coverage* concern, not a *conformance* one.

## Decision

Treat conformance and coverage as two separate computations with two different matching rules.

**Conformance matching is by product class.** Every product in the product map declares a `product_class` (e.g. `"6-in-1"`, `"MMR"`, `"Td/IPV"`) — the unit the Green Book names. Every schedule series declares the `product_class` whose doses conform to it. A dose belongs to a series if and only if the product's class equals the series' class. A 6-in-1 dose therefore matches only `6in1-primary` and is never dragged into a booster series.

**Coverage remains antigen-based** and is computed separately by aggregating the `antigens` of every product received. The antigen registry and the per-product `antigens` list are retained for this purpose (and for referential-integrity validation). The antigen-coverage output (`by_antigen`) is **deferred** to a later milestone; this ADR establishes the model so it lands additively.

Each series references a single product class for v1. Multi-class acceptance (e.g. a transitional schedule where both 5-in-1 and 6-in-1 satisfy the primary course) is a foreseeable extension and would be additive — a list, or an `accepts` set — and is explicitly out of scope now.

Cross-time substitution (the 5-in-1 → 6-in-1 case) is resolved by the two views giving two correct answers: under conformance, a dose is judged against the schedule valid *at the time of administration* (once historical versioning exists), so a 2015 Pediacel dose conforms to the 2015 schedule; under coverage, that same child shows no hepatitis B unless it was given elsewhere.

## Consequences

- The spurious INVALID booster entries disappear; queries §1 and §3 are resolved without per-report suppression hacks.
- The product map and every schedule series gain a required `product_class` field. This is a format change; all bundled data is updated in the same change.
- A *known* product whose class matches no series in the loaded schedule (e.g. Pediacel against the 2026 schedule) now conforms to nothing and is not counted. Surfacing such "known product, no conforming series in this schedule version" doses — alongside genuinely unknown product codes — is follow-up work tracked on the [roadmap](../../spec/roadmap.md) (M2).
- Conformance now depends on the product map being complete and correctly classed. Brand variation (e.g. Infanrix Hexa and Vaxelis both being "6-in-1") is handled by assigning them the same class rather than by enumerating codes per series.
