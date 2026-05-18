# Pāṇini PRD

## Problem Statement

Students learning Sanskrit encounter inflected forms and sandhi junctions in texts but have no tool that explains *why* a form looks the way it does. Existing resources give tables to memorize or glossaries to look up, but not the step-by-step derivation with sūtra citations that connects a surface form to its grammatical base. LLM agents working with Sanskrit face the same gap — they can guess at morphology but can't trace derivations with provenance.

The grammar rules exist as structured knowledge in vidya (the manas knowledge store), but vidya is a domain-general system with no reasoning capability. There is no engine to apply those rules, no product to present the results, and no interface for students or agents to explore Sanskrit morphology.

## Solution

Pāṇini is a Sanskrit grammar engine and learning tool. It fetches grammar rules from vidya as structured data, caches them in memory, and provides:

1. **Forward derivation** — given a stem, case, and number, produce the inflected form with a traced derivation citing every sūtra applied.
2. **Reverse analysis** — given a surface form, produce ranked candidate decompositions (sandhi splitting) or morphological identifications (declension analysis).
3. **Paradigm generation** — full 24-form inflection grids (8 cases × 3 numbers) with traces for every cell.
4. **A web UI** — paradigm explorer, sandhi workbench, and sūtra browser for students and demonstration.
5. **MCP tools** — agent-facing interface for programmatic grammar queries.

Pāṇini is a separate product from vidya. Vidya stores the knowledge; Pāṇini reasons over it. See ADR 0001 for rationale.

## User Stories

