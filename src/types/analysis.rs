use serde::Serialize;
use super::{FileCategory, Visibility};

#[derive(Debug, Serialize, Clone)]
pub struct ProjectSummary {
    pub repo_url: String,
    pub files_analyzed: Vec<String>,
    pub total_files: i32,
    pub file_summaries: Vec<FileSummary>,
    pub important_patterns: Vec<String>,
    pub project_overview: ProjectOverview,
    pub repository_structure: RepositoryStructure,
}

#[derive(Debug, Serialize, Clone)]
pub struct RepositoryStructure {
    pub has_src_directory: bool,
    pub has_tests: bool,
    pub has_docs: bool,
    pub primary_language: Option<String>,
    pub build_systems: Vec<String>,
    pub branch_analyzed: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProjectOverview {
    pub total_rust_files: i32,
    pub total_public_types: i32,
    pub total_public_functions: i32,
    pub total_tests: i32,
    pub main_modules: Vec<String>,
    pub key_types: Vec<String>,
    pub dependencies: Vec<String>,
    pub type_relations: Vec<TypeRelations>,
    pub method_signatures: Vec<MethodSignature>,
    pub configuration: Configuration,
}

#[derive(Debug, Serialize, Clone)]
pub struct FileSummary {
    pub path: String,
    pub size: i32,
    pub summary: String,
    pub category: FileCategory,
    pub url: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct TypeRelations {
    pub type_name: String,
    pub implemented_traits: Vec<String>,
    pub used_by: Vec<String>,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MethodSignature {
    pub name: String,
    pub params: Vec<String>,
    pub return_type: String,
    pub visibility: Visibility,
}

#[derive(Debug, Serialize, Clone)]
pub struct Configuration {
    pub constants: Vec<(String, String, String)>,
    pub feature_flags: Vec<String>,
    pub custom_attributes: Vec<String>,
}