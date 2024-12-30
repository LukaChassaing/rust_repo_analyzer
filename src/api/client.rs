use reqwest::{Client, StatusCode, header};
use tokio::time::{sleep, Duration};
use serde::de::DeserializeOwned;
use base64;
use std::env;

use crate::error::GithubAnalyzerError;
use crate::types::github::GithubContent;

pub struct GithubClient {
    client: Client,
    token: Option<String>,
}

impl GithubClient {
    pub fn new() -> Self {
        let token = env::var("GITHUB_TOKEN").ok();

        if token.is_some() {
            println!("Using authenticated GitHub API requests");
        } else {
            println!("Warning: Using unauthenticated GitHub API requests. Consider setting GITHUB_TOKEN environment variable to increase rate limits.");
        }

        Self {
            client: Client::new(),
            token,
        }
    }

    fn build_headers(&self) -> header::HeaderMap {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("GitHub-Repository-Analyzer")
        );
        
        if let Some(token) = &self.token {
            headers.insert(
                header::AUTHORIZATION,
                header::HeaderValue::from_str(&format!("token {}", token))
                    .expect("Invalid token format")
            );
        }
        
        headers
    }

    pub async fn get_with_retry<T>(&self, url: &str, max_retries: u32) -> Result<T, GithubAnalyzerError> 
where 
    T: DeserializeOwned
{
    let mut retries = 0;
    let mut last_error = None;

    while retries <= max_retries {
        if retries > 0 {
            let wait_time = 2u64.pow(retries);
            println!("Request failed, retrying in {} seconds... ({}/{})", wait_time, retries, max_retries);
            sleep(Duration::from_secs(wait_time)).await;
        }

        match self.client.get(url)
            .headers(self.build_headers())
            .send()
            .await
        {
            Ok(response) => {
                // Gérer les limites de rate
                if let Some(remaining) = response.headers()
                    .get("x-ratelimit-remaining")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u32>().ok())
                {
                    if remaining == 0 {
                        if let Some(reset) = response.headers()
                            .get("x-ratelimit-reset")
                            .and_then(|h| h.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                        {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs();
                            
                            if reset > now {
                                let wait_time = reset - now + 1;
                                println!("Rate limit exceeded. Waiting {} seconds for reset...", wait_time);
                                sleep(Duration::from_secs(wait_time)).await;
                                continue;
                            }
                        }
                    }
                }

                // Vérifier le statut de la réponse
                match response.status() {
                    status if status.is_success() => {
                        return response.json::<T>().await
                            .map_err(|e| GithubAnalyzerError::ParseError(e.to_string()));
                    },
                    StatusCode::FORBIDDEN => {
                        return Err(GithubAnalyzerError::RateLimitError(
                            response.headers()
                                .get("x-ratelimit-reset")
                                .and_then(|h| h.to_str().ok())
                                .and_then(|s| s.parse::<u64>().ok())
                                .unwrap_or(0)
                        ));
                    },
                    status => {
                        last_error = Some(GithubAnalyzerError::NetworkError(
                            format!("GitHub API returned status {}: {}", status, url)
                        ));
                    }
                }
            },
            Err(e) => {
                last_error = Some(GithubAnalyzerError::NetworkError(e.to_string()));
            }
        }

        retries += 1;
    }

    Err(last_error.unwrap_or_else(|| 
        GithubAnalyzerError::NetworkError("Maximum retries exceeded".to_string())
    ))
}

    pub async fn get_repo_contents(
        &self,
        repo_url: &str,
        path: &str,
        branch: &str,
    ) -> Result<Vec<GithubContent>, GithubAnalyzerError> {
        let api_url = repo_url
            .replace("github.com", "api.github.com/repos")
            .replace("tree/main", "")
            .replace("tree/master", "") 
            + "/contents/"
            + path
            + "?ref="
            + branch;

        // Try parsing as array first, then as single item
        match self.get_with_retry::<Vec<GithubContent>>(&api_url, 3).await {
            Ok(contents) => Ok(contents),
            Err(_e) => {
                // Try parsing as single item
                match self.get_with_retry::<GithubContent>(&api_url, 3).await {
                    Ok(item) => Ok(vec![item]),
                    Err(e) => Err(e)
                }
            }
        }
    }

    pub async fn get_file_content(
        &self,
        content_url: &str,
    ) -> Result<String, GithubAnalyzerError> {
        let content: GithubContent = self.get_with_retry(content_url, 3).await?;
        
        match (content.content, content.encoding) {
            (Some(content), Some(encoding)) if encoding == "base64" => {
                let decoded = base64::decode(content.replace("\n", ""))
                    .map_err(|e| GithubAnalyzerError::ParseError(e.to_string()))?;
                String::from_utf8(decoded)
                    .map_err(|e| GithubAnalyzerError::ParseError(e.to_string()))
            },
            _ => Err(GithubAnalyzerError::ParseError("Content or encoding unavailable".into())),
        }
    }
}