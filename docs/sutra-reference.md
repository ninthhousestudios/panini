# Sūtra reference for the declension engine

Pre-extracted mapping from Aṣṭādhyāyī sūtras to their role in the 5-layer declension pipeline. Sourced from the SCL simulator (`~/soft/scl/ashtadhyayi_simulator/june12/`) and cross-referenced with traditional grammar.

Use this instead of re-searching the SCL codebase. When adding new stem types, look up the relevant sūtras here first.

## SCL simulator conventions

The simulator's rule constructor:
```
rule(sutra_number, category, stem_regex, suffix_regex, stem_attrib, suffix_attrib)
```

Categories map to our pipeline layers:
| SCL category | Pipeline layer |
|---|---|
| (suffix table) | Layer 1: suffix selection |
| `prawyaya_viXi` | Layer 2: pratyaya modification |
| `afga_viXi` | Layer 3: aṅga modification |
| `sanXi` / `ekAxeSa` | Layer 4: junction sandhi |
| `wripAxI` | Layer 5: tripādī |

## Layer 1: suffix selection (4.1.2)

**4.1.2** svaujasamauṭchaṣṭābhyāmbhisṅebhyāmbhyasṅasibhyāmbhyasṅasosāmṅyossup — the universal sUP pratyaya list. Same for all stems, all genders. Gender/stem differences are handled in layers 2-3.

The 24 suffixes (7 cases × 3 numbers + sambodhana × 3):

| Vibhakti | Eka | Dvi | Bahu |
|---|---|---|---|
| Prathamā | su (→s) | au | jas (→as) |
| Dvitīyā | am | auṭ (→au) | śas (→as) |
| Tṛtīyā | ṭā (→ā) | bhyām | bhis |
| Caturthī | ṅe (→e) | bhyām | bhyas |
| Pañcamī | ṅasi (→as) | bhyām | bhyas |
| Ṣaṣṭhī | ṅas (→as) | os | ām |
| Saptamī | ṅi (→i) | os | sup (→su) |
| Sambodhana | su (→s) | au | jas (→as) |

Parenthesized forms show the suffix after it-marker removal (ṅ, ṭ, j, ś, p are markers).

## Layer 2: pratyaya modifications

### Shared across a-stems (masculine and neuter)

**7.1.9** — ato bhisa ais: bhis → ais after a-stem.
- SCL: `afga_viXi`, stem `\\w*a$`, suffix `root(Bis)`
- Pipeline: pratyaya rule, input "bhis" → output "ais"

**7.1.12** — ṭāṅasisiṅasinām inaṭsyāḥ: after a-stem, ṭā → ina, ṅasi → t, ṅas → sya.
- SCL: `afga_viXi`, stem `\\w*a$`, three separate rule entries
- Pipeline: three pratyaya rules

**7.1.13** — ṅeḥ ya: ṅe → ya after a-stem.
- SCL: `afga_viXi`, stem `\\w*a$`, suffix `root(fe)`
- Pipeline: pratyaya rule, input "e" → output "ya"

**7.1.54** — hrasvanadyāpo nuṭ: ām → nām (nuṬ augment) after short vowel / nadī / āp stems.
- SCL: three variants: (1) stem `\\w*[aiuq]$` for hrasva, (2) stem with `naxI` attrib, (3) stem `\\w*A$` with `Abanwa` attrib
- Pipeline: pratyaya rule, input "ām" → output "nām". Separate entries per stem_class.

### Masculine-specific

**6.1.103** — śas → n: masculine a-stem acc pl.
- Pipeline: pratyaya rule, condition_stem_class "a-stem-m", input "as" → output "n"
- Does NOT apply to neuter or feminine.

**6.1.69** — su → luk (deletion) in sambodhana after short vowel.
- Pipeline: pratyaya rule, condition_vibhakti "sambodhana", input "s" → output ""

### Neuter-specific

**7.1.24** — ato'miti: neuter a-stem nom/acc sg gets pratyaya am.
- SCL: `afga_viXi`, stem `\\w*a$`, attrib `napuMsaka`, suffix `root(su,1)|root(am)`
- Pipeline: pratyaya rule, condition_suffix "su", input "s" → output "am", condition_vibhakti "prathama"
- Then 6.1.107 (ami pūrvaḥ) handles the junction in Layer 4.

**7.1.19** — napuṃsakāt: neuter nom/acc dual au/auṭ → Śī (= ī).
- SCL: `afga_viXi`, stem `\\w*`, attrib `napuMsaka`, suffix `root(O)|root(Ot)`, substitutes `"SI"`
- Pipeline: pratyaya rule, input "au" → output "ī". Then guṇa sandhi a+ī→e in Layer 4.

**7.1.20** — jaśśasoḥ śiḥ: neuter nom/acc plural jas/śas → śi (= i).
- SCL: `afga_viXi`, stem `\\w*`, attrib `napuMsaka`, suffix `root(jas)|root(Sas)`, substitutes `"Si"`
- Pipeline: pratyaya rule, input "as" → output "i". Then nUM-āgama in Layer 3.

**1.1.42** — śi sarvanāmasthānam: śi gets sarvanāmasthāna designation.
- SCL: inline procedural code, not a rule object. Adds `sarvanAmasWAna` attribute when pratyaya has `AxeSa(Si)`.
- Pipeline: not modeled explicitly. The nUM-āgama anga rule in Layer 3 is conditioned on vacana=bahuvacana + suffix_initial="i" which captures the same cases.

### Feminine ā-stem specific

**6.1.68** — su deletion for ā-final stems (prathama and sambodhana).
- SCL: rule for stems `\\w*[IA]$` with attrib `fyanwa|Abanwa` and suffix `root(su,1)`
- Pipeline: pratyaya rule, condition_stem_class "aa-stem-f", input "s" → output ""

