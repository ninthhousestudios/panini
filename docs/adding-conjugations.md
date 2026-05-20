# How to add conjugation support to the conjugation engine

Practical guide for implementing agents. Read this instead of exploring the codebase.

## Overview

The conjugation engine derives verb forms through a 5-layer pipeline. Adding a new gaṇa (verb class) or lakāra (tense/mood) means adding **data entries** to JSON rule files and sometimes extending the engine with new **sub-passes**. The pipeline structure doesn't change.

| What | File | Format |
|---|---|---|
| Tiṅ selection (Layer 1) | `data/tin-suffix.json` | 9 entries per lakāra × pada |
| Vikaraṇa insertion (Layer 2) | `data/vikarana-rule.json` | 1 entry per gaṇa × lakāra type |
| Pre-vikaraṇa aṅga ops (Layer 3) | `data/verb-anga-rule.json` | guṇa, vṛddhi, semivowel rules |
| Pre-tiṅ operations (Layer 4) | `data/verb-anga-rule.json` | dīrgha, coalescence, guṇa_anga_final, yaṇ, śnā alternation, consonant_junction |
| Tripādī (Layer 5) | `data/tripadi-rule.json` | shared with declension |
| Engine source | `src/engine/conjugation.rs` | only if new sub-pass types needed |
| GUI conjugation picker | `src/gui/conjugation.rs` | add to gaṇa/lakāra dropdowns |
| Integration tests | `tests/integration.rs` | paradigm tests per gaṇa |

## Pipeline architecture

```
Input: (dhātu, gaṇa, lakāra, pada, puruṣa, vacana)

Layer 1: lakāra + puruṣa + vacana + pada → select tiṅ suffix
Layer 2: gaṇa + lakāra_type → select vikaraṇa, compute ṅit status
Layer 3: dhātu + vikaraṇa → guṇa/vṛddhi of dhātu vowel, semivowel substitution
           (gated by ṅit — blocked for gaṇas 4, 5, 6, 7, 8, 9)
         → form aṅga = dhātu + vikaraṇa (suffix mode)
                     or dhātu_prefix + vikaraṇa + dhātu_final (infix mode, gaṇa 7)
         → allopa: infix na → n before non-pit tiṅ (6.4.111)
Layer 4: aṅga + tiṅ → dīrgha, coalescence, guṇa of aṅga-final,
           yaṇ at junction, śnā alternation, ṇatva,
           consonant junction (8.4.55: d+t→tt, dh+t→ddh)
Layer 5: word-final → tripādī (s→r→ḥ, etc.)

Output: final form + trace
```

Each sub-pass applies first-match-wins. Rules are evaluated in declaration order within each sub-pass.

## Key mechanisms

### ṅit guṇa-blocking (1.2.4 + 1.1.5)

The engine computes two flags after vikaraṇa selection:

```
vikarana_is_nit = (lakara_type == "sārvadhātuka") && (it_markers does NOT contain "p")
vikarana_is_nit_marker = (it_markers contains "ṇ")
```

- `vikarana_is_nit` → Layer 3 guṇa/vṛddhi is **completely blocked**
- `vikarana_is_nit_marker` (gaṇa 10) → triggers guṇa/vṛddhi **branching** (see below)

| Vikaraṇa | it_markers | Sārvadh.? | Pit? | Ṅit? | Guṇa? |
|---|---|---|---|---|---|
| śap (1) | ś, p | yes | yes | no | ✅ |
| śyan (4) | ś, n | yes | no | yes | ❌ |
| śnu (5) | ś | yes | no | yes | ❌ |
| śa (6) | ś | yes | no | yes | ❌ |
| u (8) | (none) | yes | no | yes | ❌ |
| śnā (9) | ś | yes | no | yes | ❌ |
| ṇic (10) | ṇ, c | no¹ | n/a | no | ✅ (branched) |

¹ ṇic has no `lakara_type` in the JSON — it matches any lakāra since curādi always takes ṇic.

### Gaṇa 10 guṇa vs vṛddhi branching

