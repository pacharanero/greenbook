# MMR catch-up — test scenarios

Six patient cases used to test schedule-aware classification of MMR
vaccination records. Each case lists the inputs (DOB and full vaccination
history) and the expected classifier output (which doses are valid, which
are out of schedule and why, the resulting programme status, and the
eligible-from date for the next slot).

The scenarios are seeded into the prototype at
`lib/create-data.js` (search for `seededMmrPatients`) and surfaced through
the catch-up session `mmr-catchup-y9y10-current`.

## Schedule rules (UK MMR)

- **Dose 1** valid from **age 12 months**
- **Dose 2** valid from **age 15 months** *and* **at least 28 days after
  Dose 1**
- Maximum schedule length: **2 doses**. Any additional given dose is
  classified as `Additional dose`
- Doses are classified in chronological order (`createdAt` ascending)
- A non-given record (Absent, Refused, Unwell, Contraindicated) is not
  counted toward the schedule and does not affect classification of later
  given records

### Out-of-schedule reasons (enum)

| Reason | Trigger |
| --- | --- |
| `BeforeAge12Months` | Given dose where `createdAt < dob + 12 months` and Dose 1 slot is open |
| `BeforeAge15Months` | Given dose where `createdAt < dob + 15 months` and Dose 2 slot is open |
| `LessThan28DaysAfterPrevious` | Given dose where `createdAt - lastValidGiven.createdAt < 28 days` |
| `ExtraDose` | Given dose arriving when both slots are already filled |

### Eligible-from for next slot

```
nextSlot = validDoses.length + 1
if nextSlot > 2:           # schedule complete
  eligibleFrom = null
else:
  minAge = dob + (12 months if nextSlot == 1 else 15 months)
  if lastValidGiven exists:
    eligibleFrom = max(minAge, lastValidGiven.createdAt + 28 days)
  else:
    eligibleFrom = minAge
```

## Vaccine codes used

| SNOMED | Brand | Type |
| --- | --- | --- |
| `13968211000001108` | M-M-RvaxPro | MMR |
| `34925111000001104` | Priorix | MMR |
| `99926011000001103` | ProQuad | MMRV (off-schedule UK; included to model miscoded records) |
| `39779611000001104` | MenQuadfi | MenACWY |
| `7374311000001101` | Revaxis | Td/IPV |

---

## Case 1 — Alice Adams

Two valid doses. Dose 2 was recorded with an MMRV product (ProQuad), a
common data-entry error since MMRV is not on the UK schedule. The
classifier still counts it because the programme is MMR.

- **NHS number:** 9990000011
- **DOB:** 2012-01-12 (Y9, AY 2025/26)

### Vaccination history

| # | Date | Age | Vaccine | Source | Outcome |
| --- | --- | --- | --- | --- | --- |
| 1 | 2013-01-26 | 12m 14d | M-M-RvaxPro (MMR) | NHS Immunisations API | Vaccinated |
| 2 | 2015-05-22 | 40m 10d | ProQuad (MMRV) | NHS Immunisations API | Vaccinated |

### Expected classification

| # | Status | Slot | Reason |
| --- | --- | --- | --- |
| 1 | Valid | Dose 1 | – |
| 2 | Valid | Dose 2 | – |

- **Programme status:** Vaccinated
- **Eligible from:** `null` (schedule complete)

---

## Case 2 — Bilal Begum

One valid Dose 1, then a long trail of failed attempts to give Dose 2.
Bilal still needs a second MMR dose. Also has a separate Y9 Doubles
absence record from last academic year.

- **NHS number:** 9990000029
- **DOB:** 2011-03-03 (Y10, AY 2025/26)

### Vaccination history (MMR)

| # | Date | Age | Vaccine | Source | Outcome | Note |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 2012-03-17 | 12m 14d | M-M-RvaxPro | NHS Immunisations API | Vaccinated | – |
| 2 | 2015-05-03 | 50m 0d | M-M-RvaxPro | Recorded in Mavis | Absent | – |
| 3 | 2015-12-03 | 57m 0d | M-M-RvaxPro | Recorded in Mavis | Absent | – |
| 4 | 2016-04-03 | 61m 0d | M-M-RvaxPro | Recorded in Mavis | Refused | Parents declined this dose. Bilal has been under paediatric oncology care at Birmingham Children's Hospital since late 2014; MMR was deferred at earlier sessions on his consultant's advice while his immune system was compromised, and declined by his parents during one mid-treatment session. Discharge summary (June 2016, Dr Patel) confirms he is fit to resume routine immunisations once parents are ready. |

### Expected classification

| # | Status | Slot | Reason |
| --- | --- | --- | --- |
| 1 | Valid | Dose 1 | – |
| 2 | Not counted | – | Outcome `Absent`, no dose given |
| 3 | Not counted | – | Outcome `Absent`, no dose given |
| 4 | Not counted | – | Outcome `Refused`, no dose given |

- **Programme status:** Partially vaccinated
- **Eligible from:** 2012-06-03 (DOB + 15 months; far in the past, so eligible immediately)
- **Doses remaining:** 1

### Doubles records (separate programmes)

| Date | Programme | Vaccine | Outcome | Note |
| --- | --- | --- | --- | --- |
| 2025-06-12 | MenACWY | MenQuadfi | Absent | Off school with viral illness on the day of the team visit. |
| 2025-06-12 | Td/IPV | Revaxis | Absent | Off school with viral illness on the day of the team visit. |

