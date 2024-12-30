use regex::Regex;
use std::collections::{HashMap, HashSet};
use crate::types::{
    analysis::{TypeRelations, MethodSignature, Configuration},
    FileCategory, Visibility
};

/// Catégorisation des fichiers selon leur type
pub fn categorize_file(filename: &str) -> FileCategory {
    let extension = std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    
    match extension {
        // Fichiers source
        "rs" | "go" | "js" | "py" | "java" | "cpp" | "c" => 
            FileCategory::Source(extension.to_string()),
        
        // Autres types de fichiers
        _ => match filename {
            // Fichiers de configuration
            f if is_config_file(f) => FileCategory::Configuration,
            
            // Documentation
            f if is_documentation_file(f) => FileCategory::Documentation,
            
            // Tests
            f if is_test_file(f) => FileCategory::Test,
            
            // Fichiers non catégorisés
            _ => FileCategory::Unknown
        }
    }
}

fn is_config_file(filename: &str) -> bool {
    const CONFIG_FILES: [&str; 5] = [
        "Cargo.toml",
        "package.json",
        "go.mod",
        "Makefile",
        "CMakeLists.txt"
    ];
    CONFIG_FILES.contains(&filename)
}

fn is_documentation_file(filename: &str) -> bool {
    filename.ends_with(".md") 
        || filename.starts_with("LICENSE")
        || filename.starts_with("CONTRIBUTING")
        || filename.starts_with("README")
        || filename.starts_with("CHANGELOG")
}

fn is_test_file(filename: &str) -> bool {
    filename.contains("test") 
        || filename.ends_with("_test.go")
        || filename.ends_with(".test.js")
        || filename.ends_with("Test.java")
        || filename.ends_with("_test.py")
        || filename.ends_with("_spec.rb")
}

/// Structure contenant les motifs d'analyse de code
#[derive(Debug)]
pub struct CodePatterns {
    type_pattern: Regex,
    impl_pattern: Regex,
    use_pattern: Regex,
    method_pattern: Regex,
    const_pattern: Regex,
    feature_pattern: Regex,
    attribute_pattern: Regex,
    field_type_pattern: Regex,
    return_type_pattern: Regex,
    generic_type_pattern: Regex,
    trait_impl_pattern: Regex,
    derive_pattern: Regex,
    type_reference_pattern: Regex,
    method_signature_pattern: Regex,
}

impl Default for CodePatterns {
    fn default() -> Self {
        Self::new()
    }
}

impl CodePatterns {
    pub fn new() -> Self {
        Self {
            type_pattern: Regex::new(r"^pub (?:struct|enum|type) (\w+)").unwrap(),
            impl_pattern: Regex::new(r"^impl(?:<[^>]+>)? (?:([^<\s]+)(?:<[^>]+>)? for )?([^<\s]+)").unwrap(),
            use_pattern: Regex::new(r"use .+::(\w+)").unwrap(),

            derive_pattern: Regex::new(r"#\[derive\((.*?)\)\]").unwrap(),
            trait_impl_pattern: Regex::new(
                r"impl(?:\s*<[^>]*>)?\s+([A-Z][a-zA-Z0-9_]*(?:<[^>]+>)?)\s+for\s+([A-Z][a-zA-Z0-9_]*(?:<[^>]+>)?)"
            ).unwrap(),
            method_signature_pattern: Regex::new(
                r"fn\s+\w+\s*(?:<[^>]*>)?\s*\(((?:[^()]*|\([^()]*\))*)\)(?:\s*->\s*([^{;]+))?"
            ).unwrap(),
            type_reference_pattern: Regex::new(
                r"[A-Z][a-zA-Z0-9_]*(?:<[^>]+>)?"
            ).unwrap(),

            method_pattern: Regex::new(
                r"(?P<vis>pub(?:\([^)]+\))?)?\s*fn\s+(?P<name>\w+)\s*<?\s*(?P<params>[^>]*?)>\s*\((?P<args>[^)]*)\)(?:\s*->\s*(?P<ret>[^{]+))?"
            ).unwrap(),
            const_pattern: Regex::new(r"(?:pub\s+)?const\s+([A-Z_][A-Z0-9_]*)\s*:\s*([^=]+)\s*=\s*([^;]+);").unwrap(),
            feature_pattern: Regex::new(r#"#\[cfg\(feature\s*=\s*"([^"]+)"\)\]"#).unwrap(),
            attribute_pattern: Regex::new(r"#\[([^\]]+)\]").unwrap(),
            field_type_pattern: Regex::new(r":\s*(?:&)?([A-Z][a-zA-Z0-9_]*(?:<[^>]+>)?)").unwrap(),
            return_type_pattern: Regex::new(r"->\s*(?:Result<)?(?:&)?([A-Z][a-zA-Z0-9_]*(?:<[^>]+>)?)").unwrap(),
            generic_type_pattern: Regex::new(r"<[^>]*?([A-Z][a-zA-Z0-9_]*)[^>]*>").unwrap(),
        }
    }
}

pub struct FileAnalyzer {
    patterns: CodePatterns,
}

impl Default for FileAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl FileAnalyzer {
    pub fn new() -> Self {
        Self {
            patterns: CodePatterns::new(),
        }
    }