When vikaraṇa has ṇ in it_markers (`vikarana_is_nit_marker`):

1. `upadha_is_vowel(dhātu)` → false? → **no change** (e.g., √cint → cint)
2. `is_upadha_laghu(dhātu)` → true? → **guṇa** via 7.3.86 (e.g., √cur → cor)
3. Otherwise → **vṛddhi** via 7.2.115 (dīrgha vowel upadha)

### Tiṅ ṅit gate (pre-tiṅ guṇa)

The aṅga-final guṇa sub-pass (u→o for gaṇas 5/8) only fires when the **tiṅ is pit**. Non-pit sārvadhātuka tiṅ is ṅit by 1.2.4, which blocks guṇa on the preceding aṅga.

Pit tiṅ suffixes: tip (ti), sip (si), mip (mi) — all ekavacana. Marked with `"is_pit": true` in tin-suffix.json.

### ṇatva (8.4.2)

After all pre-tiṅ operations (including śnā alternation), the engine checks: if the vikaraṇa started with 'n' AND the dhātu contains r/ṣ/ṛ/ṝ, then the vikaraṇa-derived 'n' becomes 'ṇ'. This handles √krī + nā → krīṇā.

**Triggers**: only r, ṣ, ṛ, ṝ. NOT i/ī/u/ū (those are NOT ṇatva triggers despite appearing in some descriptions).

**Ordering**: ṇatva runs AFTER śnā alternation. The alternation rules match 'nā' (pre-retroflexion), then ṇatva applies to the result.

## File formats

### tin-suffix.json

One entry per (lakāra, puruṣa, vacana, pada) combination. 9 entries for laṭ parasmaipada.

```json
{
    "params": {
        "lakara": "laṭ",
        "purusha": "prathama",
        "vacana": "ekavacana",
        "pada": "parasmaipada",
        "pratyaya_name": "tip",
        "suffix": "ti",
        "is_pit": true,
        "sutra": "3.4.78",
        "sutra_position": "03.04.078"
    },
    "statement": "prathama ekavacana: tip → ti (tiptasjhi... Aṣṭ. 3.4.78)"
}
```

Fields:
- `pratyaya_name`: the traditional name (tip, tas, jhi, sip, thas, tha, mip, vas, mas)
- `suffix`: the surface form after it-marker removal
- `is_pit`: true for ekavacana suffixes (tip, sip, mip) — used for śnā alternation and pre-tiṅ guṇa gating

### vikarana-rule.json

One entry per gaṇa (per lakāra type if needed).

```json
{
    "params": {
        "gana": "4",
        "vikarana_name": "śyan",
        "suffix": "ya",
        "it_markers": ["ś", "n"],
        "lakara_type": "sārvadhātuka",
        "sutra": "3.1.69",
        "sutra_position": "03.01.069"
    },
    "statement": "divādi (class 4): śyan vikaraṇa → ya (divādibhyaḥ śyan, Aṣṭ. 3.1.69)"
}
```

Fields:
- `gana`: string "1" through "10"
- `vikarana_name`: traditional name (śap, śyan, śnu, śa, u, śnā, ṇic)
- `suffix`: surface form after it-marker removal
- `it_markers`: array of it-marker phonemes — determines ṅit and ṇit status
- `lakara_type`: `"sārvadhātuka"` or `"ārdhadhātuka"` for filtering. Omit (null) if the vikaraṇa applies regardless of lakāra (e.g., ṇic for curādi)

### verb-anga-rule.json

All pre-vikaraṇa and pre-tiṅ rules in one file. Organized by `stage` + `rule_type`.

#### Pre-vikaraṇa rules (Layer 3)

**Guṇa** (`stage: "pre_vikarana"`, `rule_type: "guna"`):
```json
{
    "params": {
        "stage": "pre_vikarana",
        "rule_type": "guna",
        "condition_dhatu_final": "ū",
        "position": "dhatu_final",
        "input": "ū",
        "output": "o",
        "sutra": "7.3.84",
        "sutra_position": "07.03.084"
    },
    "statement": "ū → o (guṇa) before sārvadhātuka ..."
}
```

