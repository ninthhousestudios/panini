# Panini

Sanskrit grammar engine. Fetches rules from [vidya](../manas/vidya) as structured data, caches them in memory, and derives inflected forms with step-by-step sutra citations.

## What it does

- **Forward derivation** — stem + case + number -> inflected form with a traced derivation citing every sutra applied
- **Reverse analysis** — surface form -> ranked candidate decompositions (sandhi splitting, declension analysis)
- **Paradigm generation** — full 8x3 inflection grids with traces for every cell
- **Sandhi** — vowel, consonant, and visarga sandhi with correct priority resolution (apavada > nitya > paribhasha > utsarga)
- **Verification** — automated consistency checks on the rule set (see [docs/verification-system.md](docs/verification-system.md))

## Current coverage

- Sandhi: vowel (6.1.77, 6.1.87, 6.1.88, 6.1.101), visarga (6.1.109, 8.3.17), consonant class assimilation (8.4.40, 8.4.41), voicing (8.4.53, 8.2.39), nasal assimilation (8.4.45), anusvara (8.3.23)
- Declension: a-stem masculine (deva paradigm, all 24 forms)

## Prerequisites

- Rust (edition 2024)
- A running [vidya](../manas/vidya) instance with Sanskrit grammar rules seeded

## Running

```sh
# Start vidya first, then:
cargo run -- serve
```

This starts the HTTP server on `127.0.0.1:4300` (default). Open http://127.0.0.1:4300 in a browser for the web UI.

The web UI has four tabs:
- **Paradigms** — enter a stem and stem type, get the full declension grid in Devanagari and IAST
- **Sandhi** — forward (join two words) and reverse (split a combined form) sandhi
- **Sutras** — browse all loaded sutras with search
- **Verification** — automated consistency report across all rule templates

### Options

```
panini serve [OPTIONS]

--stdio         Run as MCP server over stdin/stdout (for agent use)
--http-port N   Override the HTTP port (default: 4300, env: PANINI_HTTP_PORT)
--auth-token-file PATH   Enable bearer token auth from a file
```

### Environment

| Variable | Default | Description |
|---|---|---|
| `VIDYA_URL` | `http://127.0.0.1:3300/mcp` | Vidya MCP endpoint |
| `VIDYA_AUTH_TOKEN` | (none) | Bearer token for vidya |
| `PANINI_LOG_LEVEL` | `info` | Tracing filter (e.g., `debug`, `panini=trace`) |
| `PANINI_HTTP_HOST` | `127.0.0.1` | Bind address |
| `PANINI_HTTP_PORT` | `4300` | HTTP port |

## API

All endpoints except health require vidya to be running with seeded rules.

| Method | Path | Description |
|---|---|---|
| GET | `/api/health` | Cache stats |
| POST | `/api/derive` | Forward derivation (sandhi or declension) |
| POST | `/api/analyze` | Reverse analysis (sandhi or declension) |
| POST | `/api/paradigm` | Full paradigm grid |
| GET | `/api/sutras` | All loaded sutras |
| GET | `/api/check` | Rule consistency reports |

## Testing

```sh
cargo test               # all tests (unit + property-based)
cargo test --lib         # unit tests only (no vidya needed)
cargo test --test properties  # property-based tests only (no vidya needed)
```

Integration tests in `tests/integration.rs` require a running vidya instance.

## Architecture

Vidya stores the grammar rules as structured JSON claims. Panini fetches them at startup, caches them, and does all reasoning. Adding new rules means seeding new claims in vidya; no Rust code changes.

The derivation engine is a five-layer pipeline (for declension):
1. **Suffix selection** (sup_suffix) — look up the pratyaya for stem class + case + number
2. **Pratyaya modification** (pratyaya_rule) — transform the suffix (e.g., bhis -> ais for a-stems)
3. **Anga modification** (anga_rule) — transform the stem (e.g., a -> aa before certain suffixes)
4. **Junction sandhi** (sandhi_rule) — apply sandhi at the stem/suffix boundary
5. **Tripadi** (tripadi_rule) — late-pass rules from Ashtadhyayi 8.2-8.4

Each layer emits a trace step only if it fires. The trace is the derivation proof — it shows which sutra applied at each step and why.

See [docs/verification-system.md](docs/verification-system.md) for the automated verification system and how to extend it.
