use panini::config::Config;
use panini::engine::declension::{DeclensionInput, analyze_declension, derive_declension};
use panini::engine::sandhi::{SandhiInput, analyze_sandhi, derive_sandhi};
use panini::engine::TraceStep;
use panini::rule_cache::RuleCache;
use panini::vidya_client::VidyaClient;

async fn build_cache() -> RuleCache {
    dotenvy::dotenv().ok();
    let cfg = Config::from_env();
    let vidya = VidyaClient::connect(&cfg.vidya_url, cfg.vidya_auth_token.as_deref())
        .await
        .expect("failed to connect to vidya — is it running?");

    let mut cache = RuleCache::new();
    for template in ["sandhi_rule", "sup_suffix", "pratyaya_rule", "anga_rule", "tripadi_rule"] {
        let claims = vidya
            .fetch_claims("vyakarana", template)
            .await
            .unwrap_or_else(|e| panic!("failed to fetch {template}: {e}"));
        cache.load_template(template.into(), claims);
    }
    assert!(cache.rule_count("sandhi_rule") > 0, "no sandhi rules loaded from vidya");
    cache
}

#[tokio::test]
async fn derive_vowel_sandhi_guna() {
    let cache = build_cache().await;
    let rules = cache.get_rules("sandhi_rule");
    let result = derive_sandhi(
        rules,
        SandhiInput {
            first: "a".into(),
            second: "i".into(),
        },
    )
    .unwrap();
    assert_eq!(result.output["result"], "e");
    assert_eq!(result.trace.len(), 1);
    assert_eq!(result.trace[0].rule_ref.as_deref(), Some("6.1.87"));
    assert_eq!(result.trace[0].input_state, "a + i");
    assert_eq!(result.trace[0].output_state, "e");
}

#[tokio::test]
async fn derive_vowel_sandhi_all_ten_cases() {
    let cache = build_cache().await;
    let rules = cache.get_rules("sandhi_rule");

    let cases = vec![
        ("a", "a", "ā"),
        ("a", "i", "e"),
        ("a", "u", "o"),
        ("a", "e", "ai"),
        ("a", "o", "au"),
        ("i", "a", "ya"),
        ("u", "a", "va"),
        ("i", "i", "ī"),
        ("u", "u", "ū"),
        ("ṛ", "a", "ra"),
    ];

    for (first, second, expected) in cases {
        let result = derive_sandhi(
            rules,
            SandhiInput {
                first: first.into(),
                second: second.into(),
            },
        )
        .unwrap();
        assert_eq!(
            result.output["result"], expected,
            "{first} + {second} should be {expected}"
        );
    }
}

#[tokio::test]
async fn derive_visarga_sandhi_before_a() {
    let cache = build_cache().await;
    let rules = cache.get_rules("sandhi_rule");
    let result = derive_sandhi(
        rules,
        SandhiInput {
            first: "devaḥ".into(),
            second: "atra".into(),
        },
    )
    .unwrap();
    assert_eq!(result.output["result"], "devo 'tra");
    assert_eq!(result.trace[0].rule_ref.as_deref(), Some("6.1.109"));
}

#[tokio::test]
async fn derive_visarga_sandhi_before_voiced_consonant() {
    let cache = build_cache().await;
    let rules = cache.get_rules("sandhi_rule");
    let result = derive_sandhi(
        rules,
        SandhiInput {
            first: "devaḥ".into(),
            second: "gacchati".into(),
        },
    )
    .unwrap();
    assert_eq!(result.output["result"], "devogacchati");
    assert_eq!(result.trace[0].rule_ref.as_deref(), Some("8.3.17"));
}

#[tokio::test]
async fn analyze_vowel_sandhi_round_trip() {
    let cache = build_cache().await;
    let rules = cache.get_rules("sandhi_rule");

    let cases = [
        ("deva", "indra"),
        ("deva", "artha"),
        ("deva", "udaya"),
        ("devi", "atra"),
    ];

    for (first, second) in cases {
        let derived = derive_sandhi(
            rules,
            SandhiInput { first: first.into(), second: second.into() },
        ).unwrap();
        let combined = derived.output["result"].as_str().unwrap();
        let analyzed = analyze_sandhi(rules, combined).unwrap();
        let found = analyzed.candidates.iter().any(|c| c.first == first && c.second == second);
        assert!(found, "round-trip failed: {} + {} → {}", first, second, combined);
    }
}

