use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

use crate::http::HttpClient;
use crate::model::{LanguageFilter, Provider, ProviderCfg, Repo};

/// GitLab provider using explore API
pub struct GitLab {
    http: HttpClient,
}

#[derive(Debug, Deserialize)]
struct GitLabProject {
    #[allow(dead_code)]
    name: String,
    path_with_namespace: String,
    description: Option<String>,
    star_count: Option<u64>,
    web_url: String,
    #[serde(default)]
    topics: Vec<String>,
}

impl GitLab {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        Ok(Self {
            http: HttpClient::new(timeout_secs)?,
        })
    }

    /// Fetch recently created projects from GitLab
    async fn fetch_projects(&self, token: Option<&str>) -> Result<Vec<GitLabProject>> {
        // Get today's date in ISO format
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        // Search for projects created today, sorted by stars
        let url = format!(
            "https://gitlab.com/api/v4/projects?order_by=created_at&sort=desc&created_after={today}&per_page=50"
        );

        self.http.get_json(&url, token).await
    }

    /// Extract language from topics (GitLab uses topics, not a dedicated language field)
    fn extract_language(topics: &[String]) -> Option<String> {
        // Common programming language tags
        let languages = [
            "rust",
            "go",
            "python",
            "javascript",
            "typescript",
            "java",
            "c",
            "cpp",
            "csharp",
            "ruby",
            "php",
            "swift",
            "kotlin",
            "scala",
        ];

        topics
            .iter()
            .find(|topic| {
                let lower = topic.to_lowercase();
                languages.iter().any(|&lang| lower.contains(lang))
            })
            .map(|s| {
                // Capitalize first letter
                let mut chars = s.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            })
    }
}

#[async_trait]
impl Provider for GitLab {
    fn id(&self) -> &'static str {
        "gitlab"
    }

    fn icon(&self) -> &'static str {
        "[GL]"
    }

    async fn top_today(
        &self,
        cfg: &ProviderCfg,
        limit: usize,
        langs: &LanguageFilter,
    ) -> Result<Vec<Repo>> {
        let projects = self.fetch_projects(cfg.token.as_deref()).await?;

        let repos = projects
            .into_iter()
            .map(|p| {
                let language = Self::extract_language(&p.topics);
                (p, language)
            })
            .filter(|(_, lang)| langs.matches(lang.as_ref()))
            .take(limit)
            .map(|(p, language)| Repo {
                provider: self.id().to_string(),
                icon: self.icon().to_string(),
                name: p.path_with_namespace,
                language,
                description: p.description,
                url: p.web_url,
                stars_today: None, // GitLab API doesn't provide daily stars
                stars_total: p.star_count,
                approximated: true,
            })
            .collect();

        Ok(repos)
    }
}

// Add chrono dependency for date handling
// Note: We need to add chrono to Cargo.toml

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gitlab_provider_metadata() {
        let gitlab = GitLab::new(6).unwrap();
        assert_eq!(gitlab.id(), "gitlab");
        assert_eq!(gitlab.icon(), "[GL]");
    }

    #[test]
    fn test_extract_language() {
        assert_eq!(
            GitLab::extract_language(&["rust".to_string(), "cli".to_string()]),
            Some("Rust".to_string())
        );
        assert_eq!(
            GitLab::extract_language(&["web".to_string(), "python".to_string()]),
            Some("Python".to_string())
        );
        assert_eq!(GitLab::extract_language(&["web".to_string()]), None);
    }

    #[tokio::test]
    async fn test_gitlab_api() {
        let gitlab = GitLab::new(10).unwrap();
        let cfg = ProviderCfg {
            timeout_secs: 10,
            token: None,
            base_url: None,
        };
        let filter = LanguageFilter::new(vec![]);

        // Try to fetch, but don't fail the test if API is down
        match gitlab.top_today(&cfg, 3, &filter).await {
            Ok(repos) => {
                // Verify structure if API call succeeds
                for repo in repos {
                    assert_eq!(repo.provider, "gitlab");
                    assert_eq!(repo.icon, "[GL]");
                    assert!(repo.approximated);
                }
            }
            Err(e) => {
                eprintln!("GitLab API test skipped: {e}");
            }
        }
    }
}