1. As a Sanskrit student, I want to enter a noun stem and see its full paradigm table with Devanāgarī and IAST, so that I can study inflection patterns.
2. As a Sanskrit student, I want to click any cell in a paradigm table and see the step-by-step derivation with sūtra citations, so that I understand *why* the form looks the way it does.
3. As a Sanskrit student, I want to enter two words and see how they combine via sandhi, with the rule and sūtra that applies, so that I can learn sandhi rules in context.
4. As a Sanskrit student, I want to enter a combined form and see all valid sandhi decompositions ranked by specificity, so that I can figure out word boundaries in continuous text.
5. As a Sanskrit student, I want to see Devanāgarī and IAST side by side throughout the interface, so that I can learn both scripts.
6. As a Sanskrit student, I want to browse the sūtras that Pāṇini knows about, with cross-references to which derivations use them, so that I can explore the grammar system itself.
7. As an LLM agent, I want to call `panini_derive` with a domain, operation, and input, and get back a result with a traced derivation, so that I can answer grammar questions with cited provenance.
8. As an LLM agent, I want to call `panini_analyze` with a surface form and get ranked candidate decompositions, so that I can perform sandhi splitting and morphological analysis.
9. As an LLM agent, I want to call `panini_paradigm` with a stem, stem type, and gender, and get the full 24-form grid with traces, so that I can display or reason over complete inflection data.
10. As an LLM agent, I want to call `panini_health` and see rule cache statistics, so that I can verify the engine is loaded and ready.
11. As a developer, I want Pāṇini to start up, connect to vidya, cache all rules, and serve MCP + HTTP from a single binary, so that deployment is simple.
12. As a developer, I want to add new stem types by seeding new rules into vidya without modifying Pāṇini's engine code, so that grammar coverage expands through data, not code changes.
13. As a developer, I want the sandhi engine to handle vowel, consonant, and visarga sandhi with correct priority resolution (apavāda beats utsarga, later rule wins on conflict), so that derivations match the tradition.
14. As a developer, I want integration tests that verify derivations against known correct forms (Ruppel's paradigm tables), so that the grammar rules serve as their own test suite.

## Implementation Decisions

### Architecture

Single Rust binary serving three interfaces from one process:

- **MCP tools** (stdio + HTTP) — `panini_health`, `panini_derive`, `panini_analyze`, `panini_paradigm`
- **JSON API** (`/api`) — same logic as MCP tools, backing the web UI
- **Web UI** (`/`) — vanilla HTML/CSS/JS embedded as static assets at build time

Deployment follows the manas service pattern: systemd user service, fixed HTTP port. Pāṇini connects to vidya's MCP endpoint as a client.

### Boundary with vidya

Vidya is a pure knowledge store — CRUD and queries, no reasoning. Pāṇini owns all vyākaraṇa reasoning. The existing engine code in vidya (`src/engine/sandhi.rs`, `src/engine/declension.rs`, `src/engine/phoneme.rs`) migrates to Pāṇini. `vidya_derive` and `vidya_analyze` are removed from vidya. See ADR 0001.

### Rule cache

On startup, Pāṇini connects to vidya and fetches all rules by template type (`sandhi_rule`, `sup_suffix`, `pratyaya_rule`, `anga_rule`, `tripadi_rule`). Rules are cached in memory, indexed by template type and rule_type for fast matching. Total footprint: a few hundred rules, ~1-2MB. Refresh requires a restart — rules are curated data, not streaming.

If vidya is unreachable at startup, Pāṇini fails with a clear error. No degraded mode.

### MCP tool design

Tools use `domain` + `operation` params (e.g., `panini_derive(domain="vyakarana", operation="declension", ...)`). Domain is always `vyakarana` for now but keeps the interface open for related domains (Prākṛt, Vedic).

API parameter naming uses English: `stem`, `suffix`, `case`, `number`. Sanskrit terms (`prātipadika`, `pratyaya`, `vibhakti`, `vacana`) appear in sūtra citations within traces, not in the API surface.

### Sandhi engine

Migrated from vidya's `VyakaranaSandhiStrategy`. Handles forward derivation and reverse analysis. MVP coverage:

- **Vowel sandhi** (existing): savarṇa-dīrgha, guṇa, vṛddhi, yaṇ, āyava
- **Visarga sandhi** (existing): s→r, r→ḥ two-step
- **Consonant sandhi** (new): final stops, nasals, class-specific transformations

Conflict resolution follows Pāṇinian convention: apavāda beats utsarga, nitya beats anitya, vipratiṣedhe paraṁ kāryam (later rule wins on conflict). Tripādi rules apply in a late pass.

### Declension engine

Migrated from vidya's `VyakaranaDeclensionStrategy`. Five-layer pipeline:

1. Suffix selection (`sup_suffix`)
2. Pratyaya modification (`pratyaya_rule`)
3. Aṅga modification (`anga_rule`)
4. Junction sandhi (`sandhi_rule`)
5. Tripādi late-pass (`tripadi_rule`)

Each layer loads rules from the cache, matches on input conditions, applies the highest-specificity match. Layers that produce no match are omitted from the trace.

MVP stem type coverage:

- **a-stem masculine** (existing): deva paradigm, 24 forms
- **ā-stem feminine** (new): seed suffix and modification rules
- **i/u-stem** masculine, feminine, neuter (new): seed rules
- **Consonant stems** (new): rājan-type, with stem alternation

New stem types are added by seeding new rules into vidya. The 5-layer pipeline handles them without structural engine changes.

### Web UI

Vanilla HTML/CSS/JS, no framework, no build step. Embedded in the binary as static assets. Three views:

- **Paradigm explorer**: enter a stem, select stem type + gender. See 8×3 grid. Click any cell for its derivation trace with sūtra citations. Devanāgarī + IAST side by side.
- **Sandhi workbench**: forward (two words → combined form + trace) and reverse (combined form → ranked decompositions).
- **Sūtra browser**: list of encoded sūtras with number, text, and which derivations reference them.

### Modules

| Module | Interface | Internals |
|---|---|---|
| Rule cache | `get_rules(template) → Vec<Rule>` | Vidya MCP client, per-template indexing, type-specific deserialization |
| Sandhi engine | `derive(rules, input) → (result, trace)`, `analyze(rules, form) → Vec<Candidate>` | Phoneme classification, rule matching, priority resolution, reverse enumeration |
| Declension engine | `derive(rules, stem, case, number) → (form, trace)` | 5-layer pipeline, per-layer rule matching, specificity scoring |
| MCP server | Tool registration + dispatch | Thin wiring to engines |
| API server | JSON endpoints | Thin wiring, same logic as MCP |
| Web UI | Static HTML/CSS/JS | Fetch from /api, DOM rendering |

## Testing Decisions

### What makes a good test

Tests validate derivation correctness through Pāṇini's public interfaces — give an input, check the output form and trace. The Sanskrit language itself is the test suite: every correctly derived form validates the rules, every incorrect form is a bug. Ruppel's paradigm tables serve as expected output.

### Rule cache tests

Mock vidya MCP responses. Verify: correct deserialization of all template types, proper indexing by template type and rule_type, error handling when vidya is unreachable, idempotent cache refresh.

### Sandhi engine tests

Fixture rules (not live vidya). Test per sandhi category:

- Vowel sandhi: savarṇa-dīrgha, guṇa, vṛddhi, yaṇ, āyava — forward and reverse
- Visarga sandhi: the two-step s→r→ḥ pathway — forward and reverse
- Consonant sandhi: final stops, nasals — forward and reverse
- Priority resolution: apavāda beating utsarga, tripādi ordering

Carry forward existing test cases from vidya (10 vowel sandhi cases) and expand.

### Declension engine tests

Fixture rules. Test per stem type:

- a-stem masculine: full 24-form paradigm against Ruppel's deva table
- ā-stem feminine: full paradigm against Ruppel
- i/u-stems: full paradigms
- Consonant stems: full paradigms with stem alternation verification

Each test verifies both the output form and the trace structure (correct sūtras cited in correct order).

### Integration tests

Full startup → vidya connection → derivation cycle. Verify the complete path from MCP tool call to traced result against a running vidya instance with seeded rules.

## Out of Scope

- **Sentence segmentation** — Zen-style automata/transducer layer for splitting continuous text into words. Major subsystem, post-MVP.
- **Verb conjugation** — dhātu + lakāra → conjugation table. Separate engine strategy, after declension is solid.
- **Compounds (samāsa)** — analysis and generation. Depends on the segmenter.
- **Pedagogical sequencing** — exercise generation, curriculum ordering, progress tracking, spaced repetition.
- **Trie/DAG storage** — compact paradigm table serving at scale. Optimization for later.
- **Declarative rule DSL** — generic rule interpreter replacing compiled strategies. Deferred per vidya PRD until patterns emerge from 2+ strategies.
- **Multi-tradition support** — MVP assumes one canonical Pāṇinian tradition.
- **Mobile / responsive UI** — desktop-first.
- **vidya refactoring** — removing engine code and derive/analyze from vidya is a prerequisite tracked separately.

## Further Notes

### Dependency on vidya refactoring

Pāṇini's development can proceed independently — the engine code is migrated (copied), not moved in place. The vidya cleanup (removing engine strategies, `vidya_derive`, `vidya_analyze`) is a separate work stream that should happen once Pāṇini is functional and the old code is no longer needed.

### Ruppel MVP goal

The target is a working tool to demonstrate to Ajay Rao and potentially Antonia Ruppel. The MVP should handle the grammar covered in Ruppel's first several chapters: sandhi (vowel, consonant, visarga) and noun declension (a-stems, ā-stems, i/u-stems, consonant stems). The web UI is the demo surface; the MCP tools are the durable product.

### Seed data expansion

New stem types and sandhi rules require seed data in vidya. The declension pipeline handles new stem types through data (new rules), not code. Seed data authoring is manual JSON for the MVP — the same pattern vidya uses today.

### Reference oracle

The Sanskrit Heritage Platform (Zen) serves as an independent reference for validating Pāṇini's derivations. Agreement validates correctness; disagreement pinpoints investigation targets.
