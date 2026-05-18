use proptest::prelude::*;

use panini::engine::phoneme::tokenize;
use panini::engine::sandhi::{SandhiInput, analyze_sandhi, derive_sandhi};
use panini::rule_cache::CachedRule;

fn fixture_sandhi_rules() -> Vec<CachedRule> {
    let rules_json = vec![
        serde_json::json!({"first": "a", "second": "a", "result": "ā", "sutra": "6.1.101", "sutra_position": "06.01.101", "rule_type": "utsarga"}),
        serde_json::json!({"first": "a", "second": "i", "result": "e", "sutra": "6.1.87", "sutra_position": "06.01.087", "rule_type": "utsarga"}),
        serde_json::json!({"first": "a", "second": "u", "result": "o", "sutra": "6.1.87", "sutra_position": "06.01.087", "rule_type": "utsarga"}),
        serde_json::json!({"first": "a", "second": "e", "result": "ai", "sutra": "6.1.88", "sutra_position": "06.01.088", "rule_type": "utsarga"}),
        serde_json::json!({"first": "a", "second": "o", "result": "au", "sutra": "6.1.88", "sutra_position": "06.01.088", "rule_type": "utsarga"}),
        serde_json::json!({"first": "i", "second": "a", "result": "ya", "sutra": "6.1.77", "sutra_position": "06.01.077", "rule_type": "utsarga"}),
        serde_json::json!({"first": "u", "second": "a", "result": "va", "sutra": "6.1.77", "sutra_position": "06.01.077", "rule_type": "utsarga"}),
        serde_json::json!({"first": "i", "second": "i", "result": "ī", "sutra": "6.1.101", "sutra_position": "06.01.101", "rule_type": "utsarga"}),
        serde_json::json!({"first": "u", "second": "u", "result": "ū", "sutra": "6.1.101", "sutra_position": "06.01.101", "rule_type": "utsarga"}),
        serde_json::json!({"first": "ṛ", "second": "a", "result": "ra", "sutra": "6.1.77", "sutra_position": "06.01.077", "rule_type": "utsarga"}),
        serde_json::json!({"first": "aḥ", "second": "a", "result": "o '", "sutra": "6.1.109", "sutra_position": "06.01.109", "rule_type": "utsarga"}),
        serde_json::json!({"first": "aḥ", "second": "g", "result": "og", "sutra": "8.3.17", "sutra_position": "08.03.017", "rule_type": "utsarga"}),
        serde_json::json!({"first": "t", "second": "c", "result": "cc", "sutra": "8.4.40", "sutra_position": "08.04.040", "rule_type": "utsarga"}),
        serde_json::json!({"first": "t", "second": "j", "result": "jj", "sutra": "8.4.40", "sutra_position": "08.04.040", "rule_type": "utsarga"}),
        serde_json::json!({"first": "t", "second": "g", "result": "dg", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
        serde_json::json!({"first": "t", "second": "d", "result": "dd", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
        serde_json::json!({"first": "t", "second": "b", "result": "db", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
        serde_json::json!({"first": "t", "second": "n", "result": "nn", "sutra": "8.4.45", "sutra_position": "08.04.045", "rule_type": "utsarga"}),
        serde_json::json!({"first": "t", "second": "m", "result": "nm", "sutra": "8.4.45", "sutra_position": "08.04.045", "rule_type": "utsarga"}),
        serde_json::json!({"first": "m", "second": "k", "result": "ṃk", "sutra": "8.3.23", "sutra_position": "08.03.023", "rule_type": "utsarga"}),
        serde_json::json!({"first": "m", "second": "t", "result": "ṃt", "sutra": "8.3.23", "sutra_position": "08.03.023", "rule_type": "utsarga"}),
        serde_json::json!({"first": "m", "second": "p", "result": "ṃp", "sutra": "8.3.23", "sutra_position": "08.03.023", "rule_type": "utsarga"}),
        serde_json::json!({"first": "t", "second": "a", "result": "da", "sutra": "8.2.39", "sutra_position": "08.02.039", "rule_type": "utsarga"}),
        serde_json::json!({"first": "t", "second": "i", "result": "di", "sutra": "8.2.39", "sutra_position": "08.02.039", "rule_type": "utsarga"}),
        serde_json::json!({"first": "t", "second": "u", "result": "du", "sutra": "8.2.39", "sutra_position": "08.02.039", "rule_type": "utsarga"}),
        serde_json::json!({"first": "k", "second": "a", "result": "ga", "sutra": "8.2.39", "sutra_position": "08.02.039", "rule_type": "utsarga"}),
        serde_json::json!({"first": "p", "second": "a", "result": "ba", "sutra": "8.2.39", "sutra_position": "08.02.039", "rule_type": "utsarga"}),
    ];
    rules_json
        .into_iter()
        .enumerate()
        .map(|(i, params)| CachedRule {
            params,
            statement: format!("test rule {}", i + 1),
        })
        .collect()
}

const CONSONANTS: &[&str] = &[
    "k", "kh", "g", "gh", "ṅ", "c", "ch", "j", "jh", "ñ", "ṭ", "ṭh", "ḍ", "ḍh", "ṇ", "t",
    "th", "d", "dh", "n", "p", "ph", "b", "bh", "m", "y", "r", "l", "v", "ś", "ṣ", "s", "h",
];

const VOWELS: &[&str] = &["a", "ā", "i", "ī", "u", "ū", "ṛ", "e", "ai", "o", "au"];

fn sanskrit_syllable() -> impl Strategy<Value = String> {
    let onset = prop::sample::select(CONSONANTS);
    let nucleus = prop::sample::select(VOWELS);
    (onset, nucleus).prop_map(|(c, v)| format!("{c}{v}"))
}

fn sanskrit_word(min_syl: usize, max_syl: usize) -> impl Strategy<Value = String> {
    prop::collection::vec(sanskrit_syllable(), min_syl..=max_syl)
        .prop_map(|syls| syls.concat())
}

proptest! {
    #[test]
    fn tokenize_round_trip(s in sanskrit_word(1, 4)) {
        let tokens = tokenize(&s);
        let reconstructed: String = tokens.into_iter().collect();
        prop_assert_eq!(&reconstructed, &s, "tokenize round-trip failed for {:?}", s);
    }

    #[test]
    fn tokenize_non_empty_tokens(s in sanskrit_word(1, 3)) {
        let tokens = tokenize(&s);
        for (i, tok) in tokens.iter().enumerate() {
            prop_assert!(!tok.is_empty(), "token {i} is empty for input {s:?}");
        }
    }

    #[test]
    fn sandhi_derive_never_empty(
        first in sanskrit_word(1, 3),
        second in sanskrit_word(1, 3),
    ) {
        let rules = fixture_sandhi_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput { first: first.clone(), second: second.clone() },
        ).unwrap();
        let combined = result.output["result"].as_str().unwrap();
        prop_assert!(!combined.is_empty(), "empty result for {first} + {second}");
    }

    #[test]
    fn sandhi_result_no_longer_than_inputs(
        first in sanskrit_word(1, 3),
        second in sanskrit_word(1, 3),
    ) {
        let rules = fixture_sandhi_rules();
        let input_len = first.len() + second.len();
        let result = derive_sandhi(
            &rules,
            SandhiInput { first: first.clone(), second: second.clone() },
        ).unwrap();
        let combined = result.output["result"].as_str().unwrap();
        // Sandhi may add the avagraha (') + space, but the phonemic content
        // should never exceed the input. Allow 2 bytes overhead for avagraha cases.
        prop_assert!(
            combined.len() <= input_len + 4,
            "result {combined:?} too long for {first} + {second} (input bytes: {input_len})"
        );
    }

    #[test]
    fn sandhi_trace_steps_monotonic(
        first in sanskrit_word(1, 3),
        second in sanskrit_word(1, 3),
    ) {
        let rules = fixture_sandhi_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput { first, second },
        ).unwrap();
        for (i, step) in result.trace.iter().enumerate() {
            prop_assert_eq!(step.step, i + 1, "trace step numbers not monotonic");
        }
    }

    #[test]
    fn apavada_beats_utsarga(
        prefix in sanskrit_word(1, 2),
    ) {
        // a + i with both utsarga and apavāda rules: apavāda should win
        let mut rules = fixture_sandhi_rules();
        rules.push(CachedRule {
            params: serde_json::json!({
                "first": "a", "second": "i", "result": "APAVADA",
                "sutra": "test.apavada", "sutra_position": "99.99.999",
                "rule_type": "apavāda"
            }),
            statement: "test apavāda override".into(),
        });

        let first = format!("{prefix}a");
        let result = derive_sandhi(
            &rules,
            SandhiInput { first: first.clone(), second: "iti".into() },
        ).unwrap();
        let combined = result.output["result"].as_str().unwrap();
        prop_assert!(
            combined.contains("APAVADA"),
            "apavāda should override utsarga for {first} + iti, got {combined}"
        );
    }
}

#[test]
fn sandhi_vowel_round_trip_all_pairs() {
    let rules = fixture_sandhi_rules();
    let stems = ["deva", "rāma", "hari", "guru", "pitṛ"];
    let seconds = ["atra", "iti", "udaya", "eṣa", "indra"];

    let mut failures = Vec::new();

    for first in &stems {
        for second in &seconds {
            let result = derive_sandhi(
                &rules,
                SandhiInput {
                    first: first.to_string(),
                    second: second.to_string(),
                },
            )
            .unwrap();
            let combined = result.output["result"].as_str().unwrap();

            if result.trace.is_empty() {
                continue;
            }

            let analysis = analyze_sandhi(&rules, combined).unwrap();
            let found = analysis
                .candidates
                .iter()
                .any(|c| c.first == *first && c.second == *second);
            if !found {
                failures.push(format!(
                    "{first} + {second} → {combined}: not found in analysis candidates"
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "round-trip failures:\n{}",
        failures.join("\n")
    );
}
