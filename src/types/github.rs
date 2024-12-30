use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GithubContent {
    pub name: String,
    pub path: String,
    pub sha: String,
    pub size: i32,
    pub url: String,
    pub content: Option<String>,
    pub encoding: Option<String>,
    #[serde(rename = "type")]
    pub content_type: String,
}