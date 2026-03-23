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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_error_kinds() {
        assert_eq!(from_error(&TaskspaceError::Usage("x".to_string())), 2);
        assert_eq!(from_error(&TaskspaceError::Conflict("x".to_string())), 3);
        assert_eq!(from_error(&TaskspaceError::NotFound("x".to_string())), 4);
        assert_eq!(from_error(&TaskspaceError::Io("x".to_string())), 5);
        assert_eq!(
            from_error(&TaskspaceError::ExternalCommand("x".to_string())),
            5
        );
        assert_eq!(from_error(&TaskspaceError::Corrupt("x".to_string())), 6);
        assert_eq!(from_error(&TaskspaceError::Internal("x".to_string())), 1);
    }
}