    /// Analyse le contenu d'un fichier
    pub async fn analyze_content(
        &self,
        content: &str,
        file_path: &str,
    ) -> (String, Vec<TypeRelations>, Vec<MethodSignature>, Configuration) {
        println!("\n📁 Analyzing file: {}", file_path);

        let summary = self.generate_summary(content);
        println!("📝 Generated file summary");

        let type_relations = self.analyze_type_relations(content);
        println!("🔄 Analyzed type relations: {} types found", type_relations.len());

        for relation in &type_relations {
            println!("\n📌 Type: {}", relation.type_name);
            if !relation.implemented_traits.is_empty() {
                println!("  ↪ Implements: {}", relation.implemented_traits.join(", "));
            }
            if !relation.depends_on.is_empty() {
                println!("  ↪ Depends on: {}", relation.depends_on.join(", "));
            }
            if !relation.used_by.is_empty() {
                println!("  ↪ Used by: {}", relation.used_by.join(", "));
            }
        }

        let method_signatures = self.analyze_method_signatures(content);
        println!("🔍 Found {} method signatures", method_signatures.len());

        let configuration = self.analyze_configuration(content);
        println!("⚙️ Configuration analysis complete");
        println!("  ↪ {} constants", configuration.constants.len());
        println!("  ↪ {} feature flags", configuration.feature_flags.len());
        println!("  ↪ {} custom attributes", configuration.custom_attributes.len());

        (summary, type_relations, method_signatures, configuration)
    }

    /// Génère un résumé du contenu du fichier
    fn generate_summary(&self, content: &str) -> String {
        let mut summary = String::new();
        let lines: Vec<&str> = content.lines().collect();

        if !lines.is_empty() {
            summary.push_str("File start:\n");
            for line in lines.iter().take(5) {
                summary.push_str(&format!("{}\n", line));
            }
        }

        let patterns = [
            (r"^///\s*(.*)$", "Documentation: "),
            (r"^//!\s*(.*)$", "Module documentation: "),
            (r"^pub fn (\w+)", "Public method: "),
            (r"^fn (\w+)", "Private method: "),
            (r"^pub struct (\w+)", "Public struct: "),
            (r"^pub enum (\w+)", "Public enum: "),
            (r"^pub trait (\w+)", "Public trait: "),
            (r"^impl\s+(\w+)", "Implementation: "),
            (r"^\[.*\]", "Section: "),
        ];

        for (pattern, prefix) in patterns {
            let re = Regex::new(pattern).unwrap();
            for line in &lines {
                if re.is_match(line) {
                    summary.push_str(&format!("{}{}\n", prefix, line.trim()));
                }
            }
        }

        summary
    }

