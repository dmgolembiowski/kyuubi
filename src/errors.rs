use thiserror::Error;

#[derive(Error, Clone, Debug)]
pub enum KyuubiErr {
    #[error("The path does not exist: {0}")]
    DbPathNotExist(String),

    #[error("Unable to read database file: {0}")]
    DbReadError(String),

    #[error("File system error: {0}")]
    WriteError(String),

    #[error("Error encountered opening sled db from library call: {0}")]
    SledError(#[from] sled::Error),
}
