use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectInitError {
    #[error("Project directory already exists")]
    ProjectDirExists,
    
    #[error("Failed to create project directory")]
    ProjectDirCreateFail,

    #[error("Failed to clone repository")]
    GitCloneFail,
    
    #[error("Git repository not found")]
    GitRepoNotFound,

    #[error("Failed to write file in template: {0}")]
    WriteFileFail(String),

    #[error("Failed to read file in template: {0}")]
    ReadFileFail(String),
}

#[derive(Debug, Error)]
pub enum CPRConfigError {
    #[error("Failed to read cpr.toml from template")]
    FileReadFail,

    #[error("Failed to parse cpr.toml from template")]
    TomlParseFail,
}