Positions: `"dhatu_final"` (matches `condition_dhatu_final`), `"dhatu_medial"` (matches `condition_dhatu_vowel` via `replace_medial_vowel()` — targets the first non-initial non-final vowel).

**Vṛddhi** (`rule_type: "vrddhi"`): same structure as guṇa but with vṛddhi outputs (i→ai, u→au, ṛ→ār, etc.). Only fires for gaṇa 10 ṇit-marker roots with vowel upadha.

**Semivowel** (`rule_type: "semivowel"`): fires after guṇa. Converts guṇa output to semivowel+vowel (o→av, e→ay).

#### Pre-tiṅ rules (Layer 4)

**Dīrgha** (`rule_type: "dirgha"`): a → ā before yaṅ-initial tiṅ (7.3.101). Uses `condition_suffix_initial_class: "yaṅ"`.

**Coalescence** (`rule_type: "coalescence"`): a + a → a at aṅga-tiṅ junction (6.1.97). Uses `operation_input`/`operation_output`.

**Guṇa of aṅga-final** (`rule_type: "guna_anga_final"`): u → o before consonant-initial pit tiṅ (7.3.84). For gaṇas 5/8.
```json
{
    "params": {
        "stage": "pre_tin",
        "rule_type": "guna_anga_final",
        "condition_anga_final": "u",
        "input": "u",
        "output": "o",
        "sutra": "7.3.84",
        "sutra_position": "07.03.084"
    }
}
```

**Yaṇ junction** (`rule_type: "yan_junction"`): u → v before vowel-initial tiṅ (6.1.77). For gaṇas 5/8 bahuvacana.

**Śnā alternation** (`rule_type: "sna_alternation"`): nā/nī/n for gaṇa 9.
```json
{
    "params": {
        "stage": "pre_tin",
        "rule_type": "sna_alternation",
        "condition_vikarana": "śnā",
        "condition_suffix_pit": false,
        "condition_suffix_initial_type": "consonant",
        "input": "nā",
        "output": "nī",
        "sutra": "3.1.81"
    }
}
```

Three entries cover the alternation:
- `condition_suffix_pit: true` → nā (before pit tiṅ)
- `condition_suffix_pit: false, condition_suffix_initial_type: "consonant"` → nī
- `condition_suffix_pit: false, condition_suffix_initial_type: "vowel"` → n

## Engine internals — what you need to know without reading the code

### Key variables through the pipeline

`derive_conjugation` maintains these variables across all layers:

| Variable | Type | Set when | Used by |
|---|---|---|---|
| `current_dhatu` | `String` | Layer 1 (from input), mutated by Layer 3 guṇa/semivowel | Layer 3, aṅga formation |
| `current_tin` | `String` | Layer 1 (tiṅ suffix), emptied by coalescence/junction | Layer 4 sub-passes, final combine |
| `vikarana` | `String` | Layer 2 (suffix field from rule) | Layer 3 traces, aṅga formation |
| `vikarana_name` | `String` | Layer 2 (vikarana_name field) | śnā/śnam alternation matching |
| `anga` | `String` | After Layer 3: `dhatu + vikarana` (suffix) or `prefix + vikarana + final_consonant` (infix) | Layer 4 sub-passes, final combine |
| `vikarana_byte_offset` | `usize` | Aṅga formation — byte offset where vikaraṇa starts in `anga` | ṇatva (to find the 'n') |
| `tin_is_pit` | `bool` | Layer 1 (from tiṅ rule `is_pit` field) | śnā/śnam alternation, guṇa gating |
| `vikarana_is_nit` | `bool` | Layer 2 (sārvadhātuka && !pit) | Layer 3 guṇa/vṛddhi gate |
| `is_infix` | `bool` | Layer 2 (insertion_mode == "infix") | Aṅga formation, trace formatting |

### Sub-pass code pattern

