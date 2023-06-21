use ufe::{AsUserFacingError, ErrorCause, PotentiallyUnclearError, UFEContext, UserFacingError};

#[derive(Debug, thiserror::Error)]
#[error("Could not read the frobnicate")]
struct SpecialError {
    error: std::io::Error,
}

impl AsUserFacingError for SpecialError {
    fn as_user_facing_error(&self, ctx: &UFEContext) -> UserFacingError {
        UserFacingError {
            error: ErrorCause::default()
                .summary(self.to_string())
                .extended_reason("When you try to frobnicate, please ensure that the frub is available during the whole operation.".to_string()),
            related: vec![PotentiallyUnclearError::from_error(&self.error).as_user_facing_error(ctx)],
        }
    }
}
