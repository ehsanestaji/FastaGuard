use anyhow::{anyhow, Context, Result};
use serde_json::Value;

pub const FASTAGUARD_SCHEMA_JSON: &str = include_str!("../schema/fastaguard.schema.json");
pub const FINDING_CATALOG_JSON: &str = include_str!("../schema/finding-catalog.json");

pub fn schema_json() -> &'static str {
    FASTAGUARD_SCHEMA_JSON
}

pub fn finding_catalog_json() -> &'static str {
    FINDING_CATALOG_JSON
}

pub fn explain_finding_json(id: &str) -> Result<String> {
    let catalog: Value =
        serde_json::from_str(FINDING_CATALOG_JSON).context("failed to parse finding catalog")?;
    let findings = catalog
        .get("findings")
        .and_then(Value::as_array)
        .context("finding catalog is missing findings array")?;
    let finding = findings
        .iter()
        .find(|finding| finding.get("id").and_then(Value::as_str) == Some(id))
        .ok_or_else(|| anyhow!("unknown finding id '{id}'"))?;

    serde_json::to_string_pretty(finding).context("failed to serialize finding catalog entry")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_schema_is_valid_json() {
        let schema: Value = serde_json::from_str(schema_json()).unwrap();

        assert_eq!(
            schema.get("title").and_then(Value::as_str),
            Some("FastaguardReport")
        );
    }

    #[test]
    fn bundled_catalog_explains_known_finding() {
        let finding = explain_finding_json("high_n_rate").unwrap();

        assert!(finding.contains(r#""id": "high_n_rate""#), "{finding}");
        assert!(finding.contains(r#""recommended_next_tools""#), "{finding}");
    }

    #[test]
    fn bundled_catalog_rejects_unknown_finding() {
        let error = explain_finding_json("not_a_rule").unwrap_err();

        assert_eq!(error.to_string(), "unknown finding id 'not_a_rule'");
    }
}