Every Layer 4 sub-pass follows the same structure. To add a new one, copy this pattern:

```rust
// Guard: skip if tiṅ already consumed by coalescence/junction
if !current_tin.is_empty() {
    for (params, rule) in parsed_anga
        .iter()
        .filter(|(p, _)| p.stage == "pre_tin" && p.rule_type == "YOUR_RULE_TYPE")
    {
        // Match conditions against anga/tin state
        if /* condition matches */ {
            let old_state = format!("{} + {}", anga, current_tin);
            // Apply transformation to anga and/or current_tin
            step_num += 1;
            trace.push(TraceStep {
                step: step_num,
                rule: rule.statement.clone(),
                rule_ref: sutra_ref(&params.sutra),
                input_state: old_state,
                output_state: format!("{} + {}", anga, current_tin),
            });
            break; // first-match-wins
        }
    }
}
```

**Junction sub-passes** (coalescence, consonant_junction) are special: when they fire, they merge the junction into `anga` and set `current_tin = String::new()`. This causes all subsequent sub-passes to skip (via the `!current_tin.is_empty()` guard). The final combine step then just uses `anga` as-is.

### Aṅga formation modes

After Layer 3 (pre-vikaraṇa operations), the aṅga is formed. Two modes:

**Suffix (default):** `anga = current_dhatu + vikarana`, `vikarana_byte_offset = current_dhatu.len()`

**Infix** (`insertion_mode: "infix"`): Split dhātu at final phoneme:
```
anga = dhatu_prefix + vikarana + dhatu_final_consonant
vikarana_byte_offset = dhatu_prefix.len()
```
Allopa (6.4.111) fires during infix formation: before non-pit tiṅ, the trailing 'a' of the vikaraṇa is dropped (na→n).

### Algorithmic vs rule-driven operations

Most operations are rule-driven (JSON data + generic sub-pass code). Two are algorithmic (hardcoded in the engine):

- **ṇatva** (8.4.2): checks `vikarana.starts_with('n')`, then scans `anga[..vikarana_byte_offset]` for triggers (r/ṣ/ṛ/ṝ). Replaces the first 'n' after the offset with 'ṇ'.
- **Upadha guṇa** (7.3.84): when vikaraṇa is ṅit and tiṅ is pit, applies guṇa to the penultimate phoneme of the aṅga.

Both run after all rule-driven sub-passes and before consonant junction.

## Engine structs (Rust)

In `src/engine/conjugation.rs`:

```rust
struct TinSuffix {
    lakara: String,
    purusha: String,
    vacana: String,
    pada: String,
    pratyaya_name: String,
    suffix: String,
    is_pit: bool,          // true for ekavacana (tip, sip, mip)
    sutra: String,
}

struct VikaranaRule {
    gana: String,
    vikarana_name: String,
    suffix: String,
    lakara_type: Option<String>,
    it_markers: Vec<String>,    // determines ṅit and ṇit status
    sutra: String,
    insertion_mode: Option<String>, // None/"suffix" = append; "infix" = before final consonant
}

struct VerbAngaRule {
    stage: String,              // "pre_vikarana" or "pre_tin"
    rule_type: String,          // "guna", "vrddhi", "semivowel", "dirgha",
                                // "coalescence", "guna_anga_final",
                                // "yan_junction", "sna_alternation",
                                // "consonant_junction"
    condition_dhatu_final: Option<String>,
    condition_dhatu_vowel: Option<String>,
    position: Option<String>,
    input: Option<String>,
    output: Option<String>,
    condition_suffix_initial_class: Option<String>,
    operation_input: Option<String>,
    operation_output: Option<String>,
    condition_anga_final: Option<String>,
    condition_vikarana: Option<String>,
    condition_suffix_pit: Option<bool>,
    condition_suffix_initial_type: Option<String>,
    sutra: String,
}
```

## Phoneme tokenizer

Same tokenizer as declension (`src/engine/phoneme.rs`). Key functions used by conjugation:

