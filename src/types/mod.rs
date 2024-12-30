use serde::Serialize;

pub mod github;
pub mod analysis;

#[derive(Debug, Serialize, Clone)]
pub enum FileCategory {
    Source(String),
    Configuration,
    Documentation,
    Test,
    Unknown,
}

#[derive(Debug, Serialize, Clone)]
pub enum Visibility {
    Public,
    Private,
    PublicCrate,
}