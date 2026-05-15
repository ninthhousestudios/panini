# Pāṇini

A Sanskrit grammar engine and learning tool. Fetches grammar rules from vidya (knowledge store) and applies them to derive, analyze, and display word forms with full sūtra-cited traces.

## Language

**Derivation** (prakriyā):
The full traced process of producing a word form from its base — stem + suffix through all applicable transformations. A derivation contains one or more steps, each citing the sūtra that licenses it.
_Avoid_: transformation, generation

**Sūtra**:
Any entry in the Aṣṭādhyāyī — definitions (saṁjñā), meta-rules (paribhāṣā), operational rules (vidhi), extensions (atideśa), or scope markers (adhikāra).
_Avoid_: rule (when referring to non-operational sūtras)

**Rule** (vidhi):
An operational sūtra that the engine matches and applies — "replace X with Y when Z." A rule is a sūtra; not every sūtra is a rule.
_Avoid_: sūtra (when specifically meaning an engine-executable operation)

**Stem** (prātipadika):
The nominal base form before suffixes are attached. English term in code and API; Sanskrit term in sūtra citations.
_Avoid_: prātipadika (in code/API), base, root (root = dhātu, different concept)

**Suffix** (pratyaya):
The inflectional ending attached to a stem. English term in code and API; Sanskrit term in sūtra citations.
_Avoid_: pratyaya (in code/API), ending (ambiguous — could mean final sound)

**Case** (vibhakti):
The grammatical function (nominative, accusative, etc.). English term in code and API.
_Avoid_: vibhakti (in code/API)

**Number** (vacana):
Singular, dual, or plural. English term in code and API.
_Avoid_: vacana (in code/API)

**Analysis**:
The reverse of derivation — given a surface form, determine how it was produced. Sandhi analysis yields candidate word-pairs and the rule that joined them. Declension analysis yields stem, case, number, and stem type. One concept, operation-specific return shapes.
_Avoid_: splitting (too narrow — only covers sandhi), parsing (overloaded)

**Trace**:
An ordered list of steps recording how a derivation produced its result. Each step: input state, sūtra applied (number + text), transformation, output state. No-op layers are omitted — only steps that fired appear.
_Avoid_: proof, log, history

**Step**:
One sūtra application within a trace. The atomic unit of a derivation's explanation.
_Avoid_: stage, phase (phase = a future concept for sentence segmentation)

**Paradigm** (rūpāvalī):
The full inflection grid for a stem — 8 cases × 3 numbers = 24 forms, each with its trace. For MVP, a flat grid. Pattern highlighting and comparative paradigms are post-MVP.
_Avoid_: conjugation table (that's verbs), inflection table (too generic)

## Relationships

- A **derivation** produces one result and one **trace**
- A **trace** contains one or more **steps**, ordered
- Each **step** applies one **rule** (a **sūtra** of vidhi type)
- A **paradigm** contains 24 **derivations** (8 cases × 3 numbers)
- An **analysis** is the reverse of a **derivation** — it returns ranked candidates
- **Rules** are fetched from vidya as structured claims and cached in Pāṇini's engine at startup
- **Sūtras** that aren't rules (saṁjñā, paribhāṣā, adhikāra) live in vidya as entities or structural facts, not as engine-executable data

## Flagged ambiguities

- "rule" vs "sūtra" — resolved: **rule** = operational/vidhi sūtra the engine executes; **sūtra** = any Aṣṭādhyāyī entry
- "ending" — ambiguous between suffix (pratyaya) and final sound of a word. Use **suffix** for the former, avoid "ending"
- Phoneme data lives in both vidya (canonical) and Pāṇini engine (compiled helpers). Accepted duplication — vidya is source of truth for curation, engine has compiled form for performance

