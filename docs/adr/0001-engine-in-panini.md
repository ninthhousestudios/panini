# Grammar engine lives in Pāṇini, not vidya

Vidya was designed as a "structured knowledge graph with domain-specific reasoning" — knowledge storage and rule application in one system. We're splitting that: vidya becomes a pure knowledge store (CRUD + queries), and all vyākaraṇa reasoning (sandhi, declension, future operations) moves to Pāṇini.

The reason: vidya should be domain-general. The current engine strategy trait (`EngineStrategy`) requires compiled Rust for every new domain, which is the opposite of general. By moving reasoning out, vidya serves any knowledge domain without modification. Products that need reasoning over their domain's knowledge (Pāṇini for grammar, a future jyotiṣa interpreter, etc.) fetch rules from vidya as structured data and run their own engines.

The cost: vidya loses `vidya_derive` and `vidya_analyze`. It can no longer self-validate its own rules via derivation. Pāṇini takes over that responsibility. Vidya's existing engine code (sandhi strategy, declension pipeline, phoneme helpers) migrates to Pāṇini.

## Considered Options

- **Keep engine in vidya** — co-locates knowledge and reasoning, but forces every reasoning domain to modify vidya's source. Violates generality.
- **Engine as a shared library crate** — both vidya and Pāṇini depend on a `vyakarana-engine` crate. Preserves vidya_derive but adds a third project to maintain for no clear benefit.
- **Engine in Pāṇini only** (chosen) — cleanest separation. Vidya stays general. Pāṇini owns all vyākaraṇa-specific logic.
