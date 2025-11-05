use colored::Colorize;
use serde_json::json;

use crate::model::Repo;

/// Output format
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Motd,
    Json,
}

/// Render repositories in MOTD format
pub fn render(repos: &[Repo], format: OutputFormat) {
    match format {
        OutputFormat::Motd => render_motd(repos),
        OutputFormat::Json => render_json(repos),
    }
}

/// Render plain MOTD format
fn render_motd(repos: &[Repo]) {
    if repos.is_empty() {
        println!("No trending repositories found today.");
        return;
    }

    for repo in repos {
        render_repo_motd(repo);
    }
}

/// Render a single repository in MOTD format with colors
fn render_repo_motd(repo: &Repo) {
    // Icon and name (colored by provider)
    let icon = match repo.provider.as_str() {
        "github" => repo.icon.bright_purple(),
        "gitlab" => repo.icon.bright_red(),
        "gitea" => repo.icon.bright_green(),
        _ => repo.icon.white(),
    };

    let name = repo.name.bright_cyan().bold();

    // Build the output line
    let mut parts = vec![format!("{icon}"), format!("{name}")];

    // Add language if present
    if let Some(ref lang) = repo.language {
        parts.push("•".bright_black().to_string());
        parts.push(lang.bright_yellow().to_string());
    }

    // Add description if present
    if let Some(ref desc) = repo.description {
        parts.push("•".bright_black().to_string());
        // Truncate long descriptions
        let truncated = if desc.len() > 60 {
            format!("{}...", &desc[..57])
        } else {
            desc.clone()
        };
        parts.push(truncated.white().to_string());
    }

    // Add stars if present
    if let Some(stars_today) = repo.stars_today {
        parts.push("•".bright_black().to_string());
        parts.push(format!("★{stars_today} today").bright_green().to_string());
    } else if let Some(stars_total) = repo.stars_total {
        parts.push("•".bright_black().to_string());
        parts.push(format!("★{stars_total}").bright_black().to_string());
    }

    // Add approximated indicator
    if repo.approximated {
        parts.push("~".bright_black().italic().to_string());
    }

    println!("{}", parts.join(" "));
}

/// Render JSON format
fn render_json(repos: &[Repo]) {
    let output = json!(repos);
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_empty() {
        let repos = vec![];
        // This will print to stdout, but won't panic
        render(&repos, OutputFormat::Motd);
    }

    #[test]
    fn test_render_json() {
        let repos = vec![Repo {
            provider: "github".to_string(),
            icon: "[GH]".to_string(),
            name: "test/repo".to_string(),
            language: Some("Rust".to_string()),
            description: Some("Test repository".to_string()),
            url: "https://github.com/test/repo".to_string(),
            stars_today: Some(10),
            stars_total: Some(100),
            approximated: false,
        }];

        render(&repos, OutputFormat::Json);
    }

    #[test]
    fn test_render_motd() {
        let repos = vec![
            Repo {
                provider: "github".to_string(),
                icon: "[GH]".to_string(),
                name: "rust-lang/rust".to_string(),
                language: Some("Rust".to_string()),
                description: Some(
                    "Empowering everyone to build reliable and efficient software.".to_string(),
                ),
                url: "https://github.com/rust-lang/rust".to_string(),
                stars_today: Some(50),
                stars_total: Some(90000),
                approximated: false,
            },
            Repo {
                provider: "gitlab".to_string(),
                icon: "[GL]".to_string(),
                name: "gitlab-org/gitlab".to_string(),
                language: Some("Ruby".to_string()),
                description: None,
                url: "https://gitlab.com/gitlab-org/gitlab".to_string(),
                stars_today: None,
                stars_total: Some(5000),
                approximated: true,
            },
        ];

        render(&repos, OutputFormat::Motd);
    }
}
