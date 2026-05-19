# Verification System

Panini has two layers of automated verification: **property-based testing** (compile-time, via `cargo test`) and a **rule consistency checker** (runtime, via `/api/check` and the desktop GUI's Verify tab).

Together these provide confidence that the implementation is internally coherent and that the rule data in vidya is well-formed. They are not a substitute for checking correctness against Sanskrit source texts, but they catch a large class of bugs mechanically.

## Property-based testing

**File:** `tests/properties.rs`
**Dependency:** `proptest` (dev-dependency in `Cargo.toml`)
**Run:** `cargo test --test properties`

Proptest generates randomized Sanskrit inputs and checks invariants. Current properties:

| Property | What it verifies |
|---|---|
| `tokenize_round_trip` | `tokenize(s).join("") == s` for any string built from Sanskrit phonemes |
| `tokenize_non_empty_tokens` | No token is ever empty |
| `sandhi_derive_never_empty` | Derivation always produces a non-empty result |
| `sandhi_result_no_longer_than_inputs` | Sandhi doesn't create phonetic material from nothing |
| `sandhi_trace_steps_monotonic` | Trace step numbers are sequential |
| `apavada_beats_utsarga` | An apavada rule always overrides an utsarga rule at the same match point |
| `sandhi_vowel_round_trip_all_pairs` | For known stem/word pairs, `derive` then `analyze` recovers the original split |

### When to update

**Add a new operation (e.g., verb conjugation):** Write new proptest properties for it. Copy the pattern from the sandhi properties — at minimum you want: round-trip (derive then analyze), non-empty output, and trace monotonicity. Define a `sanskrit_word()` strategy (or reuse the existing one) to generate valid inputs.

**Add new sandhi rules in vidya:** No code changes needed. The fixture rules in `properties.rs` are a representative subset used for randomized testing. If the new rules introduce phoneme classes not in the fixture (e.g., retroflex vowels), add them to `fixture_sandhi_rules()` in `properties.rs`.

**Add a new rule priority type:** Update `rule_type_priority()` in `src/engine/mod.rs`. The `apavada_beats_utsarga` property already tests the priority mechanism generically, but add a specific property if the new type has non-obvious interaction semantics.

## Rule consistency checker

**File:** `src/engine/consistency.rs`
**API:** `GET /api/check` (returns JSON array of `CheckReport`)
**Desktop GUI:** Verify tab (`src/gui/verify.rs`)

The checker analyzes the loaded rule data at runtime. It does not execute any derivations — it inspects the rules structurally. It produces one `CheckReport` per template type.

### What it checks per template

#### sandhi_rule

- **Parse errors:** Rules that fail to deserialize as `SandhiParams`
- **Shadowed rules:** Two rules with the same `(first, second)` pattern and the same result, where one can never fire because the other has equal or higher priority
- **Ambiguous overlaps:** Two rules with the same `(first, second)` pattern, different results, and the same priority level (the engine would pick one non-deterministically based on sort order)
- **Coverage:** Unique phoneme pairs, rules per sutra, rules per type
- **Note:** Rules with `condition_pratyaya` (used in declension junction sandhi) are excluded from overlap analysis since they only fire in a specific context

#### sup_suffix

- **Parse errors:** Rules that fail to deserialize as `SupParams`
- **Duplicate cells:** Two rules with the same `(stem_class, vibhakti, vacana)` key (the engine uses `find_map`, so the second is unreachable)
- **Ambiguous cells:** Same key but different `pratyaya/suffix` output
- **Paradigm completeness:** For each stem class, checks that all 24 cells (8 vibhaktis x 3 vacanas) are present. Missing cells will cause the engine to return an error for that case/number combination
- **Coverage:** Stem classes, vibhaktis, vacanas

#### pratyaya_rule

- **Parse errors**
- **Shadowed/ambiguous:** Grouped by `(condition_stem_class, condition_suffix, input_suffix, condition_vibhakti)`. Priority-sorted like sandhi rules
- **Coverage:** Stem classes, pratyayas, rules per sutra

#### anga_rule

- **Parse errors**
- **Shadowed/ambiguous:** Grouped by `(condition_stem_final, condition_suffix_initial, condition_vacana, operation_input)`. Priority-sorted
- **Coverage:** Stem-final phonemes, suffix-initial phonemes, rules per sutra

#### tripadi_rule

- **Parse errors**
- **Shadowed/ambiguous:** Grouped by `(position, input, condition_preceding)`. Priority-sorted
- **Coverage:** Positions (word_final, internal), rules per sutra

### When to update

**Add new rules to an existing template:** No code changes needed — add the JSON to `data/` and the checker runs against whatever is loaded at startup.

**Add a new template type (e.g., `tin_suffix` for verbs):** You need a new `check_*` function in `consistency.rs`. Follow this pattern:

1. Define a local `Params` struct that matches the template's JSON shape (only the fields needed for key/result/priority analysis)
2. Write a `check_foo_rules(rules: &[CachedRule]) -> CheckReport` function
3. Parse all rules, collect parse errors
4. Group by the template's natural match key
5. Call `check_overlap_group()` on each group (handles shadowing and ambiguity detection using priority sort)
6. Build coverage dimensions specific to the template
7. Call `build_summary()` to generate the verdict
8. Add the check call to `api::check()` in `src/api.rs`

The `check_overlap_group()` helper is generic — it takes closures for extracting priority, position, result, sutra, and type from any params struct. You don't need to reimplement the overlap logic.

**Add a new rule_type (priority level):** Update `rule_type_priority()` in `src/engine/mod.rs`. The consistency checker uses the same function, so it will automatically respect the new priority.

**Change how the engine matches rules within a template:** If you change the match key (e.g., add a new condition field that narrows when a rule fires), update the grouping key in the corresponding `check_*` function so it doesn't flag intentional narrowing as ambiguity.

### Desktop GUI

The Verify tab in the Iced GUI (`src/gui/verify.rs`) renders `CheckReport` natively. It handles any number of reports (one per template), variable coverage dimensions, and paradigm gaps. If you add a new template checker, the GUI will render it automatically. If you add a new *kind* of check (beyond parse errors, shadowing, ambiguity, and paradigm gaps), update the rendering in `src/gui/verify.rs`.

## File inventory

| File | Role |
|---|---|
| `src/engine/consistency.rs` | All checker logic: shared types, per-template check functions, unit tests |
| `src/api.rs` | `check()` handler wires checkers to `GET /api/check` |
| `src/main.rs` | Registers `/api/check` route |
| `src/gui/verify.rs` | Desktop GUI verify tab rendering |
| `tests/properties.rs` | Property-based tests with proptest |
| `Cargo.toml` | `proptest` in dev-dependencies |

## Relationship to manual verification

These tools verify internal consistency (rules parse, no conflicts, coverage is complete) and structural invariants (round-tripping, priority ordering). They do *not* verify that the rules are correct Sanskrit — that requires checking against authoritative references like Ruppel's paradigm tables or the Ashtadhyayi commentaries. The full_deva_paradigm test in `src/engine/declension.rs` is an example of a manual correctness test: it hardcodes the 24 expected forms of "deva" and asserts the engine produces them.
