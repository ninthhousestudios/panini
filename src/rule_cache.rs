use std::collections::HashMap;

use serde::Deserialize;

use crate::vidya_client::RawClaim;

pub struct CachedRule {
    pub params: serde_json::Value,
    pub statement: String,
}

pub struct RuleCache {
    rules: HashMap<String, Vec<CachedRule>>,
}

const EMBEDDED_TEMPLATES: &[(&str, &str)] = &[
    ("sandhi_rule", include_str!("../data/sandhi-rule.json")),
    ("sup_suffix", include_str!("../data/sup-suffix.json")),
    ("pratyaya_rule", include_str!("../data/pratyaya-rule.json")),
    ("anga_rule", include_str!("../data/anga-rule.json")),
    ("tripadi_rule", include_str!("../data/tripadi-rule.json")),
    ("tin_suffix", include_str!("../data/tin-suffix.json")),
    ("vikarana_rule", include_str!("../data/vikarana-rule.json")),
    ("verb_anga_rule", include_str!("../data/verb-anga-rule.json")),
];

#[derive(Deserialize)]
struct EmbeddedRule {
    params: serde_json::Value,
    statement: String,
}

impl RuleCache {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    pub fn load_embedded() -> Self {
        let mut cache = Self::new();
        for &(slug, json) in EMBEDDED_TEMPLATES {
            let embedded: Vec<EmbeddedRule> =
                serde_json::from_str(json).expect("embedded rule JSON is valid");
            let rules = embedded
                .into_iter()
                .map(|e| CachedRule {
                    params: e.params,
                    statement: e.statement,
                })
                .collect();
            cache.rules.insert(slug.to_string(), rules);
        }
        cache
    }

    pub fn load_template(&mut self, template_slug: String, raw_claims: Vec<RawClaim>) {
        let rules = raw_claims
            .into_iter()
            .map(|c| CachedRule {
                params: c.params,
                statement: c.statement,
            })
            .collect();
        self.rules.insert(template_slug, rules);
    }

    pub fn get_rules(&self, template_slug: &str) -> &[CachedRule] {
        self.rules
            .get(template_slug)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn template_count(&self) -> usize {
        self.rules.len()
    }

    pub fn rule_count(&self, template_slug: &str) -> usize {
        self.get_rules(template_slug).len()
    }

    pub fn total_rules(&self) -> usize {
        self.rules.values().map(|v| v.len()).sum()
    }

    pub fn all_templates(&self) -> impl Iterator<Item = (&str, &[CachedRule])> {
        self.rules.iter().map(|(k, v)| (k.as_str(), v.as_slice()))
    }
}
