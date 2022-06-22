use crate::{
    capabilities::{self, formatting::get_format_text_edits},
    sway_config::SwayConfig,
    core_v2::{
        error::{
            ConfigError
        },
        traverse_parse_tree,
        traverse_typed_tree,
        token::TokenMap,
    }
};
use forc_pkg::{self as pkg};
use serde_json::Value;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, LockResult, RwLock},
};
use sway_core::{parse, semantic_analysis::ast_node::TypedAstNode, CompileAstResult, TreeType};

use tower_lsp::lsp_types::{
    CompletionItem, Diagnostic, GotoDefinitionResponse, Position, Range, SemanticToken,
    SymbolInformation, TextDocumentContentChangeEvent, TextEdit, Url, WorkspaceFolder,
};


#[derive(Debug)]
pub struct Session {
    #[allow(dead_code)]
    language_id: String,
    #[allow(dead_code)]
    version: i32,
    uri: String, 
    text: String,
    pub manifest: Option<pkg::ManifestFile>,
    pub token_map: TokenMap,
    pub config: RwLock<SwayConfig>,
}

impl Session {
    pub fn new() -> Self {
        Session {
            language_id: "sway".into(),
            version: 1,
            uri: "".to_string(),
            text: "".to_string(),
            manifest: None,
            token_map: HashMap::new(),
            config: RwLock::new(SwayConfig::default()),
        }
    }

    fn build_config(&self) -> Result<sway_core::BuildConfig, ConfigError> {
        let build_config = pkg::BuildConfig {
            print_ir: false,
            print_finalized_asm: false,
            print_intermediate_asm: false,
            silent: true,
        };
        match &self.manifest {
            Some(manifest) => {
                pkg::sway_build_config(manifest.dir(), &manifest.entry_path(), &build_config)
                .map_err(|_| ConfigError::BuildConfig)
            }
            None => {
                Err(ConfigError::NoManifestFile)
            }
        }
    }
    
    // TODO: should I return a Result with this_error Errors?
    pub fn initialize(&mut self, workspace_folder: Option<WorkspaceFolder>) {
        if let Some(workspace_folder) = workspace_folder {
            if let Ok(manifest_dir) = workspace_folder.uri.to_file_path() {
                if let Ok(manifest) = pkg::ManifestFile::from_dir(&manifest_dir, forc::utils::SWAY_GIT_TAG) {
                    self.manifest = Some(manifest);
                }
            }
        }
    }

    // TODO: create a Vec<Diagnostic> with warnings and errors
    pub fn parse_project(&mut self, uri: Url, text: String) {
        self.uri = uri.path().to_string();
        self.text = text;
        self.token_map.clear();

        // First, populate our token_map with un-typed ast nodes
        self.parse_ast_to_tokens();

        // Next, populate our token_map with typed ast nodes
        self.parse_ast_to_typed_tokens();
    }

    fn parse_ast_to_tokens(&mut self) {
        match self.build_config() {
            Ok(sway_build_config) => {
                let text = Arc::from(self.text.clone());
                let parsed_result = parse(text, Some(&sway_build_config));
                match parsed_result.value {
                    None => (),
                    Some(parse_program) => {
                        for node in &parse_program.root.tree.root_nodes {
                            traverse_parse_tree::traverse_node(node, &mut self.token_map);
                        }
                    }
                }
            },
            Err(_) => (),
        }
    }

    fn parse_ast_to_typed_tokens(&mut self) {
        match &self.manifest {
            Some(manifest) => {
                let silent_mode = true;
                let res = pkg::check(&manifest.dir(), silent_mode, forc::utils::SWAY_GIT_TAG).unwrap();
        
                match res {
                    CompileAstResult::Failure { .. } => (),
                    CompileAstResult::Success { typed_program, .. } => {
                        for node in &typed_program.root.all_nodes {
                            traverse_typed_tree::traverse_node(node, &mut self.token_map);
                        }
                    },
                }
            }
            None => (),
        }
    }

