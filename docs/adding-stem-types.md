# How to add a new stem type to the declension engine

Practical guide for implementing agents. Read this instead of exploring the codebase.

## Overview

Adding a stem type requires changes to **4 data files** and sometimes **1-2 source files**. The 5-layer pipeline doesn't change — you're adding rule entries that the existing engine consumes.

| What | File | Format |
|---|---|---|
| Suffix selection (Layer 1) | `data/sup-suffix.json` | 24 entries per stem type |
| Pratyaya modification (Layer 2) | `data/pratyaya-rule.json` | varies per stem type |
| Aṅga modification (Layer 3) | `data/anga-rule.json` | varies per stem type |
| Junction sandhi (Layer 4) | `data/sandhi-rule.json` | only if new phoneme junctions arise |
| Tripādī (Layer 5) | `data/tripadi-rule.json` | rarely needs changes |
| GUI stem picker | `src/gui/paradigm.rs:31` | add to STEM_TYPES const |
| Analyzer probe stem | `src/engine/declension.rs` | only if stem_class slug doesn't encode the stem-final |

## Pipeline architecture

```
Input: (stem, stem_type, case, number)

Layer 1: stem_type + case + number → select sUP suffix
Layer 2: stem_type + pratyaya name → substitute suffix
Layer 3: stem-final + suffix-initial [+ vacana/vibhakti] → modify stem
Layer 4: stem-end phoneme + suffix-start phoneme → sandhi junction
Layer 5: word-final patterns → tripādī transformations

Output: final form + trace
```

Each layer applies at most one matching rule (first match wins after priority sort). Rules are sorted by: rule_type priority (apavāda > nitya > utsarga), then sūtra_position.

## File formats

### sup-suffix.json

One entry per (stem_class, vibhakti, vacana) triple. 24 per stem type.

```json
{
    "params": {
        "stem_class": "a-stem-n",
        "vibhakti": "prathama",
        "vacana": "ekavacana",
        "pratyaya": "su",
        "suffix": "s",
        "markers": ["u"],
        "sutra": "4.1.2",
        "sutra_position": "04.01.002"
    },
    "statement": "prathama ekavacana: su → s (Aṣṭ. 4.1.2)"
}
```

**These are identical across stem types** — same pratyaya names, same suffixes, same markers. Only `stem_class` changes. To add a new stem type: duplicate all 24 entries from an existing type, search-replace the stem_class.

Vibhakti values: prathama, dvitiya, tritiya, caturthi, pancami, sasthi, saptami, sambodhana.
Vacana values: ekavacana, dvivacana, bahuvacana.

### pratyaya-rule.json

Suffix substitutions conditioned on stem class and pratyaya name.

```json
{
    "params": {
        "condition_stem_class": "a-stem-n",
        "condition_suffix": "su",
        "condition_markers": [],
        "input_suffix": "s",
        "output_suffix": "am",
        "sutra": "7.1.24",
        "sutra_position": "07.01.024",
        "rule_type": "nitya",
        "condition_vibhakti": "prathama"
    },
    "statement": "su → am for neuter a-stem in prathama (Aṣṭ. 7.1.24)"
}
```

Fields:
- `condition_stem_class`: must match exactly
- `condition_suffix`: the pratyaya NAME from Layer 1 (e.g., "su", "bhis", "ṭā", "ṅe")
- `input_suffix`: what the suffix looks like NOW (after Layer 1 marker removal)
- `output_suffix`: what it becomes
- `condition_vibhakti`: optional, restricts to specific case (e.g., "sambodhana", "prathama")
- `rule_type`: "nitya" (always applies), "apavada" (exception, higher priority), "utsarga" (general)
- `sutra_position`: zero-padded for sort order (e.g., "07.01.024")

### anga-rule.json

Stem modifications conditioned on stem-final phoneme and suffix-initial phoneme.

```json
{
    "params": {
        "condition_stem_final": "a",
        "condition_markers": [],
        "condition_suffix_initial": "i",
        "condition_vacana": "bahuvacana",
        "condition_vibhakti": null,
        "operation": "substitute",
        "operation_target": "stem_final",
        "operation_input": "a",
        "operation_output": "ān",
        "sutra": "7.1.72",
        "sutra_position": "07.01.072",
        "rule_type": "nitya"
    },
    "statement": "a → ān (nUM + dīrgha) before śi in bahuvacana (Aṣṭ. 7.1.72)"
}
```

