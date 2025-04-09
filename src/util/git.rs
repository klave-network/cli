use std::process::Command;
use std::path::Path;
use std::env;
use reqwest;
use indicatif::{ProgressBar, ProgressStyle};

/// Finds user's name by reading it from the git config.
pub fn find_my_name() -> String {
    match Command::new("git")
        .args(&["config", "--get", "user.name"])
        .output()
    {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
        Err(_) => String::new(),
    }
}

/// Finds user's email by reading it from the git config.
pub fn find_github_email() -> String {
    match Command::new("git")
        .args(&["config", "--get", "user.email"])
        .output()
    {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
        Err(_) => String::new(),
    }
}

/// Get the GitHub username from an email address.
pub async fn find_github_profile_url(email: &str) -> String {
    // Note: The `github-username` crate doesn't exist in Rust
    // This is a simplified implementation that uses the GitHub API
    if email.is_empty() {
        return String::new();
    }

    match reqwest::Client::new()
        .get(&format!("https://api.github.com/search/users?q={}+in:email", email))
        .header("User-Agent", "Rust GitHub Username Finder")
        .send()
        .await
    {
        Ok(response) => {
            if let Ok(json) = response.json::<serde_json::Value>().await {
                if let Some(items) = json.get("items").and_then(|i| i.as_array()) {
                    if let Some(first_item) = items.first() {
                        if let Some(username) = first_item.get("login").and_then(|l| l.as_str()) {
                            return format!("https://github.com/{}", username);
                        }
                    }
                }
            }
            String::new()
        }
        Err(_) => String::new(),
    }
}

/// Guesses the repository URL based on the author profile URL and the package slug.
pub fn guess_repo_url(author_url: &str, slug: &str) -> String {
    if author_url.starts_with("https://github.com/") {
        let normalized_slug = if slug == "." || slug == "./" {
            match env::current_dir() {
                Ok(path) => path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("")
                    .to_string(),
                Err(_) => String::new(),
            }
        } else if slug.starts_with("./") || slug.starts_with("../") {
            Path::new(slug)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("")
                .to_string()
        } else {
            String::new()
        };

        if !normalized_slug.is_empty() {
            return format!("{}/{}", author_url, normalized_slug);
        }
    }
    
    String::new()
}

/// Create an empty Git repository.
pub async fn create_git_repo_async(target_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .unwrap());
    pb.set_message("Creating an empty Git repository");
    
    let output = Command::new("git")
        .args(&["init"])
        .current_dir(target_dir)
        .output()?;
    
    if !output.status.success() {
        return Err(format!("Failed to initialize Git repository: {}", 
            String::from_utf8_lossy(&output.stderr)).into());
    }
    
    pb.finish_with_message("Created an empty Git repository");
    Ok(())
}