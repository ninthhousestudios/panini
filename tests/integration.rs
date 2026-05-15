use panini::config::Config;
use panini::engine::sandhi::{SandhiInput, analyze_sandhi, derive_sandhi};
use panini::rule_cache::RuleCache;
use panini::vidya_client::VidyaClient;

async fn build_cache() -> RuleCache {
    dotenvy::dotenv().ok();
    let cfg = Config::from_env();
    let vidya = VidyaClient::connect(&cfg.vidya_url, cfg.vidya_auth_token.as_deref())
        .await
        .expect("failed to connect to vidya — is it running?");

    let mut cache = RuleCache::new();
    let claims = vidya
        .fetch_claims("vyakarana", "sandhi_rule")
        .await
        .expect("failed to fetch sandhi rules");
    assert!(claims.len() > 0, "no sandhi rules loaded from vidya");
    cache.load_template("sandhi_rule".into(), claims);
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