- `tokenize(s)` → `Vec<&str>` — phoneme-aware split
- `first_phoneme(s)` → `Option<&str>` — used for tiṅ-initial consonant/vowel checks
- `last_phoneme(s)` → `Option<&str>` — used for aṅga-final in junction matching
- `is_vowel(phoneme)` → bool — checks against `VOWEL_PHONEMES`
- `replace_medial_vowel(dhatu, from, to)` → replaces first non-initial non-final vowel

## Currently supported

| Gaṇa | Vikaraṇa | Suffix | Mechanism |
|---|---|---|---|
| 1 (bhvādi) | śap | a | guṇa + semivowel + dīrgha + coalescence |
| 2 (adādi) | luk | (empty) | śap deleted (2.4.72); consonant junction sandhi (8.4.55) |
| 3 (juhotyādi) | ślu | (empty) | śap deleted (2.4.75); reduplication (6.1.1); abhyāsa: hrasva (7.4.59), halādiḥ śeṣaḥ (7.4.60), kuhoś cuḥ (7.4.62); jhi→ati (7.1.4) |
| 4 (divādi) | śyan | ya | ṅit blocks guṇa |
| 5 (svādi) | śnu | nu | ṅit blocks pre-vik guṇa; pre-tiṅ guṇa u→o (pit only), yaṇ u→v |
| 6 (tudādi) | śa | a | ṅit blocks guṇa |
| 7 (rudhādi) | śnam | na (infixed) | infix before final consonant; allopa na→n (6.4.111); ṇatva; consonant junction |
| 8 (tanādi) | u | u | same as gaṇa 5 |
| 9 (kryādi) | śnā | nā | ṅit blocks guṇa; śnā alternation nā/nī/n; ṇatva |
| 10 (curādi) | ṇic | aya | guṇa/vṛddhi branching by upadha |

Lakāra: laṭ only. Pada: parasmaipada only.

## Not yet supported (deferred)

Other lakāras (loṭ, laṅ, liṭ, etc.) need separate tiṅ suffix entries and may need lakāra-specific aṅga rules.

Ātmanepada needs a parallel set of tiṅ suffixes with different surface forms.

## Test patterns

### Unit tests (`src/engine/conjugation.rs` tests module)

```rust
fn fixture_tin() -> Vec<CachedRule>      // tip → ti (prathama eka)
fn fixture_vikarana() -> Vec<CachedRule>  // śap (gaṇa 1)
fn fixture_verb_anga() -> Vec<CachedRule> // guṇa ū→o, semivowel o→av
fn fixture_tripadi() -> Vec<CachedRule>   // s→r, r→ḥ

fn derive(tin, vik, anga, tri, dhatu) -> DeriveResult  // always gaṇa 1
```

To test other gaṇas in unit tests, build custom vikaraṇa fixtures with the appropriate `it_markers` and `lakara_type`.

### Integration tests (`tests/integration.rs`)

```rust
fn derive_conj(cache, dhatu, purusha, vacana) -> String       // gaṇa 1
fn derive_conj_gana(cache, dhatu, gana, purusha, vacana) -> String  // any gaṇa
```

Standard pattern: test prathama × all three vacanas, plus edge cases.

```rust
#[tokio::test]
async fn conjugation_gana5_su() {
    let cache = build_cache();
    assert_eq!(derive_conj_gana(&cache, "su", "5", "prathama", "ekavacana"), "sunoti");
    assert_eq!(derive_conj_gana(&cache, "su", "5", "prathama", "dvivacana"), "sunutaḥ");
    assert_eq!(derive_conj_gana(&cache, "su", "5", "prathama", "bahuvacana"), "sunvanti");
}
```

## Checklist: adding a new gaṇa

1. Look up the vikaraṇa name, it-markers, and suffix in grammar references
2. Determine if the vikaraṇa is sārvadhātuka or ārdhadhātuka
3. Add entry to `data/vikarana-rule.json`
4. Check if ṅit status blocks guṇa (sārvadhātuka apit → ṅit → blocked)
5. Add any new verb-anga-rule entries (new rule_types may need engine sub-passes)
6. Add integration tests with representative dhātus, testing all three vacanas
7. Run `cargo test` — all existing tests must still pass
8. Update the "Currently supported" table in this doc

