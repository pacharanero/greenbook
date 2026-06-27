# Open Questions

Questions to answer before the next implementation pass. The highest-priority group is Historical Versioning because it changes how the evaluator chooses schedule data.

## Historical Versioning

1. **What is the authoritative valid-time rule?**

   The current standard says `load_schedule_for_date(..., dob)` selects the schedule where `valid_from <= patient DOB`. Elsewhere, the conformance/coverage notes say a historical dose should be judged against the schedule valid at the time of administration. Those are not the same rule.

   Candidate interpretations:

   - Select one schedule version by DOB and evaluate the whole childhood record against that snapshot.
   - Select rules by when each dose became due, bounded by the evaluation date.
   - Select rules by administration date for recorded doses, but by due date for missing doses.
   - Keep one schedule version, but encode programme eligibility by birth cohort inside that version.

   This needs a ruling before `load_schedule_for_date` grows beyond a simple file picker.

> It needs to evaluate according to the Green Book schedule that applied at the time the does would have been due. So if the patient's life has spanned three Green Book versions, we would evaluate against the operative GB versions that would have been in effect at the time the dose was due, this means our TOML schedule needs a valid-from and valid-to date (or similar language) for each version. It is hoped that there would not be any gaps between the versions published, but given nobody has ever attempted translating vague PDF guidance into computable formats, I expect to find weird edge cases that hadn't been thought through.

2. **What question should the historical API answer by default?**

   Is the main query "is this person up to date today, using the schedule that applied to their birth cohort?" or "was this person up to date on a past evaluation date, using only schedule versions published by then?" The latter needs `evaluated_at` to constrain schedule selection so future knowledge is not used in a historical audit.

> in general it is the first question, "is this person up to date today, using the schedule that applied to their birth cohort?" - it's a question a parent or patient might plausibly ask. The more complex question of "was this person up to date at some point in the past" is a much less frequent question, really only imaginable as possibly a medicolegal inquiry, when trying to establish a person was insufficiently treated, perhaps. In this context - super rare.

3. **What is the first historical slice?**

   The best first implementation slice looks like the 2017 5-in-1 -> 6-in-1 transition because the current product map already includes Pediacel (`5-in-1`) and the mismatch is easy to demonstrate. Confirm whether that should be first, or whether an older/high-impact change such as MenC introduction, PCV introduction, HPV sex/birth-cohort changes, or MenB introduction should lead.

> We should just do all of them, if nothing else because it will undoubtedly turn up gaps in our TOML data model, things we hadn't yet considered.

4. **How far back is "enough" for the first public claim?**

   The roadmap currently says roughly 1990-present and ~8-12 schedule versions. Confirm the target horizon: 1990, start of childhood computerisation in GP systems, first Green Book predecessor schedule available in reliable source form, or another date.

> We go back initially to include all the green books, so if that's 1990-present then yes. The deep, pre-GB history is largely just showboating after that, not to say we won't do it, but all historical GB editions is still a generational leap in the tractability of this computation.

5. **Do product maps need versions?**

   A single UK SNOMED product map can contain current and historical products, and antigen composition is stable per product. But code systems, displays, and available product codes can change. Should product maps stay one append-only file per coding system for now, or should they become versioned alongside schedule files?

> I don't know the answer to this to be honest. But I have been building `sct` - a CLI that allows complete interation with SNOMED, CTV3, and ReadV2, which might help us explore this. It's going to be complex and we will find edge cases and unresolved historical coding weirdness.

6. **How much provenance is required in historical schedule files?**

   Current metadata gives one `source_document` and `source_url` per schedule file. Historical reconstruction may need per-series or per-dose citations, especially when a schedule combines changes from several publications. Decide whether file-level provenance is enough for v1 historical work.

For now, source document and URL are sufficient. If it gets gnarly in the deep history, we can probably regard a version of a document in the greenbook repo as the authoritative source, since pre-1990 sources are unlikely to have originally had URLs, and we'll be finding archived documents and scanning them.

7. **Should historical evaluation report the schedule-selection rationale?**

   For trust, the output may need a small `schedule_selection` block: candidate files considered, selected `valid_from`, selection date used, and reason. Confirm whether that belongs in every JSON result or only in verbose/debug output.

> in the verbose output only I think.

8. **What wording should replace "fully vaccinated" in historical contexts?**

   The code has a strict `fully_vaccinated` flag and a headline `UpToDateForAge`. For historical evaluation, users may ask "was this patient fully vaccinated at age X?" Confirm whether documentation should consistently translate that to "up to date for age on <date>" unless they explicitly request the strict all-series flag.

> For ultimate consistency let's plan ahead and go with "up to date for age on <date>" as the language. This allows us to show today's date by default, but also allows us to express a past date if the user wanted that (a very rare use case IMO)
