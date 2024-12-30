use std::collections::HashMap;
use async_recursion::async_recursion;

use crate::{
    error::GithubAnalyzerError,
    types::{
        analysis::{ProjectSummary, ProjectOverview, RepositoryStructure, FileSummary},
        github::GithubContent,
        FileCategory,
    },
    api::client::GithubClient,
    analysis::file::{categorize_file, FileAnalyzer},
};

pub struct RepositoryAnalyzer {
    client: GithubClient,
    file_analyzer: FileAnalyzer,
}

impl Default for RepositoryAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl RepositoryAnalyzer {
    pub fn new() -> Self {
        Self {
            client: GithubClient::new(),
            file_analyzer: FileAnalyzer::new(),
        }
    }

    /// Analyse un dépôt GitHub complet
    pub async fn analyze(&self, repo_url: &str) -> Result<ProjectSummary, GithubAnalyzerError> {
        let branches = ["main", "master"];
        let mut last_error = None;
        
        // Essaie chaque branche jusqu'à ce qu'une fonctionne
        for branch in branches {
            match self.try_analyze_branch(repo_url, branch).await {
                Ok(summary) => return Ok(summary),
                Err(e) => last_error = Some(e),
            }
        }
        
        Err(last_error.unwrap_or_else(|| 
            GithubAnalyzerError::NetworkError("Failed to access repository on any branch".to_string())
        ))
    }

    /// Tente d'analyser une branche spécifique du dépôt
    async fn try_analyze_branch(
        &self,
        repo_url: &str,
        branch: &str,
    ) -> Result<ProjectSummary, GithubAnalyzerError> {
        let mut project_summary = ProjectSummary {
            repo_url: repo_url.to_string(),
            files_analyzed: Vec::new(),
            total_files: 0,
            file_summaries: Vec::new(),
            important_patterns: Vec::new(),
            project_overview: ProjectOverview {
                total_rust_files: 0,
                total_public_types: 0,
                total_public_functions: 0,
                total_tests: 0,
                main_modules: Vec::new(),
                key_types: Vec::new(),
                dependencies: Vec::new(),
                type_relations: Vec::new(),
                method_signatures: Vec::new(),
                configuration: crate::types::analysis::Configuration {
                    constants: Vec::new(),
                    feature_flags: Vec::new(),
                    custom_attributes: Vec::new(),
                },
            },
            repository_structure: RepositoryStructure {
                has_src_directory: false,
                has_tests: false,
                has_docs: false,
                primary_language: None,
                build_systems: Vec::new(),
                branch_analyzed: branch.to_string(),
            },
        };

        // Analyse récursive du dépôt
        self.analyze_directory("", branch, &mut project_summary).await?;

        // Finalise l'analyse
        self.finalize_analysis(&mut project_summary);

        Ok(project_summary)
    }

    /// Analyse récursivement un répertoire du dépôt
    #[async_recursion]
    async fn analyze_directory(
        &self,
        path: &str,
        branch: &str,
        project_summary: &mut ProjectSummary,
    ) -> Result<(), GithubAnalyzerError> {
        let contents = self.client.get_repo_contents(&project_summary.repo_url, path, branch).await?;
        
        for content in contents {
            match content.content_type.as_str() {
                "dir" => {
                    self.process_directory(&content, branch, project_summary).await?;
                },
                "file" => {
                    self.process_file(&content, project_summary).await?;
                },
                _ => {} // Ignore other types
            }
        }
        
        Ok(())
    }

    /// Traite un répertoire
    async fn process_directory(
        &self,
        content: &GithubContent,
        branch: &str,
        project_summary: &mut ProjectSummary,
    ) -> Result<(), GithubAnalyzerError> {
        // Met à jour la structure du projet
        if content.path.starts_with("src/") {
            let module_name = content.path.replace("src/", "").replace(".rs", "");
            if !module_name.is_empty() && !project_summary.project_overview.main_modules.contains(&module_name) {
                project_summary.project_overview.main_modules.push(module_name);
            }
            project_summary.repository_structure.has_src_directory = true;
        }

        // Analyse récursive du répertoire
        self.analyze_directory(&content.path, branch, project_summary).await
    }

    /// Traite un fichier individuel
    async fn process_file(
        &self,
        content: &GithubContent,
        project_summary: &mut ProjectSummary,
    ) -> Result<(), GithubAnalyzerError> {
        // Skip des fichiers trop gros
        if content.size > 1000000 {
            return Ok(());
        }

        let category = categorize_file(&content.name);
        
        // Mise à jour de la structure du projet selon le type de fichier
        self.update_project_structure(content, &category, project_summary);

        // Analyse du contenu pour certains types de fichiers
        if matches!(category, 
            FileCategory::Source(_) | 
            FileCategory::Configuration | 
            FileCategory::Documentation
        ) {
            if let Ok(file_content) = self.client.get_file_content(&content.url).await {
                let (summary, type_relations, method_signatures, configuration) = 
                    self.file_analyzer.analyze_content(&file_content, &content.path).await;

                self.update_project_summary(
                    content,
                    &summary,
                    type_relations,
                    method_signatures,
                    configuration,
                    category,
                    project_summary,
                );
            }
        }

        project_summary.files_analyzed.push(content.path.clone());
        Ok(())
    }