    fn analyze_type_relations(&self, content: &str) -> Vec<TypeRelations> {
        println!("\n🔎 Starting type relations analysis");

        let mut relations = Vec::new();
        let mut current_type: Option<String> = None;
        let mut dependencies = HashSet::new();
        let mut traits_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut usage_map: HashMap<String, HashSet<String>> = HashMap::new();
        let mut processed_types = HashSet::new();

        let mut project_types = HashSet::new();
        let type_decl = Regex::new(r"^(?:pub\s+)?(?:struct|enum|type)\s+([A-Z][a-zA-Z0-9_]*)").unwrap();

        // Première passe : collecter tous les types déclarés
        for line in content.lines() {
            let line = line.trim();
            if let Some(captures) = type_decl.captures(line) {
                let type_name = captures[1].to_string();
                println!("  Found type declaration: {}", type_name);
                project_types.insert(type_name);
            }
        }

        println!("  Discovered types: {:?}", project_types);

        // Deuxième passe : analyser les relations
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();

            // Analyse des dérivations (#[derive(...)])
            if line.starts_with("#[derive") {
                println!("  📍 Found derive: {}", line);
                if let Some(next_line) = lines.get(i + 1) {
                    if let Some(captures) = type_decl.captures(next_line.trim()) {
                        let type_name = captures[1].to_string();
                        let derive_pattern = Regex::new(r"#\[derive\((.*?)\)\]").unwrap();
                        if let Some(derive_captures) = derive_pattern.captures(line) {
                            let traits = derive_captures[1]
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .collect::<Vec<_>>();
                            traits_map.insert(type_name, traits);
                        }
                    }
                }
            }

            // Analyse des déclarations de types
            if let Some(captures) = type_decl.captures(line) {
                let type_name = captures[1].to_string();

                // Ne traiter que si c'est un nouveau type
                if !processed_types.contains(&type_name) {
                    println!("  → Analyzing new type: {}", type_name);

                    // Finaliser le type précédent
                    if let Some(prev_type) = current_type.take() {
                        println!("    Finalizing previous type: {}", prev_type);
                        self.add_type_relations(
                            &mut relations,
                            &prev_type,
                            &dependencies,
                            &traits_map,
                            &usage_map,
                            &project_types
                        );
                        dependencies.clear();
                    }

                    current_type = Some(type_name.clone());
                    processed_types.insert(type_name);
                } else {
                    println!("    Skipping already processed type: {}", type_name);
                }
            }

            // Analyse du contexte du type courant
            if let Some(ref current) = current_type {
                self.analyze_dependencies(
                    line,
                    current,
                    &mut dependencies,
                    &mut usage_map,
                    &project_types
                );
            }

            i += 1;
        }

        // Traiter le dernier type
        if let Some(type_name) = current_type {
            println!("  → Finalizing last type: {}", type_name);
            self.add_type_relations(
                &mut relations,
                &type_name,
                &dependencies,
                &traits_map,
                &usage_map,
                &project_types
            );
        }

        println!("🔄 Building transitive relations");
        self.build_type_relations(&mut relations);
        println!("✅ Type analysis complete: {} relations found", relations.len());

        relations
    }

    /// Analyse les dépendances dans une ligne de code
    fn analyze_dependencies(
        &self,
        line: &str,
        current_type: &str,
        dependencies: &mut HashSet<String>,
        usage_map: &mut HashMap<String, HashSet<String>>,
        project_types: &HashSet<String>,
    ) {
        if line.contains("impl") || line.contains(": ") || line.contains("->") {
            println!("    Analyzing line: {}", line);
        }

        for pattern in [
            r":\s*(?:&\s*)?([A-Z][a-zA-Z0-9_]*)\s*(?:<[^>]*>)?",
            r"fn\s+\w+\s*(?:<[^>]*>)?\s*\([^)]*?(?:&\s*)?([A-Z][a-zA-Z0-9_]*)",
            r"->\s*(?:Result<)?(?:&\s*)?([A-Z][a-zA-Z0-9_]*)",
            r"impl(?:\s*<[^>]*>)?\s+([A-Z][a-zA-Z0-9_]*)\s+for",
            r"<[^>]*?([A-Z][a-zA-Z0-9_]*)[^>]*>",
            r"(?:Vec|Option|Box)<([A-Z][a-zA-Z0-9_]*)>",
        ] {
            let re = Regex::new(pattern).unwrap();
            for captures in re.captures_iter(line) {
                let type_name = captures[1].to_string();
                if project_types.contains(&type_name) && type_name != current_type {
                    println!("    Found dependency: {} -> {}", current_type, type_name);
                    dependencies.insert(type_name.clone());
                    usage_map
                        .entry(type_name)
                        .or_default()
                        .insert(current_type.to_string());
                }
            }
        }
    }

    /// Ajoute les relations d'un type
    fn add_type_relations(
        &self,
        relations: &mut Vec<TypeRelations>,
        type_name: &str,
        dependencies: &HashSet<String>,
        traits_map: &HashMap<String, Vec<String>>,
        usage_map: &HashMap<String, HashSet<String>>,
        project_types: &HashSet<String>,
    ) {
        let mut implemented_traits = traits_map
            .get(type_name)
            .cloned()
            .unwrap_or_default();

        let mut used_by = vec![];
        if let Some(users) = usage_map.get(type_name) {
            used_by.extend(users.iter().cloned());
        }

        let depends_on = dependencies
            .iter()
            .filter(|dep| project_types.contains(*dep))
            .cloned()
            .collect();

        relations.push(TypeRelations {
            type_name: type_name.to_string(),
            implemented_traits,
            used_by,
            depends_on,
        });
    }

