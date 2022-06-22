use std::{io, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not create a sway build config from manifest")]
    BuildConfig,

    #[error("no manifest file found")]
    NoManifestFile,
}

#[derive(Debug, Error)]
pub enum DocumentError {
    #[error("document not found")]
    DocumentNotFound,

    #[error("document already stored")]
    DocumentAlreadyStored,
}
