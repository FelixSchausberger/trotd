use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

use crate::http::HttpClient;
use crate::model::{LanguageFilter, Provider, ProviderCfg, Repo};

/// GitHub provider using trending API
pub struct GitHub {
    http: HttpClient,
}

#[derive(Debug, Deserialize)]
struct TrendingRepo {
    author: String,
    name: String,
    description: Option<String>,
    language: Option<String>,
    #[serde(rename = "currentPeriodStars")]
    current_period_stars: Option<u64>,
    stars: Option<u64>,
    url: String,
}

impl GitHub {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        Ok(Self {
            http: HttpClient::new(timeout_secs)?,
        })
    }

    /// Fetch trending repositories from GitHub
    async fn fetch_trending(
        &self,
        language: Option<&str>,
        token: Option<&str>,
    ) -> Result<Vec<TrendingRepo>> {
        let url = if let Some(lang) = language {
            format!("https://gh-trending-api.vicary.workers.dev/repositories?since=daily&language={lang}")
        } else {
            "https://gh-trending-api.vicary.workers.dev/repositories?since=daily".to_string()
        };

        self.http.get_json(&url, token).await
    }
}

#[async_trait]
impl Provider for GitHub {
    fn id(&self) -> &'static str {
        "github"
    }

    fn icon(&self) -> &'static str {
        "[GH]"
    }

    async fn top_today(
        &self,
        cfg: &ProviderCfg,
        limit: usize,
        langs: &LanguageFilter,
    ) -> Result<Vec<Repo>> {
        // If language filter is specified, try to fetch for first language
        let trending = if langs.languages.is_empty() {
            self.fetch_trending(None, cfg.token.as_deref()).await?
        } else {
            // Try fetching for each language filter and combine results
            let mut all_repos = Vec::new();
            for lang in &langs.languages {
                if let Ok(repos) = self.fetch_trending(Some(lang), cfg.token.as_deref()).await {
                    all_repos.extend(repos);
                }
            }
            all_repos
        };

        let repos = trending
            .into_iter()
            .filter(|r| langs.matches(r.language.as_ref()))
            .take(limit)
            .map(|r| Repo {
                provider: self.id().to_string(),
                icon: self.icon().to_string(),
                name: format!("{}/{}", r.author, r.name),
                language: r.language,
                description: r.description,
                url: r.url,
                stars_today: r.current_period_stars,
                stars_total: r.stars,
                approximated: false,
            })
            .collect();

        Ok(repos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_provider_metadata() {
        let github = GitHub::new(6).unwrap();
        assert_eq!(github.id(), "github");
        assert_eq!(github.icon(), "[GH]");
    }

    #[tokio::test]
    async fn test_github_trending_api() {
        // This is an integration test that requires network access
        // In CI, this might be mocked with mockito
        let github = GitHub::new(10).unwrap();
        let cfg = ProviderCfg {
            timeout_secs: 10,
            token: None,
            base_url: None,
        };
        let filter = LanguageFilter::new(vec![]);

        // Try to fetch, but don't fail the test if API is down
        match github.top_today(&cfg, 3, &filter).await {
            Ok(repos) => {
                // Verify structure if API call succeeds
                for repo in repos {
                    assert_eq!(repo.provider, "github");
                    assert_eq!(repo.icon, "[GH]");
                    assert!(!repo.name.is_empty());
                }
            }
            Err(e) => {
                // API might be down or rate-limited, that's okay for this test
                eprintln!("GitHub API test skipped: {e}");
            }
        }
    }
}
