use thiserror::Error;
use tower_lsp::lsp_types::Diagnostic;

#[derive(Debug, Error)]
pub enum LspError {
    #[error("could not create a sway build config from manifest")]
    BuildConfig,

    #[error("no build config found")]
    BuildConfigNotFound,

    #[error("no manifest file found")]
    ManifestFileNotFound,

    #[error("document not found")]
    DocumentNotFound,

    #[error("document already stored")]
    DocumentAlreadyStored,

    #[error("Failed to parse typed AST")]
    FailedToParse(Vec<Diagnostic>),
}
