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
    #[error(transparent)]
    Rpkl(#[from] rpkl::Error),
    #[error(transparent)]
    SerdeYaml(#[from] serde_yaml::Error),
    #[error("{0}")]
    Diagnostic(String),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    Miette(#[from] Box<dyn miette::Diagnostic + Send + Sync>),
}

pub type Result<T> = std::result::Result<T, Error>;

#[macro_export]
macro_rules! bail {
    ($($key:ident = $value:expr,)* $fmt:literal $($arg:tt)*) => {
        return Result::Err($crate::error::Error::Diagnostic(format!($fmt $($arg)*)));
    };
}
