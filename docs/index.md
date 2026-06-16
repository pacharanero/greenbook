# greenbook

**Turning the Green Book from a PDF into a computable, versioned source of truth - and an engine that can actually read it.**

The UK childhood immunisation schedule - [Green Book](https://www.gov.uk/government/publications/immunisation-schedule-the-green-book-chapter-11) Chapter 11 - is published as human-readable PDFs. As of 2026 there is **no computable version**: every digital system that needs the schedule re-implements it by hand from the PDF, a lossy and error-prone derivative.

greenbook is a proof-of-concept for the other way round: a **computable, versioned schedule** as the upstream source of truth, and an **evaluation engine** that takes a patient's FHIR vaccination history and answers - honestly - whether they are up to date for their age.

[:octicons-arrow-right-24: What the Green Book is](green-book/what-it-is.md) ·
[:octicons-arrow-right-24: Why a computable version](green-book/why-computable.md) ·
[:octicons-arrow-right-24: Getting started](getting-started.md)

## See it run

Pick an example patient and watch the engine evaluate them - the headline verdict, conformance by series, and antigen coverage. This is a pared-back, read-only slice of the full [interactive demo](demo/index.html).

<div class="gb-mini" id="gb-mini-demo" data-demo-base="demo/">
  <div class="gb-mini__bar" role="group" aria-label="Example patients"></div>
  <div class="gb-mini__body">
    <div class="gb-mini__head"></div>
    <div class="gb-mini__series" aria-label="Conformance by series"></div>
  </div>
  <p class="gb-mini__foot"><a href="demo/index.html">Open the full interactive demo - build your own patient, switch to the timeline view &rarr;</a></p>
</div>

<div class="grid cards" markdown>

-   :material-book-open-variant:{ .lg .middle } __The Green Book today__

    ---

    What the schedule is, the "digital paper" problem, and why answering
    *"is this child fully vaccinated?"* is genuinely hard.

    [:octicons-arrow-right-24: Read on](green-book/what-it-is.md)

-   :material-sitemap:{ .lg .middle } __The concepts__

    ---

    The domain model - Schedule → Series → Dose, product class vs antigen -
    and the two ideas that make the engine correct: **conformance vs coverage**
    and the **status model**.

    [:octicons-arrow-right-24: The domain model](concepts/domain-model.md)

-   :material-console:{ .lg .middle } __Getting started__

    ---

    Install and run the engine. A reference implementation in **Rust** and a
    peer implementation in **JavaScript**, validated against one shared
    conformance suite.

    [:octicons-arrow-right-24: Run it](getting-started.md)

-   :material-flask:{ .lg .middle } __Try it live__

    ---

    An [interactive demo](demo/index.html) that evaluates example patients - or
    build your own and watch the result update live - and the
    [presentation](presentation/presentation.html) behind the project.

    [:octicons-arrow-right-24: Walkthrough](walkthrough.md)

</div>

## The long-term goal

What if the **computable** schedule were the source of truth, and the PDF, the website, and clinical tools were all **generated from it**? Experts would author schedule changes directly in a computable form, and every downstream publication - PDFs, websites, clinical decision support - would be rendered from the same trusted data. That replaces today's "digital paper" workflow, where each vendor reverse-engineers the rules from a PDF and quietly diverges.

This prototype builds that correctly for the *current* schedule first, so historical versioning becomes an additive extension rather than a rewrite.