**7.1.18** — ā-stem (āp-anta) dual au/auṭ → Śī (= ī).
- SCL: `afga_viXi`, stem `\\w*A$`, attrib `Abanwa`, suffix `root(O)|root(Ot)`, substitutes `"SI"`
- Pipeline: pratyaya rule, input "au" → output "ī". Then guṇa sandhi ā+ī→e.

**7.3.113** — oblique singular suffixes for ā-stems get y-insertion (ṅe → yai, ṅasi → yās, ṅas → yās).
- SCL: `afga_viXi`, stem `\\w*A$`, attrib `Abanwa`, suffix `f-iw` (ṅ-it marker)
- Pipeline: modeled as pratyaya substitutions (suffix changes to y-prefixed forms).

**7.3.116** — saptamī singular for ā-stems: ṅi → yām.
- SCL: `afga_viXi`, stem `\\w*A$`, attrib `Abanwa`, suffix `root(fi)` (ṅi)
- Pipeline: pratyaya rule, input "i" → output "yām"

## Layer 3: aṅga modifications

### a-stem (masculine and neuter shared)

**7.3.101/102** — stem-final a → ā before yaÑ-initial (y, n, bh, t) suffixes.
- Pipeline: anga rules with condition_stem_final "a", various condition_suffix_initial values.
- operation: a → ā (lengthening)

**7.3.103** — stem-final a → e before jhaL-initial suffixes in bahuvacana (saptamī pl: deveṣu, pañcamī/caturthī pl: devebhyaḥ).
- Pipeline: anga rule, condition_stem_final "a", condition_suffix_initial "bh"/"s", condition_vacana "bahuvacana"
- operation: a → e

**7.3.104** — stem-final a → e before os (ṣaṣṭhī/saptamī dual).
- Pipeline: anga rule, condition_stem_final "a", condition_suffix_initial "o"
- operation: a → e

### Neuter-specific

**7.1.72** — nUM-āgama before sarvanāmasthāna (śi) for neuter stems + **6.4.3** — dīrgha (lengthening before nUM).
- Pipeline: combined into one anga rule: condition_stem_final "a", condition_suffix_initial "i", condition_vacana "bahuvacana"
- operation: a → ān (collapses nUM insertion + lengthening)
- Discriminated from saptamī sg (also suffix "i") by vacana condition.

### Feminine ā-stem specific

**7.3.105** — āṅgastriyām: stem-final ā → shortening+y before ṭā and os suffixes.
- SCL: `afga_viXi`, stem `\\w*A$`, attrib `Abanwa`, suffix `root(tA)|root(os)`
- Pipeline: two anga rules:
  - condition_stem_final "ā", condition_suffix_initial "ā" → operation: ā → ay (for ṭā/tṛtīyā sg)
  - condition_stem_final "ā", condition_suffix_initial "o" → operation: ā → ay (for os/ṣaṣṭhī-saptamī du)

**7.3.106** — sāmbodhane ca: stem-final ā → e in sambodhana singular.
- Pipeline: anga rule, condition_stem_final "ā", condition_vibhakti "sambodhana", condition_vacana "ekavacana"
- operation: ā → e

## Layer 4: junction sandhi

### Key rules (already in sandhi-rule.json)

**6.1.101** — savarṇa-dīrgha: a+a→ā, a+ā→ā, ā+a→ā, ā+ā→ā, i+i→ī, u+u→ū (utsarga)

**6.1.107** — ami pūrvaḥ: a+a→a before pratyaya am (apavāda of 6.1.101). Handles deva+am→devam, phala+am→phalam.

**6.1.87** — ādguṇaḥ: a/ā + i/ī → e, a/ā + u/ū → o, a/ā + ṛ/ṝ → ar (guṇa)

**6.1.88** — vṛddhi: a + e → ai, a + o → au, a + ai → ai, a + au → au

**6.1.77** — yaṇ: i+a→ya, i+ā→yā, u+a→va, u+ā→vā, ṛ+a→ra

**6.1.78** — ayādi: e+a→aya, o+a→ava

## Layer 5: tripādī

**8.2.66** — sasajuṣo ruḥ: word-final s → r (then 8.3.15 converts to visarga)

**8.3.15** — kharavasānayorvisarjanīyaḥ: r → ḥ at avasāna (word end)

**8.3.59** — ādeśapratyayayoḥ: s → ṣ after i/u/ṛ/e/o in pratyaya (iUK retroflexion)

## Sūtras needed for future stem types

### i/ī-stems
- **7.3.111** — guṇa of stem-final i/ī before consonant-initial suffixes (i→e, ī→e)
- **7.3.115** — dīrgha of i-stem before ṅit suffixes in feminine (ī-stems)
- **6.4.77** — aci śnudhātubhruvāṃ yvor iyaṅ uvaṅau — i→iy, u→uv before vowel-initial suffixes
- **7.1.73** — iko'ci vibhaktau — nUM before vowel-initial vibhakti for i/u/ṛ-final neuter stems

### u/ū-stems
- Similar to i-stems: guṇa u→o, yaṇ u+a→va
- **7.3.111** applies here too (u→o before consonant-initial)

### Consonant stems (r-stems, n-stems, s-stems)
- **8.2.7** — nalopaḥ prātipadikāntasya (n-deletion for rājan etc.)
- **7.1.70** — ugidacāṃ sarvanāmasthāne'dhātoḥ (nUM before sarvanāmasthāna for ugit stems)
- **7.3.110** — ṛtaḥ idudbhyām — guṇa of ṛ (ṛ→ar) before id/ud
- **6.4.134** — al lopo'naḥ (elision rules for n-stems)
- Various stem-final consonant sandhi rules (8.2.30ff)
