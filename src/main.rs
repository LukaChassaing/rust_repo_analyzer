mod error;
mod types;
mod analysis;
mod api;
mod export;

use std::error::Error;
use crate::analysis::repository::analyze_repository;
use export::ProjectExporter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <repo_url1> [repo_url2] ...", args[0]);
        return Ok(());
    }

    let repos = &args[1..];
    
    for repo_url in repos {
        println!("Analyzing repository: {}", repo_url);
        match analyze_repository(repo_url).await {
            Ok(summary) => {
                let mut exporter = ProjectExporter::new(repo_url)?;

                if let Err(e) = exporter.write_summary(&summary) {
                    println!("Warning: Failed to write analysis summary: {}", e);
                } else {
                    println!("✓ Analysis JSON exported");
                }

                let client = api::client::GithubClient::new();
                for file_summary in &summary.file_summaries {
                    match client.get_file_content(&file_summary.url).await {
                        Ok(content) => {
                            if let Err(e) = exporter.add_file(file_summary.path.clone(), content) {
                                println!("Warning: Failed to export {}: {}", file_summary.path, e);
                            }
                        }
                        Err(e) => {
                            println!("Warning: Failed to fetch {}: {}", file_summary.path, e);
                            continue;
                        }
                    }
                }

                if let Err(e) = exporter.finish() {
                    println!("Warning: Failed to finalize export: {}", e);
                } else {
                    let repo_name = repo_url.split('/').last().unwrap_or("repo");
                    println!("✓ Export completed in output/{}/", repo_name);
                    println!("  → Copy output/{}/complete_analysis.txt to share the entire codebase", repo_name);
                }
                
                println!("Quick stats:");
                println!("  - Files analyzed: {}", summary.total_files);
                println!("  - Primary language: {}", 
                    summary.repository_structure.primary_language.as_deref().unwrap_or("Unknown"));
                println!("  - Build systems: {}", 
                    summary.repository_structure.build_systems.join(", "));
                if summary.project_overview.total_rust_files > 0 {
                    println!("  - Rust files: {}", summary.project_overview.total_rust_files);
                    println!("  - Public types: {}", summary.project_overview.total_public_types);
                    println!("  - Public functions: {}", summary.project_overview.total_public_functions);
                }
            },
            Err(e) => println!("✗ Error analyzing {}: {}", repo_url, e),
        }
    }

    Ok(())
}