## Checklist: adding a new lakāra

1. Add 9 tiṅ suffix entries to `data/tin-suffix.json` (one per puruṣa × vacana)
2. Set `is_pit` correctly on ekavacana entries
3. Check if new vikaraṇa entries are needed (some lakāras use different vikaraṇas)
4. Add any lakāra-specific verb-anga-rules (e.g., augment for laṅ)
5. Add integration tests
6. The engine's `lakara_to_type()` function maps lakāras to sārvadhātuka/ārdhadhātuka — verify it handles the new lakāra

## Checklist: adding ātmanepada

1. Add 9 tiṅ suffix entries per lakāra with `"pada": "ātmanepada"`
2. Ātmanepada suffixes have different surface forms (ta, ātām, anta/jha, thās, āthām, dhvam, e, āvahe, āmahe for laṭ)
3. Set `is_pit` on the appropriate suffixes
4. The engine already filters by pada — no engine changes needed for basic support
5. Some ātmanepada-specific aṅga rules may be needed (e.g., different tiṅ substitutions)

## Gotchas discovered during implementation

1. **ṇatva trigger list**: only r/ṣ/ṛ/ṝ — NOT i/ī/u/ū despite some descriptions including them
2. **ṇatva ordering**: must run AFTER śnā alternation, not before. Alternation matches "nā", then ṇatva converts the surviving "n" to "ṇ"
3. **Gaṇa 10 lakāra_type**: ṇic must have no `lakara_type` (null) so it matches any lakāra — curādi verbs always take ṇic
4. **Consonant upadha**: for ṇit-marker vikaraṇas (gaṇa 10), when upadha is a consonant (not a vowel), neither guṇa nor vṛddhi applies (√cint → cint, not caint or cent)
5. **Pre-tiṅ guṇa gating**: aṅga-final guṇa (u→o for gaṇas 5/8) only fires when the **tiṅ itself** is pit. Non-pit sārvadhātuka tiṅ is ṅit by 1.2.4, blocking guṇa
6. **Coalescence empties tiṅ**: when coalescence fires (a+a→a), it combines aṅga and tiṅ into one string and sets `current_tin` to empty. Subsequent sub-passes check `!current_tin.is_empty()` and skip
7. **Empty vikaraṇa works without engine changes**: gaṇa 2 (luk) uses `suffix: ""`. The concatenation `dhatu + ""` just gives the dhātu. Preserve śap's it-markers `["ś", "p"]` per sthānivat (1.1.56) so guṇa isn't incorrectly blocked
8. **Consonant junction also empties tiṅ**: same pattern as coalescence — when a junction rule fires (dt→tt, dht→ddh), the merged result goes into `anga` and `current_tin` is emptied
9. **ṇatva uses vikarana_byte_offset, not dhatu length**: for infix mode, the vikaraṇa starts at `prefix.len()`, not `current_dhatu.len()`. The offset variable tracks this for both modes
10. **Infix allopa is pit-gated, not vowel-gated**: the doc originally said na→n "before vowel-initial suffix" but it actually fires for ALL non-pit tiṅ (6.4.111). Consonant-initial non-pit tiṅ also gets the reduced form (dvivacana "taḥ" is non-pit → na→n)
11. **Layer 3 guṇa skipped for empty vikaraṇa**: when vikaraṇa is "" (gaṇas 2/3), pre-vikaraṇa guṇa is entirely skipped. Guṇa fires through Layer 4 guna_anga_final instead (pit-gated). This prevents medial guṇa from incorrectly targeting the abhyāsa vowel in reduplicated forms (e.g., "juhu" would become "johu" if medial guṇa fired)
12. **Aspiration displacement** (dh+t→ddh): this is phonologically complex (8.4.55 + aspiration transfer) but implementable as a single junction rule. Don't try to model the intermediate steps
