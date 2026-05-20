use panini::engine::conjugation::{ConjugationInput, derive_conjugation};
use panini::engine::declension::{DeclensionInput, analyze_declension, derive_declension};
use panini::engine::sandhi::{SandhiInput, analyze_sandhi, derive_sandhi};
use panini::engine::TraceStep;
use panini::rule_cache::RuleCache;
use panini::vidya_client::VidyaClient;

fn build_cache() -> RuleCache {
    RuleCache::load_embedded()
}

#[tokio::test]
async fn derive_vowel_sandhi_guna() {
    let cache = build_cache();
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
    let cache = build_cache();
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
    let cache = build_cache();
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
    let cache = build_cache();
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
    let cache = build_cache();
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
    let cache = build_cache();
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
async fn derive_consonant_sandhi_palatalization() {
    let cache = build_cache();
    let rules = cache.get_rules("sandhi_rule");
    let cases = [
        ("tat", "ca", "tacca", "8.4.40"),
        ("tat", "jayati", "tajjayati", "8.4.40"),
        ("tat", "ṭīkā", "taṭṭīkā", "8.4.41"),
    ];
    for (first, second, expected, sutra) in cases {
        let result = derive_sandhi(
            rules,
            SandhiInput { first: first.into(), second: second.into() },
        ).unwrap();
        assert_eq!(
            result.output["result"], expected,
            "{first} + {second} should be {expected}"
        );
        assert_eq!(result.trace.len(), 1);
        assert_eq!(result.trace[0].rule_ref.as_deref(), Some(sutra));
    }
}

#[tokio::test]
async fn derive_consonant_sandhi_voicing() {
    let cache = build_cache();
    let rules = cache.get_rules("sandhi_rule");
    let cases = [
        ("tat", "gacchati", "tadgacchati", "8.4.53"),
        ("tat", "atra", "tadatra", "8.2.39"),
        ("vāk", "īśvaraḥ", "vāgīśvaraḥ", "8.2.39"),
        ("tat", "nayati", "tannayati", "8.4.45"),
    ];
    for (first, second, expected, sutra) in cases {
        let result = derive_sandhi(
            rules,
            SandhiInput { first: first.into(), second: second.into() },
        ).unwrap();
        assert_eq!(
            result.output["result"], expected,
            "{first} + {second} should be {expected}"
        );
        assert_eq!(
            result.trace[0].rule_ref.as_deref(), Some(sutra),
            "{first} + {second} should cite {sutra}"
        );
    }
}

#[tokio::test]
async fn derive_consonant_sandhi_anusvara() {
    let cache = build_cache();
    let rules = cache.get_rules("sandhi_rule");
    let result = derive_sandhi(
        rules,
        SandhiInput { first: "sam".into(), second: "kalpam".into() },
    ).unwrap();
    assert_eq!(result.output["result"], "saṃkalpam");
    assert_eq!(result.trace[0].rule_ref.as_deref(), Some("8.3.23"));
}

#[tokio::test]
async fn derive_no_false_sandhi() {
    let cache = build_cache();
    let rules = cache.get_rules("sandhi_rule");
    let result = derive_sandhi(
        rules,
        SandhiInput { first: "vāk".into(), second: "ca".into() },
    ).unwrap();
    assert_eq!(result.output["result"], "vākca");
    assert!(result.trace.is_empty(), "vāk + ca should have no sandhi");
}

#[tokio::test]
async fn analyze_consonant_sandhi_round_trip() {
    let cache = build_cache();
    let rules = cache.get_rules("sandhi_rule");
    let cases = [
        ("tat", "ca"),
        ("tat", "jayati"),
        ("tat", "ṭīkā"),
        ("tat", "gacchati"),
        ("tat", "nayati"),
        ("sam", "kalpam"),
        ("tat", "atra"),
        ("vāk", "īśvaraḥ"),
    ];
    for (first, second) in cases {
        let derived = derive_sandhi(
            rules,
            SandhiInput { first: first.into(), second: second.into() },
        ).unwrap();
        let combined = derived.output["result"].as_str().unwrap();
        let analyzed = analyze_sandhi(rules, combined).unwrap();
        let found = analyzed.candidates.iter().any(|c| c.first == first && c.second == second);
        assert!(
            found,
            "consonant round-trip failed: {} + {} → {}: candidates = {:#?}",
            first, second, combined, analyzed.candidates
        );
    }
}

#[tokio::test]
async fn analyze_consonant_ranking() {
    let cache = build_cache();
    let rules = cache.get_rules("sandhi_rule");
    let analyzed = analyze_sandhi(rules, "tacca").unwrap();
    let correct = analyzed.candidates.iter().find(|c| c.first == "tat" && c.second == "ca");
    assert!(
        correct.is_some(),
        "expected tat + ca in candidates for tacca: {:#?}",
        analyzed.candidates
    );
}

#[tokio::test]
async fn health_returns_rule_counts() {
    let cache = build_cache();
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
    let cache = build_cache();
    assert!(cache.rule_count("sup_suffix") > 0, "no sup_suffix rules");
    assert!(cache.rule_count("pratyaya_rule") > 0, "no pratyaya rules");
    assert!(cache.rule_count("anga_rule") > 0, "no anga rules");
    assert!(cache.rule_count("tripadi_rule") > 0, "no tripadi rules");
}

#[tokio::test]
async fn derive_deva_paradigm() {
    let cache = build_cache();
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
    let cache = build_cache();
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
    let cache = build_cache();
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
    let cache = build_cache();
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
    let cache = build_cache();
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

// --- Neuter a-stem tests ---

#[tokio::test]
async fn paradigm_phala_complete() {
    let cache = build_cache();
    let cells = generate_paradigm(&cache, "phala", "a-stem-n");
    assert_eq!(cells.len(), 24);

    let expected = vec![
        ("1", "sg", "phalam"),
        ("1", "du", "phale"),
        ("1", "pl", "phalāni"),
        ("2", "sg", "phalam"),
        ("2", "du", "phale"),
        ("2", "pl", "phalāni"),
        ("3", "sg", "phalena"),
        ("3", "du", "phalābhyām"),
        ("3", "pl", "phalaiḥ"),
        ("4", "sg", "phalāya"),
        ("4", "du", "phalābhyām"),
        ("4", "pl", "phalebhyaḥ"),
        ("5", "sg", "phalāt"),
        ("5", "du", "phalābhyām"),
        ("5", "pl", "phalebhyaḥ"),
        ("6", "sg", "phalasya"),
        ("6", "du", "phalayoḥ"),
        ("6", "pl", "phalānām"),
        ("7", "sg", "phale"),
        ("7", "du", "phalayoḥ"),
        ("7", "pl", "phaleṣu"),
        ("8", "sg", "phala"),
        ("8", "du", "phale"),
        ("8", "pl", "phalāni"),
    ];

    let mut errors = Vec::new();
    for (case, number, exp) in expected {
        let cell = cells.iter().find(|(c, n, _)| c == case && n == number).unwrap();
        match &cell.2 {
            Ok((form, trace)) => {
                if form != exp {
                    errors.push(format!("case={case} number={number}: got {form}, expected {exp}"));
                }
                if !trace.iter().any(|t| t.rule_ref.is_some()) {
                    errors.push(format!("case={case} number={number}: trace missing sūtra citations"));
                }
            }
            Err(e) => errors.push(format!("case={case} number={number}: {e}")),
        }
    }
    assert!(errors.is_empty(), "phala paradigm errors:\n{}", errors.join("\n"));
}

#[tokio::test]
async fn analyze_neuter_form() {
    let cache = build_cache();
    let analyzed = analyze_declension(
        cache.get_rules("sup_suffix"),
        cache.get_rules("pratyaya_rule"),
        cache.get_rules("anga_rule"),
        cache.get_rules("sandhi_rule"),
        cache.get_rules("tripadi_rule"),
        "phalam",
    )
    .unwrap();
    assert!(
        analyzed.candidates.iter().any(|c| c.stem == "phala" && c.stem_type == "a-stem-n"),
        "should find phalam as a-stem-n: {:?}",
        analyzed.candidates
    );
}

// --- Feminine ā-stem tests ---

#[tokio::test]
async fn paradigm_vidya_complete() {
    let cache = build_cache();
    let cells = generate_paradigm(&cache, "vidyā", "aa-stem-f");
    assert_eq!(cells.len(), 24);

    let expected = vec![
        ("1", "sg", "vidyā"),
        ("1", "du", "vidye"),
        ("1", "pl", "vidyāḥ"),
        ("2", "sg", "vidyām"),
        ("2", "du", "vidye"),
        ("2", "pl", "vidyāḥ"),
        ("3", "sg", "vidyayā"),
        ("3", "du", "vidyābhyām"),
        ("3", "pl", "vidyābhiḥ"),
        ("4", "sg", "vidyāyai"),
        ("4", "du", "vidyābhyām"),
        ("4", "pl", "vidyābhyaḥ"),
        ("5", "sg", "vidyāyāḥ"),
        ("5", "du", "vidyābhyām"),
        ("5", "pl", "vidyābhyaḥ"),
        ("6", "sg", "vidyāyāḥ"),
        ("6", "du", "vidyayoḥ"),
        ("6", "pl", "vidyānām"),
        ("7", "sg", "vidyāyām"),
        ("7", "du", "vidyayoḥ"),
        ("7", "pl", "vidyāsu"),
        ("8", "sg", "vidye"),
        ("8", "du", "vidye"),
        ("8", "pl", "vidyāḥ"),
    ];

    let mut errors = Vec::new();
    for (case, number, exp) in expected {
        let cell = cells.iter().find(|(c, n, _)| c == case && n == number).unwrap();
        match &cell.2 {
            Ok((form, trace)) => {
                if form != exp {
                    errors.push(format!("case={case} number={number}: got {form}, expected {exp}"));
                }
                if !trace.iter().any(|t| t.rule_ref.is_some()) {
                    errors.push(format!("case={case} number={number}: trace missing sūtra citations"));
                }
            }
            Err(e) => errors.push(format!("case={case} number={number}: {e}")),
        }
    }
    assert!(errors.is_empty(), "vidyā paradigm errors:\n{}", errors.join("\n"));
}

#[tokio::test]
async fn analyze_feminine_form() {
    let cache = build_cache();
    let analyzed = analyze_declension(
        cache.get_rules("sup_suffix"),
        cache.get_rules("pratyaya_rule"),
        cache.get_rules("anga_rule"),
        cache.get_rules("sandhi_rule"),
        cache.get_rules("tripadi_rule"),
        "vidyāyai",
    )
    .unwrap();
    assert!(
        analyzed.candidates.iter().any(|c| c.stem == "vidyā" && c.stem_type == "aa-stem-f"),
        "should find vidyāyai as aa-stem-f: {:?}",
        analyzed.candidates
    );
}

#[tokio::test]
async fn deva_paradigm_unchanged() {
    let cache = build_cache();
    let cells = generate_paradigm(&cache, "deva", "a-stem-m");
    let expected = vec![
        ("1", "sg", "devaḥ"), ("1", "du", "devau"), ("1", "pl", "devāḥ"),
        ("2", "sg", "devam"), ("2", "du", "devau"), ("2", "pl", "devān"),
        ("3", "sg", "devena"), ("3", "du", "devābhyām"), ("3", "pl", "devaiḥ"),
        ("4", "sg", "devāya"), ("4", "du", "devābhyām"), ("4", "pl", "devebhyaḥ"),
        ("5", "sg", "devāt"), ("5", "du", "devābhyām"), ("5", "pl", "devebhyaḥ"),
        ("6", "sg", "devasya"), ("6", "du", "devayoḥ"), ("6", "pl", "devānām"),
        ("7", "sg", "deve"), ("7", "du", "devayoḥ"), ("7", "pl", "deveṣu"),
        ("8", "sg", "deva"), ("8", "du", "devau"), ("8", "pl", "devāḥ"),
    ];
    for (case, number, exp) in expected {
        let cell = cells.iter().find(|(c, n, _)| c == case && n == number).unwrap();
        let form = cell.2.as_ref().unwrap().0.as_str();
        assert_eq!(form, exp, "regression: case={case} number={number}");
    }
}

// --- Conjugation tests ---

fn derive_conj(cache: &RuleCache, dhatu: &str, purusha: &str, vacana: &str) -> String {
    derive_conj_gana(cache, dhatu, "1", purusha, vacana)
}

fn derive_conj_gana(
    cache: &RuleCache,
    dhatu: &str,
    gana: &str,
    purusha: &str,
    vacana: &str,
) -> String {
    let result = derive_conjugation(
        cache.get_rules("tin_suffix"),
        cache.get_rules("vikarana_rule"),
        cache.get_rules("verb_anga_rule"),
        cache.get_rules("tripadi_rule"),
        ConjugationInput {
            dhatu: dhatu.into(),
            gana: gana.into(),
            lakara: "laṭ".into(),
            pada: "parasmaipada".into(),
            purusha: purusha.into(),
            vacana: vacana.into(),
        },
    )
    .unwrap();
    result.output["form"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn conjugation_bhuu_full_paradigm() {
    let cache = build_cache();
    let expected = [
        ("prathama", "ekavacana", "bhavati"),
        ("prathama", "dvivacana", "bhavataḥ"),
        ("prathama", "bahuvacana", "bhavanti"),
        ("madhyama", "ekavacana", "bhavasi"),
        ("madhyama", "dvivacana", "bhavathaḥ"),
        ("madhyama", "bahuvacana", "bhavatha"),
        ("uttama", "ekavacana", "bhavāmi"),
        ("uttama", "dvivacana", "bhavāvaḥ"),
        ("uttama", "bahuvacana", "bhavāmaḥ"),
    ];
    for (purusha, vacana, exp) in expected {
        let form = derive_conj(&cache, "bhū", purusha, vacana);
        assert_eq!(form, exp, "√bhū {purusha} {vacana} expected={exp}");
    }
}

#[tokio::test]
async fn conjugation_nii_spot_check() {
    let cache = build_cache();
    assert_eq!(derive_conj(&cache, "nī", "prathama", "ekavacana"), "nayati");
    assert_eq!(derive_conj(&cache, "nī", "uttama", "ekavacana"), "nayāmi");
}

#[tokio::test]
async fn conjugation_budh_medial_guna() {
    let cache = build_cache();
    assert_eq!(
        derive_conj(&cache, "budh", "prathama", "ekavacana"),
        "bodhati"
    );
}

#[tokio::test]
async fn conjugation_path_no_guna() {
    let cache = build_cache();
    assert_eq!(
        derive_conj(&cache, "paṭh", "prathama", "ekavacana"),
        "paṭhati"
    );
}

#[tokio::test]
async fn conjugation_ji() {
    let cache = build_cache();
    assert_eq!(
        derive_conj(&cache, "ji", "prathama", "ekavacana"),
        "jayati"
    );
}

#[tokio::test]
async fn conjugation_trace_shows_sutra_refs() {
    let cache = build_cache();
    let result = derive_conjugation(
        cache.get_rules("tin_suffix"),
        cache.get_rules("vikarana_rule"),
        cache.get_rules("verb_anga_rule"),
        cache.get_rules("tripadi_rule"),
        ConjugationInput {
            dhatu: "bhū".into(),
            gana: "1".into(),
            lakara: "laṭ".into(),
            pada: "parasmaipada".into(),
            purusha: "prathama".into(),
            vacana: "ekavacana".into(),
        },
    )
    .unwrap();
    let refs: Vec<_> = result
        .trace
        .iter()
        .filter_map(|t| t.rule_ref.as_deref())
        .collect();
    assert!(refs.contains(&"3.4.78"), "missing tiṅ sūtra");
    assert!(refs.contains(&"3.1.68"), "missing vikaraṇa sūtra");
    assert!(refs.contains(&"7.3.84"), "missing guṇa sūtra");
    assert!(refs.contains(&"6.1.78"), "missing semivowel sūtra");
}

#[tokio::test]
async fn conjugation_tripadi_derives_visarga() {
    let cache = build_cache();
    let result = derive_conjugation(
        cache.get_rules("tin_suffix"),
        cache.get_rules("vikarana_rule"),
        cache.get_rules("verb_anga_rule"),
        cache.get_rules("tripadi_rule"),
        ConjugationInput {
            dhatu: "bhū".into(),
            gana: "1".into(),
            lakara: "laṭ".into(),
            pada: "parasmaipada".into(),
            purusha: "prathama".into(),
            vacana: "dvivacana".into(),
        },
    )
    .unwrap();
    assert_eq!(result.output["form"], "bhavataḥ");
    let refs: Vec<_> = result
        .trace
        .iter()
        .filter_map(|t| t.rule_ref.as_deref())
        .collect();
    assert!(refs.contains(&"8.2.66"), "missing s→r tripadi rule");
    assert!(refs.contains(&"8.3.15"), "missing r→ḥ tripadi rule");
}

#[tokio::test]
async fn conjugation_cache_loads_templates() {
    let cache = build_cache();
    assert!(cache.rule_count("tin_suffix") > 0, "no tin_suffix rules");
    assert!(
        cache.rule_count("vikarana_rule") > 0,
        "no vikarana rules"
    );
    assert!(
        cache.rule_count("verb_anga_rule") > 0,
        "no verb_anga rules"
    );
}

#[tokio::test]
async fn conjugation_medial_guna_skips_final_u() {
    let cache = build_cache();
    // "śru" — u is final, not medial. The medial guṇa rule should not fire.
    let form = derive_conj(&cache, "śru", "prathama", "ekavacana");
    assert_ne!(form, "śroati", "medial guṇa should not apply to final u");
}

#[tokio::test]
async fn conjugation_vikarana_rejects_ardhadhatuka() {
    let cache = build_cache();
    let result = derive_conjugation(
        cache.get_rules("tin_suffix"),
        cache.get_rules("vikarana_rule"),
        cache.get_rules("verb_anga_rule"),
        cache.get_rules("tripadi_rule"),
        ConjugationInput {
            dhatu: "bhū".into(),
            gana: "1".into(),
            lakara: "liṭ".into(),
            pada: "parasmaipada".into(),
            purusha: "prathama".into(),
            vacana: "ekavacana".into(),
        },
    );
    assert!(result.is_err(), "liṭ should not find sārvadhātuka vikaraṇa");
}

// --- Gaṇa 4 (divādi/śyan) ---

#[tokio::test]
async fn conjugation_gana4_div() {
    let cache = build_cache();
    assert_eq!(derive_conj_gana(&cache, "div", "4", "prathama", "ekavacana"), "divyati");
    assert_eq!(derive_conj_gana(&cache, "div", "4", "prathama", "dvivacana"), "divyataḥ");
    assert_eq!(derive_conj_gana(&cache, "div", "4", "prathama", "bahuvacana"), "divyanti");
}

#[tokio::test]
async fn conjugation_gana4_nrt_no_guna() {
    let cache = build_cache();
    assert_eq!(
        derive_conj_gana(&cache, "nṛt", "4", "prathama", "ekavacana"),
        "nṛtyati",
        "gaṇa 4 ṅit should block guṇa on ṛ"
    );
}

// --- Gaṇa 6 (tudādi/śa) ---

#[tokio::test]
async fn conjugation_gana6_tud() {
    let cache = build_cache();
    assert_eq!(
        derive_conj_gana(&cache, "tud", "6", "prathama", "ekavacana"),
        "tudati",
        "gaṇa 6 ṅit should block medial guṇa"
    );
}

#[tokio::test]
async fn conjugation_gana6_vis() {
    let cache = build_cache();
    assert_eq!(
        derive_conj_gana(&cache, "viś", "6", "prathama", "ekavacana"),
        "viśati",
        "gaṇa 6 ṅit should block medial guṇa on viś"
    );
}

// --- Gaṇa 10 (curādi/ṇic) ---

#[tokio::test]
async fn conjugation_gana10_cur_guna() {
    let cache = build_cache();
    assert_eq!(
        derive_conj_gana(&cache, "cur", "10", "prathama", "ekavacana"),
        "corayati",
        "gaṇa 10 laghu upadha should get guṇa"
    );
}

#[tokio::test]
async fn conjugation_gana10_cint_no_guna() {
    let cache = build_cache();
    assert_eq!(
        derive_conj_gana(&cache, "cint", "10", "prathama", "ekavacana"),
        "cintayati",
        "gaṇa 10 consonant upadha — no guṇa or vṛddhi"
    );
}

// --- Gaṇa 5 (svādi/śnu) ---

#[tokio::test]
async fn conjugation_gana5_su() {
    let cache = build_cache();
    assert_eq!(derive_conj_gana(&cache, "su", "5", "prathama", "ekavacana"), "sunoti");
    assert_eq!(derive_conj_gana(&cache, "su", "5", "prathama", "dvivacana"), "sunutaḥ");
    assert_eq!(derive_conj_gana(&cache, "su", "5", "prathama", "bahuvacana"), "sunvanti");
}

// --- Gaṇa 8 (tanādi/u) ---

#[tokio::test]
async fn conjugation_gana8_tan() {
    let cache = build_cache();
    assert_eq!(derive_conj_gana(&cache, "tan", "8", "prathama", "ekavacana"), "tanoti");
    assert_eq!(derive_conj_gana(&cache, "tan", "8", "prathama", "dvivacana"), "tanutaḥ");
    assert_eq!(derive_conj_gana(&cache, "tan", "8", "prathama", "bahuvacana"), "tanvanti");
}

#[tokio::test]
async fn conjugation_gana8_kr() {
    let cache = build_cache();
    // √kṛ ekavacana: upadha ṛ gets guṇa (→ ar) before pit tiṅ, aṅga-final u → o
    assert_eq!(derive_conj_gana(&cache, "kṛ", "8", "prathama", "ekavacana"), "karoti");
    // √kṛ dvivacana: tiṅ taḥ is NOT pit → no upadha guṇa, no aṅga-final guṇa
    assert_eq!(derive_conj_gana(&cache, "kṛ", "8", "prathama", "dvivacana"), "kṛutaḥ");
}

// --- Gaṇa 9 (kryādi/śnā) ---

#[tokio::test]
async fn conjugation_gana9_krii() {
    let cache = build_cache();
    // pit tiṅ → nā; ṇatva: n→ṇ after ī
    assert_eq!(derive_conj_gana(&cache, "krī", "9", "prathama", "ekavacana"), "krīṇāti");
    // non-pit consonant-initial → nī; ṇatva
    assert_eq!(derive_conj_gana(&cache, "krī", "9", "prathama", "dvivacana"), "krīṇītaḥ");
    // non-pit vowel-initial → n; ṇatva
    assert_eq!(derive_conj_gana(&cache, "krī", "9", "prathama", "bahuvacana"), "krīṇanti");
}

// --- Gaṇa 2 (adādi/luk) ---

#[tokio::test]
async fn conjugation_gana2_as() {
    let cache = build_cache();
    assert_eq!(derive_conj_gana(&cache, "as", "2", "prathama", "ekavacana"), "asti");
}

// --- Gaṇa 7 (rudhādi/śnam infix) ---

#[tokio::test]
async fn conjugation_gana7_bhid() {
    let cache = build_cache();
    assert_eq!(derive_conj_gana(&cache, "bhid", "7", "prathama", "ekavacana"), "bhinatti");
    assert_eq!(derive_conj_gana(&cache, "bhid", "7", "prathama", "bahuvacana"), "bhindanti");
}

#[tokio::test]
async fn conjugation_gana7_rudh() {
    let cache = build_cache();
    assert_eq!(derive_conj_gana(&cache, "rudh", "7", "prathama", "ekavacana"), "ruṇaddhi");
    assert_eq!(derive_conj_gana(&cache, "rudh", "7", "prathama", "dvivacana"), "ruṇddhaḥ");
    assert_eq!(derive_conj_gana(&cache, "rudh", "7", "prathama", "bahuvacana"), "ruṇdhanti");
}

#[tokio::test]
async fn conjugation_gana2_ad() {
    let cache = build_cache();
    assert_eq!(derive_conj_gana(&cache, "ad", "2", "prathama", "ekavacana"), "atti");
    assert_eq!(derive_conj_gana(&cache, "ad", "2", "prathama", "dvivacana"), "attaḥ");
    assert_eq!(derive_conj_gana(&cache, "ad", "2", "prathama", "bahuvacana"), "adanti");
}

// --- Gaṇa 3 (juhotyādi/ślu + reduplication) ---

#[tokio::test]
async fn conjugation_gana3_hu() {
    let cache = build_cache();
    assert_eq!(derive_conj_gana(&cache, "hu", "3", "prathama", "ekavacana"), "juhoti");
    assert_eq!(derive_conj_gana(&cache, "hu", "3", "prathama", "dvivacana"), "juhutaḥ");
    assert_eq!(derive_conj_gana(&cache, "hu", "3", "prathama", "bahuvacana"), "juhvati");
}

#[tokio::test]
async fn conjugation_gana3_dhaa() {
    let cache = build_cache();
    assert_eq!(derive_conj_gana(&cache, "dhā", "3", "prathama", "ekavacana"), "dadhāti");
    assert_eq!(derive_conj_gana(&cache, "dhā", "3", "prathama", "dvivacana"), "dhattaḥ");
    assert_eq!(derive_conj_gana(&cache, "dhā", "3", "prathama", "bahuvacana"), "dadhati");
}

#[tokio::test]
async fn conjugation_gana3_bhii() {
    let cache = build_cache();
    assert_eq!(derive_conj_gana(&cache, "bhī", "3", "prathama", "ekavacana"), "bibheti");
    assert_eq!(derive_conj_gana(&cache, "bhī", "3", "prathama", "dvivacana"), "bibhītaḥ");
    assert_eq!(derive_conj_gana(&cache, "bhī", "3", "prathama", "bahuvacana"), "bibhyati");
}

// --- Consonant junction coverage (madhyama/uttama) ---

#[tokio::test]
async fn conjugation_gana2_ad_madhyama() {
    let cache = build_cache();
    assert_eq!(derive_conj_gana(&cache, "ad", "2", "madhyama", "ekavacana"), "atsi");
    assert_eq!(derive_conj_gana(&cache, "ad", "2", "madhyama", "dvivacana"), "atthaḥ");
    assert_eq!(derive_conj_gana(&cache, "ad", "2", "madhyama", "bahuvacana"), "attha");
}

#[tokio::test]
async fn conjugation_gana2_ad_uttama() {
    let cache = build_cache();
    assert_eq!(derive_conj_gana(&cache, "ad", "2", "uttama", "ekavacana"), "admi");
    assert_eq!(derive_conj_gana(&cache, "ad", "2", "uttama", "dvivacana"), "advaḥ");
    assert_eq!(derive_conj_gana(&cache, "ad", "2", "uttama", "bahuvacana"), "admaḥ");
}

#[tokio::test]
async fn conjugation_gana7_rudh_madhyama() {
    let cache = build_cache();
    assert_eq!(derive_conj_gana(&cache, "rudh", "7", "madhyama", "ekavacana"), "ruṇatsi");
    assert_eq!(derive_conj_gana(&cache, "rudh", "7", "madhyama", "dvivacana"), "ruṇtthaḥ");
    assert_eq!(derive_conj_gana(&cache, "rudh", "7", "madhyama", "bahuvacana"), "ruṇttha");
}

#[tokio::test]
async fn conjugation_gana7_rudh_uttama() {
    let cache = build_cache();
    assert_eq!(derive_conj_gana(&cache, "rudh", "7", "uttama", "ekavacana"), "ruṇadhmi");
    assert_eq!(derive_conj_gana(&cache, "rudh", "7", "uttama", "dvivacana"), "ruṇdhvaḥ");
    assert_eq!(derive_conj_gana(&cache, "rudh", "7", "uttama", "bahuvacana"), "ruṇdhmaḥ");
}
