# Conformance vs coverage

This is the single most important idea in greenbook. There are **two different clinical questions**, and they need **two different matching rules**. Collapsing them into one - using antigen overlap to answer both - was the original bug.

<div class="grid" markdown>

!!! note "Conformance"
    *"Did the patient get the doses the Green Book named, at valid ages?"*

    - Matched by **product class**
    - Answered **per series**

!!! info "Coverage"
    *"What diseases is the patient protected against?"*

    - Computed across the **antigens** of every product received
    - Answered **per antigen**

</div>

## The 6-in-1 trap

A 6-in-1 dose contains Hib, tetanus, diphtheria and polio - antigens that *also* appear in later booster series (the Hib/MenC booster, the Td/IPV pre-school booster). So what happens when a child has had their 6-in-1 primary course?

=== "Match by antigen overlap (wrong)"

    The 6-in-1 dose shares antigens with the Hib/MenC and Td/IPV booster series, so it is **dragged into** them and flagged *"invalid - given too early"*.

    This is clinically nonsense: it is a primary dose, not a failed booster. The engine has invented a problem that does not exist.

=== "Match by product class (right)"

    A 6-in-1 dose is class `6-in-1`, so it conforms **only** to the `6-in-1` series. It is never matched against the Hib/MenC or Td/IPV booster series, because those are different product classes.

    The child's Hib protection is still real - but that is the separate **coverage** question, not conformance.

## Two questions, two answers

For a child who has completed the 6-in-1 primary course but not yet had the boosters:

- **Hib coverage**: satisfied ✓ (the antigen has been delivered)
- **Hib/MenC booster conformance**: not yet (no `Hib/MenC` class dose recorded)

Both answers are correct, and they are *different answers to different questions*. A tool that gives only one number cannot tell you this.

??? quote "From the project's ubiquitous-language dialogue"
    **Dev:** "A child got one Infanrix Hexa dose. That product covers Hib, so does it count toward the Hib/MenC booster *series*?"

    **Clinical informaticist:** "No. For **conformance** we match by **product class**, not **antigen**. Infanrix Hexa is class `6-in-1`, so it only counts toward the `6in1-primary` series. The Hib/MenC booster is class `Hib/MenC` - only a Menitorix dose conforms there."

    **Dev:** "But the child *is* protected against Hib after that dose?"

    **Clinical informaticist:** "Right - that's **coverage**, a separate question answered over **antigens**. Hib coverage is satisfied; Hib/MenC booster conformance is not. Two questions, two answers."

The full rationale, including why the reference engine computes conformance while the demo adds a coverage view, is recorded in [`spec/conformance-vs-coverage.md`](https://github.com/pacharanero/greenbook/blob/main/spec/conformance-vs-coverage.md).