Fields:
- `condition_stem_final`: single phoneme, matched against `current_stem.chars().last()`
- `condition_suffix_initial`: matched against `first_phoneme(current_suffix)` — uses the phoneme tokenizer (handles digraphs like "bh", diphthongs like "ai")
- `condition_vacana`: optional filter
- `condition_vibhakti`: optional filter (added for aa-stem-f sambodhana rule)
- `operation_input`/`operation_output`: stem.ends_with(input) → replace with output

**Important**: anga rules do NOT have condition_stem_class. They fire based on phoneme conditions. If you need a rule to fire only for a specific stem type, use condition_vacana and/or condition_vibhakti as discriminators. If those aren't sufficient, you may need to add condition_stem_class to the AngaRule struct (small engine change).

### sandhi-rule.json

Junction rules: combine stem-end with suffix-start.

```json
{
    "params": {
        "first": "ā",
        "second": "ī",
        "result": "e",
        "sutra": "6.1.87",
        "sutra_position": "06.01.087",
        "rule_type": "utsarga",
        "condition_pratyaya": null
    },
    "statement": "ā + ī → e (guṇa-sandhi, Aṣṭ. 6.1.87)"
}
```

Fields:
- `first`/`second`: phoneme strings matched via the tokenizer
- `result`: the combined output (replaces both first and second)
- `condition_pratyaya`: optional, restricts to specific pratyaya name

**Only add entries for phoneme junctions that don't already exist.** These are shared across all stem types. Check the file before adding — it already has extensive savarṇa-dīrgha, guṇa, vṛddhi, yaṇ, and consonant assimilation rules.

### tripadi-rule.json

Word-final transformations. Rarely needs additions.

```json
{
    "params": {
        "context": "word_final",
        "condition_preceding": null,
        "condition_following": null,
        "input": "s",
        "output": "r",
        "position": "word_final",
        "sutra": "8.2.66",
        "sutra_position": "08.02.066",
        "rule_type": "nitya"
    },
    "statement": "s → ru (= r) word-finally (sasajuṣo ruḥ, Aṣṭ. 8.2.66)"
}
```

## Engine structs (Rust)

The JSON params fields map directly to these structs in `src/engine/declension.rs`:

```rust
struct SupSuffix {
    stem_class: String,
    vibhakti: String,
    vacana: String,
    pratyaya: String,
    suffix: String,
    markers: Vec<String>,
    sutra: String,
    sutra_position: String,
}

struct PratyayaRule {
    condition_stem_class: String,
    condition_suffix: String,
    condition_markers: Vec<String>,
    input_suffix: String,
    output_suffix: String,
    sutra: String,
    sutra_position: String,
    rule_type: String,
    condition_vibhakti: Option<String>,  // optional
}

struct AngaRule {
    condition_stem_final: String,
    condition_markers: Vec<String>,
    condition_suffix_initial: Option<String>,
    condition_vacana: Option<String>,
    condition_vibhakti: Option<String>,  // optional
    operation: String,
    operation_target: String,
    operation_input: String,
    operation_output: String,
    sutra: String,
    sutra_position: String,
    rule_type: String,
}

struct DeclensionSandhiRule {
    first: String,
    second: String,
    result: String,
    sutra: String,
    sutra_position: String,
    rule_type: String,
    condition_pratyaya: Option<String>,
}

struct TripadiRule {
    context: String,
    condition_preceding: Option<String>,
    condition_following: Option<String>,
    input: String,
    output: String,
    position: String,
    sutra: String,
    sutra_position: String,
    rule_type: String,
}
```

## Phoneme tokenizer

The engine uses a phoneme-aware tokenizer (`src/engine/phoneme.rs`) for matching at layer boundaries. Important behavior:

- Diphthongs `ai`, `au` are single phonemes (matched before `a`)
- Long vowels `ā`, `ī`, `ū`, `ṝ` are single phonemes
- Aspirate digraphs `kh`, `gh`, `ch`, `jh`, `ṭh`, `ḍh`, `th`, `dh`, `ph`, `bh` are single phonemes
- `first_phoneme("au")` returns `"au"`, NOT `"a"`
- `last_phoneme("vidyā")` returns `"ā"`, NOT `"a"` (chars().last(), not tokenizer, but same result for vowels)

