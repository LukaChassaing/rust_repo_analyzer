use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;
use serde::Serialize;

const DELIMITER: &str = "\n<document>\n<source>{}</source>\n<document_content>\n{}\n</document_content>\n</document>\n";
const CHUNK_SIZE: usize = 5;

pub struct ProjectExporter {
    project_dir: PathBuf,
    current_files: Vec<(String, String)>,
    chunk_counter: usize,
}

impl ProjectExporter {
    pub fn new(repo_url: &str) -> std::io::Result<Self> {

        let repo_name = repo_url
            .split('/')
            .last()
            .unwrap_or("unknown_repo")
            .replace(".git", "");

        let project_dir = Path::new("output").join(&repo_name);
        fs::create_dir_all(&project_dir)?;
        
        Ok(Self {
            project_dir,
            current_files: Vec::new(),
            chunk_counter: 0,
        })
    }
    
    pub fn add_file(&mut self, filename: String, content: String) -> std::io::Result<()> {
        self.current_files.push((filename, content));
        
        if self.current_files.len() >= CHUNK_SIZE {
            self.write_chunk()?;
        }
        
        Ok(())
    }
    
    fn write_chunk(&mut self) -> std::io::Result<()> {
        if self.current_files.is_empty() {
            return Ok(());
        }
        
        let mut chunk_content = String::new();
        for (filename, content) in &self.current_files {
            chunk_content.push_str(&DELIMITER.replace("{}", filename));
            chunk_content.push_str(content);
        }

        let chunks_dir = self.project_dir.join("chunks");
        fs::create_dir_all(&chunks_dir)?;
        let chunk_filename = chunks_dir.join(format!("chunk_{}.txt", self.chunk_counter));
        let mut file = fs::File::create(chunk_filename)?;
        file.write_all(chunk_content.as_bytes())?;
        
        self.current_files.clear();
        self.chunk_counter += 1;
        
        Ok(())
    }
    
    pub fn write_summary<T: Serialize>(&self, summary: &T) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(summary)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        let summary_path = self.project_dir.join("analysis.json");
        fs::write(summary_path, json)?;
        
        Ok(())
    }
    
    pub fn finish(mut self) -> std::io::Result<()> {
        // Écrire le dernier chunk si nécessaire
        self.write_chunk()?;
        
        // Créer l'index qui combine tous les chunks
        let mut index_content = String::new();
        
        // Ajouter le JSON d'analyse
        index_content.push_str("\n<document>\n<source>analysis.json</source>\n<document_content>\n");
        if let Ok(analysis) = fs::read_to_string(self.project_dir.join("analysis.json")) {
            index_content.push_str(&analysis);
        }
        index_content.push_str("\n</document_content>\n</document>\n");
        
        // Ajouter tous les chunks dans l'ordre
        let chunks_dir = self.project_dir.join("chunks");
        for i in 0..self.chunk_counter {
            let chunk_path = chunks_dir.join(format!("chunk_{}.txt", i));
            if let Ok(chunk_content) = fs::read_to_string(chunk_path) {
                index_content.push_str(&chunk_content);
            }
        }
        
        // Écrire le fichier d'index
        let index_path = self.project_dir.join("complete_analysis.txt");
        fs::write(&index_path, index_content)?;
        
        // Mettre à jour le README
        let readme_content = format!(
            "# Repository Analysis Output\n\n\
            This directory contains the analysis results for the repository.\n\n\
            ## Files\n\
            - `complete_analysis.txt`: **Single file containing everything** - Use this for easy copy-paste into AI tools\n\
            - `analysis.json`: Complete analysis of the repository in JSON format\n\
            - `chunks/`: Directory containing code files split into manageable chunks\n\
                - Each chunk contains up to {} files\n\
                - Files are formatted with XML-style tags for easy parsing\n\n\
            ## Format\n\
            Files are wrapped in XML-style tags:\n\
            ```\n\
            <document>\n\
            <source>filename</source>\n\
            <document_content>\n\
            // actual file content\n\
            </document_content>\n\
            </document>\n\
            ```\n\n\
            ## Usage\n\
            To analyze the entire codebase:\n\
            1. Copy the entire content of `complete_analysis.txt`\n\
            2. Paste it into your conversation with the AI\n\
            3. The AI will automatically recognize and parse all the files\n",
            CHUNK_SIZE
        );
        
        let readme_path = self.project_dir.join("README.md");
        fs::write(readme_path, readme_content)?;
        
        Ok(())
    }
}