#[tokio::test]
async fn analyze_visarga_round_trip() {
    let cache = build_cache().await;
    let rules = cache.get_rules("sandhi_rule");

    let derived = derive_sandhi(
        rules,
        SandhiInput { first: "devaḥ".into(), second: "atra".into() },
    ).unwrap();
    let combined = derived.output["result"].as_str().unwrap();
    let analyzed = analyze_sandhi(rules, combined).unwrap();
    let found = analyzed.candidates.iter().any(|c| c.first == "devaḥ" && c.second == "atra");
    assert!(found, "visarga round-trip failed: devaḥ + atra → {}", combined);
}

#[tokio::test]
async fn health_returns_rule_counts() {
    let cache = build_cache().await;
    assert!(cache.template_count() >= 1);
    assert!(cache.total_rules() > 0);
    assert!(cache.rule_count("sandhi_rule") > 0);
}

#[tokio::test]
async fn fails_if_vidya_unreachable() {
    let result = VidyaClient::connect("http://127.0.0.1:19999/mcp", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn declension_cache_loads_all_templates() {
    let cache = build_cache().await;
    assert!(cache.rule_count("sup_suffix") > 0, "no sup_suffix rules");
    assert!(cache.rule_count("pratyaya_rule") > 0, "no pratyaya rules");
    assert!(cache.rule_count("anga_rule") > 0, "no anga rules");
    assert!(cache.rule_count("tripadi_rule") > 0, "no tripadi rules");
}

#[tokio::test]
async fn derive_deva_paradigm() {
    let cache = build_cache().await;
    let expected = vec![
        ("1", "sg", "devaḥ"),
        ("1", "du", "devau"),
        ("1", "pl", "devāḥ"),
        ("2", "sg", "devam"),
        ("2", "du", "devau"),
        ("2", "pl", "devān"),
        ("3", "sg", "devena"),
        ("3", "du", "devābhyām"),
        ("3", "pl", "devaiḥ"),
        ("4", "sg", "devāya"),
        ("4", "du", "devābhyām"),
        ("4", "pl", "devebhyaḥ"),
        ("5", "sg", "devāt"),
        ("5", "du", "devābhyām"),
        ("5", "pl", "devebhyaḥ"),
        ("6", "sg", "devasya"),
        ("6", "du", "devayoḥ"),
        ("6", "pl", "devānām"),
        ("7", "sg", "deve"),
        ("7", "du", "devayoḥ"),
        ("7", "pl", "deveṣu"),
        ("8", "sg", "deva"),
        ("8", "du", "devau"),
        ("8", "pl", "devāḥ"),
    ];
    for (case, number, exp) in expected {
        let result = derive_declension(
            cache.get_rules("sup_suffix"),
            cache.get_rules("pratyaya_rule"),
            cache.get_rules("anga_rule"),
            cache.get_rules("sandhi_rule"),
            cache.get_rules("tripadi_rule"),
            DeclensionInput {
                stem: "deva".into(),
                stem_type: "a-stem-m".into(),
                case: case.into(),
                number: number.into(),
            },
        )
        .unwrap();
        let form = result.output["form"].as_str().unwrap();
        assert_eq!(form, exp, "case={case} number={number} expected={exp}");
        assert!(
            result.trace.iter().any(|t| t.rule_ref.is_some()),
            "trace should include sutra citations for case={case} number={number}"
        );
    }
}

fn generate_paradigm(
    cache: &RuleCache,
    stem: &str,
    stem_type: &str,
) -> Vec<(String, String, Result<(String, Vec<TraceStep>), String>)> {
    let cases = ["1", "2", "3", "4", "5", "6", "7", "8"];
    let numbers = ["sg", "du", "pl"];
    let mut cells = Vec::with_capacity(24);
    for case in cases {
        for number in numbers {
            let input = DeclensionInput {
                stem: stem.into(),
                stem_type: stem_type.into(),
                case: case.into(),
                number: number.into(),
            };
            match derive_declension(
                cache.get_rules("sup_suffix"),
                cache.get_rules("pratyaya_rule"),
                cache.get_rules("anga_rule"),
                cache.get_rules("sandhi_rule"),
                cache.get_rules("tripadi_rule"),
                input,
            ) {
                Ok(result) => {
                    let form = result.output["form"].as_str().unwrap().to_string();
                    cells.push((case.into(), number.into(), Ok((form, result.trace))));
                }
                Err(e) => {
                    cells.push((case.into(), number.into(), Err(e.to_string())));
                }
            }
        }
    }
    cells
}

#[tokio::test]
async fn paradigm_deva_complete() {
    let cache = build_cache().await;
    let cells = generate_paradigm(&cache, "deva", "a-stem-m");

    assert_eq!(cells.len(), 24, "paradigm should have 24 cells");

    let expected = vec![
        ("1", "sg", "devaḥ"),
        ("1", "du", "devau"),
        ("1", "pl", "devāḥ"),
        ("2", "sg", "devam"),
        ("2", "du", "devau"),
        ("2", "pl", "devān"),
        ("3", "sg", "devena"),
        ("3", "du", "devābhyām"),
        ("3", "pl", "devaiḥ"),
        ("4", "sg", "devāya"),
        ("4", "du", "devābhyām"),
        ("4", "pl", "devebhyaḥ"),
        ("5", "sg", "devāt"),
        ("5", "du", "devābhyām"),
        ("5", "pl", "devebhyaḥ"),
        ("6", "sg", "devasya"),
        ("6", "du", "devayoḥ"),
        ("6", "pl", "devānām"),
        ("7", "sg", "deve"),
        ("7", "du", "devayoḥ"),
        ("7", "pl", "deveṣu"),
        ("8", "sg", "deva"),
        ("8", "du", "devau"),
        ("8", "pl", "devāḥ"),
    ];

    let mut errors = Vec::new();
    for (case, number, exp) in expected {
        let cell = cells
            .iter()
            .find(|(c, n, _)| c == case && n == number)
            .unwrap_or_else(|| panic!("missing cell case={case} number={number}"));
        match &cell.2 {
            Ok((form, trace)) => {
                if form != exp {
                    errors.push(format!("case={case} number={number}: got {form}, expected {exp}"));
                }
                if !trace.iter().any(|t| t.rule_ref.is_some()) {
                    errors.push(format!("case={case} number={number}: trace missing sūtra citations"));
                }
            }
            Err(e) => {
                errors.push(format!("case={case} number={number}: derivation failed: {e}"));
            }
        }
    }
    assert!(errors.is_empty(), "paradigm errors:\n{}", errors.join("\n"));
}

#[tokio::test]
async fn paradigm_cell_errors_dont_fail_whole_request() {
    let cache = build_cache().await;
    let cells = generate_paradigm(&cache, "deva", "nonexistent-stem-type");
    assert_eq!(cells.len(), 24, "should still produce 24 cells");
    for (case, number, result) in &cells {
        assert!(
            result.is_err(),
            "case={case} number={number}: expected error for nonexistent stem type"
        );
    }
}

#[tokio::test]
async fn analyze_declension_round_trip() {
    let cache = build_cache().await;
    let spot_checks = [
        ("1", "sg", "devaḥ"),
        ("3", "sg", "devena"),
        ("4", "pl", "devebhyaḥ"),
        ("7", "pl", "deveṣu"),
    ];
    for (case, number, expected_form) in spot_checks {
        let derived = derive_declension(
            cache.get_rules("sup_suffix"),
            cache.get_rules("pratyaya_rule"),
            cache.get_rules("anga_rule"),
            cache.get_rules("sandhi_rule"),
            cache.get_rules("tripadi_rule"),
            DeclensionInput {
                stem: "deva".into(),
                stem_type: "a-stem-m".into(),
                case: case.into(),
                number: number.into(),
            },
        )
        .unwrap();
        assert_eq!(
            derived.output["form"].as_str().unwrap(),
            expected_form,
            "derive mismatch for case={case} number={number}"
        );

        let analyzed = analyze_declension(
            cache.get_rules("sup_suffix"),
            cache.get_rules("pratyaya_rule"),
            cache.get_rules("anga_rule"),
            cache.get_rules("sandhi_rule"),
            cache.get_rules("tripadi_rule"),
            expected_form,
        )
        .unwrap();
        assert!(
            analyzed
                .candidates
                .iter()
                .any(|c| c.stem == "deva" && c.case == case && c.number == number),
            "round-trip failed: case={case} number={number} form={expected_form}"
        );
    }
}

#[tokio::test]
async fn analyze_declension_ambiguous() {
    let cache = build_cache().await;
    let analyzed = analyze_declension(
        cache.get_rules("sup_suffix"),
        cache.get_rules("pratyaya_rule"),
        cache.get_rules("anga_rule"),
        cache.get_rules("sandhi_rule"),
        cache.get_rules("tripadi_rule"),
        "devābhyām",
    )
    .unwrap();
    let matching: Vec<_> = analyzed
        .candidates
        .iter()
        .filter(|c| c.stem == "deva" && c.number == "du")
        .collect();
    assert!(
        matching.len() >= 3,
        "devābhyām should match inst/dat/abl du, got {} candidates",
        matching.len()
    );
}