---

## Case 3 — Chiamaka Chen

A dose was given at 11 months, before the 12-month minimum. The
classifier marks it out of schedule and the next given dose (at 50
months) becomes Dose 1.

- **NHS number:** 9990000037
- **DOB:** 2011-11-28 (Y9, AY 2025/26)

### Vaccination history

| # | Date | Age | Vaccine | Source | Outcome |
| --- | --- | --- | --- | --- | --- |
| 1 | 2012-10-28 | 11m 0d | M-M-RvaxPro | NHS Immunisations API | Vaccinated |
| 2 | 2016-01-28 | 50m 0d | M-M-RvaxPro | Recorded in Mavis | Vaccinated |

### Expected classification

| # | Status | Slot | Reason |
| --- | --- | --- | --- |
| 1 | Out of schedule | – | `BeforeAge12Months` |
| 2 | Valid | Dose 1 | – |

- **Programme status:** Partially vaccinated
- **Eligible from:** 2016-02-25 (DOB + 15 months *or* dose 1 + 28 days; the latter wins)
- **Doses remaining:** 1

---

## Case 4 — Dmitri Dixit

A dose was given one day before the first birthday (11m 29d). The
classifier marks it out of schedule. The next given dose at 48m was
recorded as Dose 2 by the original clinician, but the classifier
promotes it to Dose 1 because Dose 1 is unfilled — so the patient
appears to still need a Dose 2.

The parent on the catch-up session has refused MMR with reason
"Vaccine already received".

- **NHS number:** 9990000045
- **DOB:** 2012-07-05 (Y9, AY 2025/26)

### Vaccination history

| # | Date | Age | Vaccine | Source | Outcome |
| --- | --- | --- | --- | --- | --- |
| 1 | 2013-07-04 | 11m 29d | M-M-RvaxPro | NHS Immunisations API | Vaccinated |
| 2 | 2016-07-19 | 48m 14d | M-M-RvaxPro | NHS Immunisations API | Vaccinated |

### Expected classification

| # | Status | Slot | Reason |
| --- | --- | --- | --- |
| 1 | Out of schedule | – | `BeforeAge12Months` |
| 2 | Valid | Dose 1 | – |

- **Programme status:** Partially vaccinated
- **Eligible from:** 2016-08-16 (dose 1 + 28 days; later than DOB + 15 months)
- **Doses remaining:** 1

### Consent

| Date | Decision | Refusal reason |
| --- | --- | --- |
| 2026-04-27 | Refused | Vaccine already received |

---

## Case 5 — Eshe Edwards

One clinical Dose 1 event recorded three times — the original GP record
plus two echo duplicates from other feeds, arriving days apart. The
classifier counts the earliest as Dose 1 and marks the duplicates as
out of schedule because they fall within the 28-day interval rule.

- **NHS number:** 9990000053
- **DOB:** 2010-10-15 (Y10, AY 2025/26)

### Vaccination history

| # | Date | Age | Vaccine | Source | Outcome |
| --- | --- | --- | --- | --- | --- |
| 1 | 2011-11-15 | 13m 0d | M-M-RvaxPro | NHS Immunisations API (GP) | Vaccinated |
| 2 | 2011-11-18 | 13m 3d | M-M-RvaxPro | NHS Immunisations API (echo feed) | Vaccinated |
| 3 | 2011-11-19 | 13m 4d | M-M-RvaxPro | NHS Immunisations API (second echo) | Vaccinated |

### Expected classification

| # | Status | Slot | Reason |
| --- | --- | --- | --- |
| 1 | Valid | Dose 1 | – |
| 2 | Out of schedule | – | `LessThan28DaysAfterPrevious` |
| 3 | Out of schedule | – | `LessThan28DaysAfterPrevious` |

- **Programme status:** Partially vaccinated
- **Eligible from:** 2012-01-15 (DOB + 15 months; later than dose 1 + 28 days)
- **Doses remaining:** 1

---

## Case 6 — Farah Farooq

A single dose recorded with a `2P` sequence label by the source feed
(i.e. the originating system labelled it as the second/pre-school
dose), but no Dose 1 record arrived. The classifier slots it as Dose 1
per the schedule, so the source label and the schedule position
disagree.

- **NHS number:** 9990000061
- **DOB:** 2012-03-15 (Y9, AY 2025/26)

### Vaccination history

| # | Date | Age | Vaccine | Source | Outcome | Sequence label |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 2016-03-15 | 48m 0d | M-M-RvaxPro | NHS Immunisations API | Vaccinated | `2P` |

### Expected classification

| # | Status | Slot | Reason |
| --- | --- | --- | --- |
| 1 | Valid | Dose 1 | – |

- **Programme status:** Partially vaccinated
- **Eligible from:** 2016-04-12 (dose 1 + 28 days; later than DOB + 15 months)
- **Doses remaining:** 1

---

## Cohort session

All six patients are tied to a single co-located session for the
catch-up scenario:

- **Session id:** `mmr-catchup-y9y10-current`
- **School:** Grace Academy Coventry (id `135335`)
- **Date:** 17 June 2026 (summer term)
- **Year groups:** Y9 and Y10
- **Programmes:** MMR catch-up + Doubles (MenACWY + Td/IPV) co-located
- **Consents seeded:** positive consent for every patient × programme,
  except Dmitri's MMR (Refused, "Vaccine already received") and Bilal's
  Y9 Doubles (already absent at last year's session, no current consent)