    /// Met à jour la structure du projet en fonction du type de fichier
    fn update_project_structure(
        &self,
        content: &GithubContent,
        category: &FileCategory,
        project_summary: &mut ProjectSummary,
    ) {
        match category {
            FileCategory::Source(_) => {
                if content.path.starts_with("src/") {
                    project_summary.repository_structure.has_src_directory = true;
                }
            },
            FileCategory::Test => {
                project_summary.repository_structure.has_tests = true;
            },
            FileCategory::Documentation => {
                project_summary.repository_structure.has_docs = true;
            },
            FileCategory::Configuration => {
                self.update_build_systems(&content.name, project_summary);
            },
            FileCategory::Unknown => {}
        }
    }

    /// Met à jour la liste des systèmes de build détectés
    fn update_build_systems(&self, filename: &str, project_summary: &mut ProjectSummary) {
        let build_system = match filename {
            "Cargo.toml" => Some("Rust/Cargo"),
            "package.json" => Some("Node.js/npm"),
            "go.mod" => Some("Go/modules"),
            "pom.xml" => Some("Java/Maven"),
            "build.gradle" => Some("Java/Gradle"),
            "CMakeLists.txt" => Some("C++/CMake"),
            _ => None
        };

        if let Some(system) = build_system {
            if !project_summary.repository_structure.build_systems.contains(&system.to_string()) {
                project_summary.repository_structure.build_systems.push(system.to_string());
            }
        }
    }

    /// Met à jour le résumé du projet avec les résultats de l'analyse d'un fichier
    fn update_project_summary(
        &self,
        content: &GithubContent,
        summary: &str,
        type_relations: Vec<crate::types::analysis::TypeRelations>,
        method_signatures: Vec<crate::types::analysis::MethodSignature>,
        configuration: crate::types::analysis::Configuration,
        category: FileCategory,
        project_summary: &mut ProjectSummary,
    ) {
        // Met à jour les statistiques spécifiques au langage
        if let FileCategory::Source(ref lang) = category {
            if lang == "rs" {
                project_summary.project_overview.total_rust_files += 1;
                self.update_rust_stats(summary, &type_relations, &method_signatures, &configuration, project_summary);
            }
        }

        // Ajoute le résumé du fichier
        project_summary.file_summaries.push(FileSummary {
            path: content.path.clone(),
            size: content.size,
            summary: summary.to_string(),
            category,
            url: content.url.clone(), // Ajout de l'URL
        });
    }

    /// Met à jour les statistiques spécifiques à Rust
    fn update_rust_stats(
        &self,
        summary: &str,
        type_relations: &[crate::types::analysis::TypeRelations],
        method_signatures: &[crate::types::analysis::MethodSignature],
        configuration: &crate::types::analysis::Configuration,
        project_summary: &mut ProjectSummary,
    ) {
        // Met à jour les relations de types et signatures
        project_summary.project_overview.type_relations.extend_from_slice(type_relations);
        project_summary.project_overview.method_signatures.extend_from_slice(method_signatures);
        
        // Met à jour la configuration
        project_summary.project_overview.configuration.constants.extend_from_slice(&configuration.constants);
        project_summary.project_overview.configuration.feature_flags.extend_from_slice(&configuration.feature_flags);
        project_summary.project_overview.configuration.custom_attributes.extend_from_slice(&configuration.custom_attributes);
        
        // Met à jour les statistiques
        project_summary.project_overview.total_public_types += 
            summary.matches("Public struct: ").count() as i32
            + summary.matches("Public enum: ").count() as i32
            + summary.matches("Public trait: ").count() as i32;
        
        project_summary.project_overview.total_public_functions += 
            summary.lines()
                .filter(|line| line.contains("Public method: "))
                .count() as i32;
        
        project_summary.project_overview.total_tests += 
            summary.matches("Unit test: ").count() as i32;
    }

    /// Finalise l'analyse en calculant les statistiques globales
    fn finalize_analysis(&self, project_summary: &mut ProjectSummary) {
        project_summary.total_files = project_summary.files_analyzed.len() as i32;
        
        // Détermine le langage principal
        let mut language_counts: HashMap<String, usize> = HashMap::new();
        for summary in &project_summary.file_summaries {
            if let FileCategory::Source(ref lang) = summary.category {
                *language_counts.entry(lang.clone()).or_insert(0) += 1;
            }
        }
        
        if let Some((lang, _)) = language_counts.into_iter().max_by_key(|(_, count)| *count) {
            project_summary.repository_structure.primary_language = Some(lang);
        }
    }
}

/// Point d'entrée principal pour l'analyse d'un dépôt
pub async fn analyze_repository(repo_url: &str) -> Result<ProjectSummary, GithubAnalyzerError> {
    let analyzer = RepositoryAnalyzer::new();
    analyzer.analyze(repo_url).await
}