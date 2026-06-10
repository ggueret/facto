use chrono::NaiveDate;
use serde::Deserialize;

use crate::models::{EolStatus, RuntimeVersion};
use crate::runtimes::RuntimeResult;

/// Raw response from endoflife.date API
#[derive(Debug, Deserialize)]
struct EolCycle {
    cycle: String,
    latest: String,
    #[serde(rename = "releaseDate")]
    release_date: Option<NaiveDate>,
    #[serde(rename = "latestReleaseDate")]
    latest_release_date: Option<NaiveDate>,
    eol: EolStatus,
    #[serde(default)]
    lts: LtsValue,
    support: Option<EolStatus>,
}

/// endoflife.date returns `lts` as either a bool or a date string.
/// We only care about the bool meaning (is it LTS or not).
#[derive(Debug, Deserialize, Default)]
#[serde(untagged)]
enum LtsValue {
    Bool(bool),
    Date(#[allow(dead_code)] String),
    #[default]
    Missing,
}

impl LtsValue {
    fn is_lts(&self) -> bool {
        match self {
            LtsValue::Bool(b) => *b,
            LtsValue::Date(_) => true,
            LtsValue::Missing => false,
        }
    }
}

pub struct EndOfLifeClient {
    client: reqwest::Client,
}

impl EndOfLifeClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Fetch all release cycles for a product from endoflife.date
    pub async fn fetch_cycles(&self, product_id: &str) -> RuntimeResult<Vec<RuntimeVersion>> {
        let url = format!("https://endoflife.date/api/{}.json", product_id);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(crate::runtimes::RuntimeError::Parse(format!(
                "endoflife.date returned {} for {}",
                resp.status(),
                product_id
            )));
        }

        let cycles: Vec<EolCycle> = crate::runtimes::bounded_json(resp).await?;

        Ok(cycles
            .into_iter()
            .map(|c| RuntimeVersion {
                cycle: c.cycle,
                latest: c.latest,
                release_date: c.release_date,
                latest_release_date: c.latest_release_date,
                eol: c.eol,
                lts: c.lts.is_lts(),
                support: c.support,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_eol_as_date() {
        let json = r#"{"cycle":"3.14","latest":"3.14.3","releaseDate":"2025-10-07","latestReleaseDate":"2026-02-03","eol":"2030-10-31","lts":false,"support":"2027-10-01"}"#;
        let cycle: EolCycle = serde_json::from_str(json).unwrap();
        assert_eq!(cycle.cycle, "3.14");
        assert!(!cycle.lts.is_lts());
        match cycle.eol {
            EolStatus::Date(d) => assert_eq!(d, NaiveDate::from_ymd_opt(2030, 10, 31).unwrap()),
            _ => panic!("expected Date variant"),
        }
    }

    #[test]
    fn test_deserialize_eol_as_bool() {
        let json = r#"{"cycle":"3.14","latest":"3.14.3","eol":false,"lts":false}"#;
        let cycle: EolCycle = serde_json::from_str(json).unwrap();
        match cycle.eol {
            EolStatus::Bool(b) => assert!(!b),
            _ => panic!("expected Bool variant"),
        }
    }

    #[test]
    fn test_deserialize_lts_as_date() {
        let json = r#"{"cycle":"20","latest":"20.18.2","eol":"2026-04-30","lts":"2023-10-24"}"#;
        let cycle: EolCycle = serde_json::from_str(json).unwrap();
        assert!(cycle.lts.is_lts());
    }

    #[test]
    fn test_deserialize_missing_support() {
        let json = r#"{"cycle":"1.80","latest":"1.80.1","eol":false,"lts":false}"#;
        let cycle: EolCycle = serde_json::from_str(json).unwrap();
        assert!(cycle.support.is_none());
    }
}
