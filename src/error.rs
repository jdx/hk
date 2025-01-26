use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub enum Error {
    #[error(transparent)]
    GlobSet(#[from] globset::Error),
    #[error(transparent)]
    Git(#[from] git2::Error),
    #[error(transparent)]
    XX(#[from] xx::XXError),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Ensember(#[from] ensembler::Error),
    #[error(transparent)]
    Tera(#[from] tera::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
