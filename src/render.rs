use chrono::{Duration, Utc};
use colored::Colorize;
use serde_json::json;
use unicode_width::{UnicodeWidthStr, UnicodeWidthChar};

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

/// Render plain MOTD format with table alignment
fn render_motd(repos: &[Repo]) {
    if repos.is_empty() {
        println!("No trending repositories found today.");
        return;
    }

    // Calculate column widths for alignment
    let max_name_len = repos
        .iter()
        .map(|r| r.name.width())
        .max()
        .unwrap_or(0)
        .min(40); // Cap name width at 40 chars

    let max_lang_len = repos
        .iter()
        .filter_map(|r| r.language.as_ref().map(|l| l.width()))
        .max()
        .unwrap_or(0)
        .min(15); // Cap language width at 15 chars

    for repo in repos {
        render_repo_motd(repo, max_name_len, max_lang_len);
    }
}

/// Clean description by removing/simplifying markdown syntax
fn clean_description(desc: &str) -> String {
    let mut result = desc.to_string();

    // Remove image markdown ![alt](url) - must be done before link conversion
    let img_re = regex::Regex::new(r"!\[[^\]]*\]\([^)]*\)").unwrap();
    result = img_re.replace_all(&result, "").to_string();

    // Convert markdown links [text](url) to just text
    let link_re = regex::Regex::new(r"\[([^\]]+)\]\([^)]+\)").unwrap();
    result = link_re.replace_all(&result, "$1").to_string();

    // Remove standalone markdown link syntax that might be malformed
    let broken_link_re = regex::Regex::new(r"\[[^\]]*\]\([^)]*$").unwrap();
    result = broken_link_re.replace_all(&result, "").to_string();

    // Remove bold/italic markers
    result = result.replace("**", "").replace("__", "");

    // Collapse multiple spaces
    let spaces_re = regex::Regex::new(r"\s+").unwrap();
    result = spaces_re.replace_all(&result, " ").to_string();

    result.trim().to_string()
}

/// Clean up truncated text to remove incomplete words or markdown
fn clean_truncated_text(text: &str) -> String {
    let mut result = text.to_string();

    // Remove incomplete markdown link at the end: [text](partial or [text or [
    if let Some(bracket_pos) = result.rfind('[') {
        // Check if there's a closing bracket after it
        if result[bracket_pos..].find(']').is_none()
            || (result[bracket_pos..].contains(']')
                && result[bracket_pos..].contains('(')
                && !result[bracket_pos..].contains(')'))
        {
            result = result[..bracket_pos].to_string();
        }
    }

    // Remove incomplete parenthetical at the end
    if let Some(paren_pos) = result.rfind('(') {
        if !result[paren_pos..].contains(')') {
            result = result[..paren_pos].to_string();
        }
    }

    // Remove trailing incomplete word (if ends with letter, backtrack to space)
    result = result.trim_end().to_string();

    result
}

/// Format recency from last_activity timestamp
fn format_recency(repo: &Repo) -> String {
    match repo.last_activity {
        Some(dt) => {
            let now = Utc::now();
            let diff = now.signed_duration_since(dt);

            if diff < Duration::hours(24) {
                "today".to_string()
            } else if diff < Duration::hours(48) {
                "yesterday".to_string()
            } else if diff < Duration::days(7) {
                format!("{}d ago", diff.num_days())
            } else if diff < Duration::days(30) {
                format!("{}w ago", diff.num_weeks())
            } else {
                format!("{}mo ago", diff.num_days() / 30)
            }
        }
        None => "unknown".to_string(),
    }
}

