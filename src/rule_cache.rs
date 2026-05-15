use std::collections::HashMap;

use crate::vidya_client::RawClaim;

pub struct CachedRule {
    pub params: serde_json::Value,
    pub statement: String,
}

pub struct RuleCache {
    rules: HashMap<String, Vec<CachedRule>>,
}

impl RuleCache {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
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
}