    /// Construit les relations transitives entre types
    fn build_type_relations(&self, relations: &mut [TypeRelations]) {
        let mut deps_graph: HashMap<String, HashSet<String>> = HashMap::new();
        let mut users_graph: HashMap<String, HashSet<String>> = HashMap::new();

        // Construction des graphes initiaux
        for relation in relations.iter() {
            deps_graph
                .entry(relation.type_name.clone())
                .or_default()
                .extend(relation.depends_on.iter().cloned());

            for dep in &relation.depends_on {
                users_graph
                    .entry(dep.clone())
                    .or_default()
                    .insert(relation.type_name.clone());
            }
        }

        // Calcul des fermetures transitives
        let mut changed = true;
        while changed {
            changed = false;

            for relation in relations.iter() {
                // Dépendances transitives
                let current_deps = deps_graph
                    .get(&relation.type_name)
                    .cloned()
                    .unwrap_or_default();

                let mut new_deps = current_deps.clone();
                for dep in &current_deps {
                    if let Some(indirect_deps) = deps_graph.get(dep) {
                        for indirect_dep in indirect_deps {
                            if new_deps.insert(indirect_dep.clone()) {
                                changed = true;
                            }
                        }
                    }
                }

                if changed {
                    deps_graph.insert(relation.type_name.clone(), new_deps);
                }

                // Utilisateurs transitifs
                let current_users = users_graph
                    .get(&relation.type_name)
                    .cloned()
                    .unwrap_or_default();

                let mut new_users = current_users.clone();
                for user in &current_users {
                    if let Some(indirect_users) = users_graph.get(user) {
                        for indirect_user in indirect_users {
                            if new_users.insert(indirect_user.clone()) {
                                changed = true;
                            }
                        }
                    }
                }

                if changed {
                    users_graph.insert(relation.type_name.clone(), new_users);
                }
            }
        }

        // Mise à jour des relations avec les dépendances transitives
        for relation in relations.iter_mut() {
            if let Some(deps) = deps_graph.get(&relation.type_name) {
                relation.depends_on = deps.iter().cloned().collect();
                relation.depends_on.sort();
            }

            if let Some(users) = users_graph.get(&relation.type_name) {
                relation.used_by = users.iter().cloned().collect();
                relation.used_by.sort();
            }
        }
    }

    /// Analyse les signatures des méthodes
    fn analyze_method_signatures(&self, content: &str) -> Vec<MethodSignature> {
        let mut signatures = Vec::new();
        
        for line in content.lines() {
            if let Some(captures) = self.patterns.method_pattern.captures(line) {
                let visibility = match captures.name("vis").map(|m| m.as_str()) {
                    Some("pub") => Visibility::Public,
                    Some("pub(crate)") => Visibility::PublicCrate,
                    _ => Visibility::Private,
                };
                
                let name = captures.name("name")
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();

                let params = captures.name("args")
                    .map(|args| args.as_str()
                        .split(',')
                        .filter_map(|p| Some(p.trim().to_string()))
                        .collect())
                    .unwrap_or_default();
                    
                let return_type = captures.name("ret")
                    .map(|r| r.as_str().trim().to_string())
                    .unwrap_or_else(|| "()".to_string());
                    
                signatures.push(MethodSignature {
                    name,
                    params,
                    return_type,
                    visibility,
                });
            }
        }
        
        signatures
    }

    /// Analyse la configuration (constantes, features, attributs)
    fn analyze_configuration(&self, content: &str) -> Configuration {
        let mut config = Configuration {
            constants: Vec::new(),
            feature_flags: Vec::new(),
            custom_attributes: Vec::new(),
        };
        
        for line in content.lines() {
            // Analyse des constantes
            if let Some(captures) = self.patterns.const_pattern.captures(line) {
                config.constants.push((
                    captures[1].to_string(),
                    captures[2].trim().to_string(),
                    captures[3].trim().to_string(),
                ));
            }
            
            // Analyse des features
            if let Some(captures) = self.patterns.feature_pattern.captures(line) {
                config.feature_flags.push(captures[1].to_string());
            }
            
            // Analyse des attributs personnalisés
            if let Some(captures) = self.patterns.attribute_pattern.captures(line) {
                let attr = captures[1].to_string();
                if !attr.starts_with("cfg") && !attr.starts_with("test") {
                    config.custom_attributes.push(attr);
                }
            }
        }
        
        config
    }
}