## analyze_declension and probe stems

The analysis function (`analyze_declension`) reverse-engineers forms by trying all stem classes with a "probe stem" extracted from the stem_class name:

```rust
let probe_stem = stem_class.split('-').next().unwrap_or(stem_class);
```

For `a-stem-m` this gives `"a"` — a valid stem-final. For stem types where this heuristic fails (e.g., `aa-stem-f` gives `"aa"` instead of `"ā"`), add an explicit case:

```rust
let probe_stem = match stem_class.as_str() {
    "aa-stem-f" => "ā",
    // add new overrides here
    _ => stem_class.split('-').next().unwrap_or(stem_class),
};
```

## Naming conventions

Stem class slug format: `{stem-vowel}-stem-{gender}`

| Stem class | Slug | Probe stem |
|---|---|---|
| Masculine a-stem | `a-stem-m` | `a` |
| Neuter a-stem | `a-stem-n` | `a` |
| Feminine ā-stem | `aa-stem-f` | `ā` (needs override) |
| Masculine i-stem | `i-stem-m` | `i` |
| Feminine ī-stem | `ii-stem-f` | needs override |
| Masculine u-stem | `u-stem-m` | `u` |
| Feminine ū-stem | `uu-stem-f` | needs override |

No diacritics in slugs. Double vowel = long vowel.

## Test patterns

Unit tests in `src/engine/declension.rs` (tests module):

```rust
// Helper: derive a form using test fixtures
fn derive_neuter(case: &str, number: &str) -> DeriveResult {
    let sup = fixture_sup_suffixes();      // includes all stem types
    let pratyaya = fixture_pratyaya_rules();
    let anga = fixture_anga_rules();
    let sandhi = fixture_sandhi_rules();
    let tripadi = fixture_tripadi_rules();
    derive_declension(&sup, &pratyaya, &anga, &sandhi, &tripadi,
        DeclensionInput {
            stem: "phala".into(),
            stem_type: "a-stem-n".into(),
            case: case.into(),
            number: number.into(),
        },
    ).unwrap()
}

// Full paradigm test
#[test]
fn full_phala_paradigm() {
    let expected: Vec<(&str, &str, &str)> = vec![
        ("1", "sg", "phalam"),
        ("1", "du", "phale"),
        // ... all 24 cells
    ];
    for (case, number, exp) in expected {
        assert_eq!(/* derive and extract form */, exp);
    }
}
```

The fixture functions embed rule data as JSON literals. When adding a new stem type, extend each fixture function with the new rules.

## Checklist for adding a stem type

1. Look up the paradigm in `docs/sutra-reference.md` or traditional grammar
2. Identify which sūtras differ from existing stems
3. Add 24 sup-suffix entries to `data/sup-suffix.json`
4. Add pratyaya rules to `data/pratyaya-rule.json`
5. Add anga rules to `data/anga-rule.json` (if new stem-final behavior)
6. Check if new sandhi junctions are needed in `data/sandhi-rule.json`
7. Update `STEM_TYPES` in `src/gui/paradigm.rs`
8. Fix probe stem in `src/engine/declension.rs` if needed
9. Add fixture data and paradigm test in `src/engine/declension.rs`
10. Run `cargo test` and verify all 24 cells match expected forms
11. Start the GUI (`cargo run`) and generate a paradigm to visual-check

## Reference: existing rules per stem type

### a-stem-m (masculine, deva)
- Pratyaya: 8 rules (7.1.9, 7.1.12×3, 7.1.13, 7.1.54, 6.1.69, 6.1.103)
- Anga: 7 rules (7.3.101/102×4, 7.3.103×2, 7.3.104)

### a-stem-n (neuter, phala)
- Pratyaya: 12 rules (7 shared with masculine + 5 neuter-specific: 7.1.24, 7.1.19×2, 7.1.20×2)
- Anga: 1 new rule (7.1.72 nUM-āgama) + shares all masculine anga rules via stem-final "a"

### aa-stem-f (feminine ā-stem, vidyā)
- Pratyaya: 8 rules (6.1.68, 7.1.18×2, 7.3.113×3, 7.3.116, 7.1.54)
- Anga: 3 new rules (7.3.105×2, 7.3.106) — conditioned on stem-final "ā", no overlap with "a" rules
