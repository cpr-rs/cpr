use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectInitError {
    #[error("Project directory already exists")]
    ProjectDirExists,
    
    #[error("Failed to create project directory")]
    ProjectDirCreateFailed,

    #[error("Failed to clone repository")]
    GitCloneFailed,
}
