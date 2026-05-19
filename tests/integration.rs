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

// --- Conjugation tests ---

fn derive_conj(cache: &RuleCache, dhatu: &str, purusha: &str, vacana: &str) -> String {
    let result = derive_conjugation(
        cache.get_rules("tin_suffix"),
        cache.get_rules("vikarana_rule"),
        cache.get_rules("verb_anga_rule"),
        cache.get_rules("tripadi_rule"),
        ConjugationInput {
            dhatu: dhatu.into(),
            gana: "1".into(),
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
