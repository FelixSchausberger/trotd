use anyhow::{Context, Result};
use clap::Parser;
use futures::stream::{FuturesUnordered, StreamExt};
use std::sync::Arc;

mod cache;
mod config;
mod http;
mod model;
mod providers;
mod render;

use cache::Cache;
use config::Config;
use model::{LanguageFilter, Provider, ProviderCfg};
use providers::{GitHub, GitLab, Gitea};
use render::{render, OutputFormat};

/// Trending repositories of the day - minimal MOTD CLI
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Maximum repositories per provider
    #[arg(short = 'n', long = "max", value_name = "N")]
    max_per_provider: Option<usize>,

    /// Enable specific providers (comma-separated: gh,gl,ge)
    #[arg(short, long, value_name = "LIST", value_delimiter = ',')]
    provider: Option<Vec<String>>,

    /// Filter by language (comma-separated: rust,go)
    #[arg(short, long, value_name = "LIST", value_delimiter = ',')]
    lang: Option<Vec<String>>,

    /// Disable cache
    #[arg(long)]
    no_cache: bool,

    /// Output as JSON instead of MOTD
    #[arg(long)]
    json: bool,
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Load configuration
    let mut config = Config::load().context("Failed to load configuration")?;

    // Apply CLI overrides
    if let Some(max) = args.max_per_provider {
        config.general.max_per_provider = max;
    }

    if let Some(langs) = args.lang {
        config.general.language_filter = langs;
    }

    // Determine output format
    let format = if args.json {
        OutputFormat::Json
    } else {
        OutputFormat::Motd
    };

    // Initialize cache
    let cache = if args.no_cache {
        None
    } else {
        Some(Cache::new(config.general.cache_ttl_mins).context("Failed to initialize cache")?)
    };

    // Determine enabled providers
    let enabled_providers = if let Some(ref providers) = args.provider {
        // Parse short names: gh -> github, gl -> gitlab, ge -> gitea
        providers
            .iter()
            .map(|p| match p.as_str() {
                "gh" => "github",
                "gl" => "gitlab",
                "ge" => "gitea",
                _ => p.as_str(),
            })
            .collect::<Vec<_>>()
    } else {
        config.enabled_providers()
    };

    // Build provider instances
    let mut provider_instances: Vec<(String, Box<dyn Provider>)> = Vec::new();

    for provider_id in enabled_providers {
        match provider_id {
            "github" => {
                if let Ok(gh) = GitHub::new(config.general.timeout_secs) {
                    provider_instances.push(("github".to_string(), Box::new(gh)));
                }
            }
            "gitlab" => {
                if let Ok(gl) = GitLab::new(config.general.timeout_secs) {
                    provider_instances.push(("gitlab".to_string(), Box::new(gl)));
                }
            }
            "gitea" => {
                if let Ok(ge) = Gitea::new(config.general.timeout_secs) {
                    provider_instances.push(("gitea".to_string(), Box::new(ge)));
                }
            }
            _ => {}
        }
    }

    if provider_instances.is_empty() {
        anyhow::bail!("No providers enabled or available");
    }

    // Create language filter
    let lang_filter = LanguageFilter::new(config.general.language_filter.clone());

    // Fetch repositories in parallel
    let cache_arc = Arc::new(cache);
    let mut futures = FuturesUnordered::new();

    for (provider_id, provider) in provider_instances {
        let cache_ref = Arc::clone(&cache_arc);
        let lang_filter_clone = lang_filter.clone();
        let config_clone = config.clone();

        let future = async move {
            // Try cache first
            if let Some(ref cache) = *cache_ref {
                if let Some(cached_repos) = cache.get(&provider_id).await {
                    return Ok((provider_id.clone(), cached_repos));
                }
            }

            // Build provider config
            let provider_cfg = ProviderCfg {
                timeout_secs: config_clone.general.timeout_secs,
                token: match provider_id.as_str() {
                    "github" => config_clone.auth.github_token.clone(),
                    "gitlab" => config_clone.auth.gitlab_token.clone(),
                    "gitea" => config_clone.auth.gitea_token.clone(),
                    _ => None,
                },
                base_url: if provider_id == "gitea" {
                    Some(config_clone.gitea.base_url.clone())
                } else {
                    None
                },
            };

            // Fetch from provider
            let repos = provider
                .top_today(
                    &provider_cfg,
                    config_clone.general.max_per_provider,
                    &lang_filter_clone,
                )
                .await?;

            // Cache the result
            if let Some(ref cache) = *cache_ref {
                let _ = cache.set(&provider_id, repos.clone()).await;
            }

            Ok::<_, anyhow::Error>((provider_id, repos))
        };

        futures.push(future);
    }

    // Collect results
    let mut all_repos = Vec::new();
    let mut errors = Vec::new();

    while let Some(result) = futures.next().await {
        match result {
            Ok((provider_id, repos)) => {
                if !repos.is_empty() {
                    all_repos.extend(repos);
                } else if format!("{format:?}") == "Motd" {
                    eprintln!("⚠ No repositories found for {provider_id}");
                }
            }
            Err(e) => {
                errors.push(e);
            }
        }
    }

    // Handle errors
    if !errors.is_empty() {
        for error in &errors {
            eprintln!("✗ Error: {error}");
        }
    }

    // If all providers failed and we have no repos, exit with error
    if all_repos.is_empty() && !errors.is_empty() {
        anyhow::bail!("All providers failed");
    }

    // Render output
    render(&all_repos, format);

    Ok(())
}