    // // update sway config
    // pub fn update_config(&self, options: Value) {
    //     if let LockResult::Ok(mut config) = self.config.write() {
    //         *config = SwayConfig::with_options(options);
    //     }
    // }

    // pub fn parse_document(&self, path: &str) -> Result<Vec<Diagnostic>, DocumentError> {
    //     match self.documents.get_mut(path) {
    //         Some(ref mut document) => document.parse(),
    //         _ => Err(DocumentError::DocumentNotFound),
    //     }
    // }

    // pub fn contains_sway_file(&self, url: &Url) -> bool {
    //     self.documents.contains_key(url.path())
    // }

    // pub fn update_text_document(&self, url: &Url, changes: Vec<TextDocumentContentChangeEvent>) {
    //     if let Some(ref mut document) = self.documents.get_mut(url.path()) {
    //         changes.iter().for_each(|change| {
    //             document.apply_change(change);
    //         });
    //     }
    // }

    // // Token
    // pub fn get_token_ranges(&self, url: &Url, position: Position) -> Option<Vec<Range>> {
    //     if let Some(document) = self.documents.get(url.path()) {
    //         if let Some(token) = document.get_token_at_position(position) {
    //             let result = document
    //                 .get_all_tokens_by_single_name(&token.name)
    //                 .unwrap()
    //                 .iter()
    //                 .map(|token| token.range)
    //                 .collect();

    //             return Some(result);
    //         }
    //     }

    //     None
    // }

    // pub fn get_token_definition_response(
    //     &self,
    //     url: Url,
    //     position: Position,
    // ) -> Option<GotoDefinitionResponse> {
    //     let key = url.path();

    //     if let Some(document) = self.documents.get(key) {
    //         if let Some(token) = document.get_token_at_position(position) {
    //             if token.is_initial_declaration() {
    //                 return Some(capabilities::go_to::to_definition_response(url, token));
    //             } else {
    //                 for document_ref in &self.documents {
    //                     if let Some(declared_token) = document_ref.get_declared_token(&token.name) {
    //                         return match Url::from_file_path(document_ref.key()) {
    //                             Ok(url) => Some(capabilities::go_to::to_definition_response(
    //                                 url,
    //                                 declared_token,
    //                             )),
    //                             Err(_) => None,
    //                         };
    //                     }
    //                 }
    //             }
    //         }
    //     }

    //     None
    // }

    // pub fn get_completion_items(&self, url: &Url) -> Option<Vec<CompletionItem>> {
    //     if let Some(document) = self.documents.get(url.path()) {
    //         return Some(capabilities::completion::to_completion_items(
    //             document.get_tokens(),
    //         ));
    //     }

    //     None
    // }

    // pub fn get_semantic_tokens(&self, url: &Url) -> Option<Vec<SemanticToken>> {
    //     if let Some(document) = self.documents.get(url.path()) {
    //         return Some(capabilities::semantic_tokens::to_semantic_tokes(
    //             document.get_tokens(),
    //         ));
    //     }

    //     None
    // }

    // pub fn get_symbol_information(&self, url: &Url) -> Option<Vec<SymbolInformation>> {
    //     if let Some(document) = self.documents.get(url.path()) {
    //         return Some(capabilities::document_symbol::to_symbol_information(
    //             document.get_tokens(),
    //             url.clone(),
    //         ));
    //     }

    //     None
    // }

    // pub fn format_text(&self, url: &Url) -> Option<Vec<TextEdit>> {
    //     if let Some(document) = self.documents.get(url.path()) {
    //         match self.config.read() {
    //             std::sync::LockResult::Ok(config) => {
    //                 let config: SwayConfig = *config;
    //                 get_format_text_edits(Arc::from(document.get_text()), config.into())
    //             }
    //             _ => None,
    //         }
    //     } else {
    //         None
    //     }
    // }
}
