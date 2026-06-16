# What the Green Book is

The **Green Book** - [*Immunisation against infectious disease*](https://www.gov.uk/government/publications/immunisation-schedule-the-green-book-chapter-11) - is the UK's definitive guidance on vaccines and vaccination. Chapter 11 carries the **routine childhood immunisation schedule**: which vaccines a child should receive, and at what ages.

It is published by UKHSA as **human-readable PDFs** on GOV.UK. For a clinician, that is exactly right. For a computer, it is a problem.

## The "digital paper" problem

As of 2026 there is **no computable version** of the schedule. The PDF is the source of truth. So every digital system that needs the schedule - GP systems, child health information services, the NHS App - re-implements it by hand from the PDF:

```
Word document  →  PDF on GOV.UK  →  Hand-written code in each vendor's system
```

Each vendor reverse-engineers the rules into code. That derivative is lossy and error-prone, and - because there is no shared machine-readable original - the implementations **diverge, drift, and break** independently. We call this *digital paper*: a document that looks structured but carries no structure a machine can trust.

## "Is this child fully vaccinated?"

The question the schedule exists to answer turns out to be deceptively hard. To answer it you have to know three separate things:

1. **Which schedule applied** when the child was born - and what was in scope by the time each dose was due.
2. **What they actually received** - the recorded doses, often from several systems.
3. **Whether what they received satisfies** the schedule that applied to them.

And "fully vaccinated" does not even mean one thing - see [the status model](../concepts/status-model.md). A perfectly on-track 6-month-old has not had MMR yet (it is not due until 12 months); calling them "not fully vaccinated" is technically true but clinically misleading. The honest headline is **up to date for age**.

[:octicons-arrow-right-24: Why a computable version is necessary](why-computable.md)