/// Render a single repository in MOTD format with colors and alignment
fn render_repo_motd(repo: &Repo, name_width: usize, lang_width: usize) {
    // Icon (colored by provider)
    let icon = match repo.provider.as_str() {
        "github" => repo.icon.bright_purple(),
        "gitlab" => repo.icon.bright_red(),
        "gitea" => repo.icon.bright_green(),
        _ => repo.icon.white(),
    };

    // Name (truncate if too long, pad for alignment)
    let name_display = if repo.name.width() > name_width {
        let mut total_width = 0;
        let truncated: String = repo.name.chars().take_while(|c| {
            let char_width = if c.is_ascii() { 1 } else { c.width().unwrap_or(1) };
            total_width += char_width;
            total_width <= name_width - 2
        }).collect();
        format!("{truncated}..")
    } else {
        repo.name.clone()
    };
    let name_padded = format!("{:<width$}", name_display, width = name_width);
    let name = name_padded.bright_cyan().bold();

    // Language (pad for alignment)
    let lang_display = repo.language.as_deref().unwrap_or("-");
    let lang_truncated = if lang_display.width() > lang_width {
        let mut total_width = 0;
        let truncated: String = lang_display.chars().take_while(|c| {
            let char_width = if c.is_ascii() { 1 } else { c.width().unwrap_or(1) };
            total_width += char_width;
            total_width <= lang_width - 2
        }).collect();
        format!("{truncated}..")
    } else {
        lang_display.to_string()
    };
    let lang_padded = format!("{:<width$}", lang_truncated, width = lang_width);
    let lang = lang_padded.bright_yellow();

    // Stars
    let stars = if let Some(stars_today) = repo.stars_today {
        format!("★{:<4} today", stars_today).bright_green().to_string()
    } else if let Some(stars_total) = repo.stars_total {
        format!("★{:<10}", stars_total)
            .bright_black()
            .to_string()
    } else {
        format!("{:<11}", "").to_string()
    };

    // Recency
    let recency = format_recency(repo);
    let recency_colored = match recency.as_str() {
        "today" => recency.bright_green(),
        "yesterday" => recency.yellow(),
        _ => recency.bright_black(),
    };

    // Description (truncate for remaining space)
    let desc = if let Some(ref d) = repo.description {
        let cleaned = clean_description(d);
        if cleaned.width() > 45 {
            let mut total_width = 0;
            let truncated: String = cleaned.chars().take_while(|c| {
                let char_width = if c.is_ascii() { 1 } else { c.width().unwrap_or(1) };
                total_width += char_width;
                total_width <= 42
            }).collect();
            let final_text = clean_truncated_text(&truncated);
            format!("{final_text}...")
        } else {
            cleaned
        }
    } else {
        String::new()
    };

    // Print aligned columns
    println!(
        "{} {} {} {} {} {}",
        icon,
        name,
        lang,
        stars,
        format!("{:<10}", recency_colored),
        desc.white()
    );
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
            last_activity: Some(Utc::now()),
            topics: vec!["rust".to_string(), "cli".to_string()],
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
                last_activity: Some(Utc::now()),
                topics: vec!["rust".to_string(), "compiler".to_string()],
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
                last_activity: Some(Utc::now() - Duration::days(3)),
                topics: vec!["gitlab".to_string(), "ruby".to_string()],
            },
        ];

        render(&repos, OutputFormat::Motd);
    }

    #[test]
    fn test_clean_description_markdown_links() {
        // Full markdown links should be converted to just text
        let desc = "Check out [README](https://example.com) for more info";
        let cleaned = clean_description(desc);
        assert_eq!(cleaned, "Check out README for more info");
    }

    #[test]
    fn test_clean_description_multiple_links() {
        let desc = "See [docs](url1) and [API](url2) for details";
        let cleaned = clean_description(desc);
        assert_eq!(cleaned, "See docs and API for details");
    }

    #[test]
    fn test_clean_description_bold_italic() {
        let desc = "This is **bold** and __also bold__ text";
        let cleaned = clean_description(desc);
        assert_eq!(cleaned, "This is bold and also bold text");
    }

    #[test]
    fn test_clean_description_images() {
        let desc = "Project logo ![logo](image.png) here";
        let cleaned = clean_description(desc);
        assert_eq!(cleaned, "Project logo here");
    }

    #[test]
    fn test_clean_truncated_incomplete_link() {
        // Simulates truncating "[README](https://..." to "[README](h"
        let truncated = "Check out [README](h";
        let cleaned = clean_truncated_text(truncated);
        assert_eq!(cleaned, "Check out");
    }

    #[test]
    fn test_clean_truncated_incomplete_bracket() {
        let truncated = "See the [docs";
        let cleaned = clean_truncated_text(truncated);
        assert_eq!(cleaned, "See the");
    }

    #[test]
    fn test_clean_truncated_incomplete_paren() {
        let truncated = "Some text (partial";
        let cleaned = clean_truncated_text(truncated);
        assert_eq!(cleaned, "Some text");
    }

    #[test]
    fn test_clean_truncated_complete_link() {
        // Complete links should not be removed
        let truncated = "Check [docs](url) here";
        let cleaned = clean_truncated_text(truncated);
        assert_eq!(cleaned, "Check [docs](url) here");
    }
}

