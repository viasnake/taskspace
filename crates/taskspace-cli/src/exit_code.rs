use taskspace_core::TaskspaceError;

pub fn from_error(err: &TaskspaceError) -> i32 {
    match err {
        TaskspaceError::Usage(_) => 2,
        TaskspaceError::Conflict(_) => 3,
        TaskspaceError::NotFound(_) => 4,
        TaskspaceError::Io(_) | TaskspaceError::ExternalCommand(_) => 5,
        TaskspaceError::Corrupt(_) => 6,
        TaskspaceError::Internal(_) => 1,
    }
}
