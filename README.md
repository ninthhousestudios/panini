# Panini

Sanskrit grammar engine with embedded rule data. Derives inflected forms with step-by-step sutra citations, splits sandhi, generates paradigm tables — all standalone, no external services required.

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

## Running

### Desktop GUI (default)

```sh
cargo run
```

Launches the Iced desktop GUI with IAST-to-Devanagari transliteration, paradigm tables, sandhi tools, and sutra browser.

### HTTP server (MCP + API)

```sh
cargo run -- serve
```

Starts the HTTP server on `127.0.0.1:4300` (default). Exposes the MCP endpoint at `/mcp` and REST API routes at `/api/*`. No web UI — use the desktop GUI for interactive use.

### MCP server

```sh
cargo run -- serve --stdio
```

Runs as an MCP server over stdin/stdout for agent use.

### Options

```
panini [gui|serve] [OPTIONS]

--vidya-url URL  Fetch rules from a vidya MCP endpoint instead of using embedded data
--stdio          (serve) Run as MCP server over stdin/stdout
--http-port N    (serve) Override the HTTP port (default: 4300, env: PANINI_HTTP_PORT)
--auth-token-file PATH  (serve) Enable bearer token auth from a file
```

### Environment

| Variable | Default | Description |
|---|---|---|
| `VIDYA_URL` | (none) | If set, fetch rules from this vidya MCP endpoint instead of embedded data |
| `VIDYA_AUTH_TOKEN` | (none) | Bearer token for vidya (only used with `VIDYA_URL`) |
| `PANINI_LOG_LEVEL` | `info` | Tracing filter (e.g., `debug`, `panini=trace`) |
| `PANINI_HTTP_HOST` | `127.0.0.1` | Bind address |
| `PANINI_HTTP_PORT` | `4300` | HTTP port |

## Rule data

Grammar rules live in `data/` as JSON files, one per template:

| File | Rules | Content |
|---|---|---|
| `sandhi-rule.json` | 167 | Vowel, consonant, and visarga sandhi |
| `sup-suffix.json` | 24 | sUP pratyaya table (8 cases x 3 numbers) |
| `pratyaya-rule.json` | 8 | Suffix modifications by stem class |
| `anga-rule.json` | 7 | Stem modifications before suffixes |
| `tripadi-rule.json` | 3 | Late-pass rules (Ashtadhyayi 8.2-8.4) |

These are compiled into the binary via `include_str!`. To add a new rule template (e.g., verb conjugations): add the JSON file to `data/`, add one line to `EMBEDDED_TEMPLATES` in `src/rule_cache.rs`.

## API

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
cargo test                       # all 96 tests
cargo test --lib                 # unit tests only
cargo test --test integration    # integration tests (uses embedded rules)
cargo test --test properties     # property-based tests
```

## Architecture

Grammar rules are embedded as JSON and loaded at startup into a `RuleCache`. The engine does all reasoning locally. Optionally, rules can be fetched from a [vidya](../manas/vidya) MCP endpoint via `--vidya-url` for development or when vidya hosts commentary-tradition data.

The derivation engine is a five-layer pipeline (for declension):
1. **Suffix selection** (sup_suffix) — look up the pratyaya for stem class + case + number
2. **Pratyaya modification** (pratyaya_rule) — transform the suffix (e.g., bhis -> ais for a-stems)
3. **Anga modification** (anga_rule) — transform the stem (e.g., a -> aa before certain suffixes)
4. **Junction sandhi** (sandhi_rule) — apply sandhi at the stem/suffix boundary
5. **Tripadi** (tripadi_rule) — late-pass rules from Ashtadhyayi 8.2-8.4

Each layer emits a trace step only if it fires. The trace is the derivation proof — it shows which sutra applied at each step and why.

See [docs/verification-system.md](docs/verification-system.md) for the automated verification system and how to extend it.
