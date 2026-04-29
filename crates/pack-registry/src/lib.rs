//! RubyGems registry client.

use pack_core::{GemName, GemVersion, PackError, PackResult};
use serde::Deserialize;

pub struct Registry {
    client: reqwest::Client,
    base_url: String,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://rubygems.org".to_string(),
        }
    }

    pub async fn fetch_gem_versions(&self, name: &GemName) -> PackResult<Vec<GemVersion>> {
        let url = format!("{}/api/v1/versions/{}.json", self.base_url, name.0);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| PackError::Registry(e.to_string()))?;

        #[derive(Deserialize)]
        struct VersionRecord {
            number: String,
        }

        let versions: Vec<VersionRecord> = resp
            .json()
            .await
            .map_err(|e| PackError::Registry(e.to_string()))?;

        Ok(versions.into_iter().map(|v| GemVersion(v.number)).collect())
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
