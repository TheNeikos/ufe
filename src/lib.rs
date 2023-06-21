use std::{marker::PhantomData, ops::Range};

pub mod render;

pub trait AsUserFacingError {
    fn as_user_facing_error(&self, ctx: &UFEContext) -> UserFacingError;
}

pub struct UFEContext {
    _private: PhantomData<()>,
}

pub struct UFEConverter {
    convert: fn(&(dyn std::error::Error + 'static), ctx: &UFEContext) -> Option<UserFacingError>,
}

fn convert<T: AsUserFacingError + std::error::Error + 'static>(
    e: &(dyn std::error::Error + 'static),
    ctx: &UFEContext,
) -> Option<UserFacingError> {
    if let Some(e) = (&*e).downcast_ref::<T>() {
        Some(e.as_user_facing_error(ctx))
    } else {
        None
    }
}

impl UFEConverter {
    pub const fn for_ufe<T: AsUserFacingError + std::error::Error + 'static>() -> Self {
        UFEConverter {
            convert: convert::<T>,
        }
    }

    pub const fn custom(
        f: fn(&(dyn std::error::Error + 'static), ctx: &UFEContext) -> Option<UserFacingError>,
    ) -> Self {
        UFEConverter { convert: f }
    }
}

#[linkme::distributed_slice]
pub static UFE_SUPPORTED: [UFEConverter] = [..];

/// An error with all information contained to inform users
///
/// A `UserFacingError` is _not_ a [`std::error::Error`]. As it contains information meant to be
/// consumed by an enduser. The primary use-case is that application authors can be more in control
/// of their error story.
///
/// The typical way this error is constructed is with [`AsUserFacingError`]. Developers are the
/// encouraged to provide as much information as possible that can be useful to users on how to
/// resolve or otherwise interpret this error.
///
/// Nested errors can be handled by for example writing custom error wrappers. One such example is
/// [`PotentiallyUnclearError`], which is meant as a last resort 'give something' to users, but can
/// be, as the name indicates, not particularly helpful. A better path would be to: Simply not tell
/// about the underlying error and instead say directly how to fix it by looking at what the error
/// represents.
///
/// For example, if your application reads a configuration file and you simply expose any IO Errors
/// to your user they will have to understand system internas and how _you_ the application
/// developer interact with the system. The better way would be to generate a [`UserFacingError`]
/// and instead of directly printing the io error, you generate an appropriate response as to:
/// 1. Summary of what the error is -> "Could not read the configuration file"
/// 2. How it occured -> "On startup, the configuration is read from the path given on the command
///    line."
/// 3. How it can be fixed
///     - If its an NOT_FOUND -> "The configuration file at path {absolute_path}, could not be found. Please
///     ensure the file is at the given path."
///     - If its an ACCESS_DENIED -> "The configuration file was found but could not be read as
///     access was denied upon reading it."
///         - If file owner != app user -> "The file is owned by {file_user}, the application was run as
///           {my_user}. Please ensure that the user with which the application is run has the rights to
///           access files by {file_user}. Otherwise, change the owner of the file to align it with the
///           application user."
///
/// This approach is more work, but leads to more informative errors for end users.
#[derive(Debug)]
pub struct UserFacingError {
    pub error: ErrorCause,
    pub related: Vec<UserFacingError>,
}

#[derive(Debug)]
pub struct FileLabel {
    pub range: Range<usize>,
    pub message: String,
}

#[derive(Debug)]
pub struct FileHighlight {
    pub path: String,
    pub content: String,
    pub labels: Vec<FileLabel>,
}

#[derive(Debug, Default, derive_setters::Setters)]
#[setters(strip_option)]
#[non_exhaustive]
pub struct ErrorCause {
    pub summary: String,
    pub extended_reason: Option<String>,
    pub file_highlights: Vec<FileHighlight>,
}

#[derive(Debug)]
pub struct PotentiallyUnclearError<E>(pub E);

impl AsUserFacingError for PotentiallyUnclearError<&(dyn std::error::Error + 'static)> {
    fn as_user_facing_error(&self, ctx: &UFEContext) -> UserFacingError {
        for conv in UFE_SUPPORTED {
            if let Some(ufe) = (conv.convert)(self.0, ctx) {
                return ufe;
            }
        }

        UserFacingError {
            error: ErrorCause::default().summary(self.0.to_string()),
            related: self
                .0
                .source()
                .map(|e| PotentiallyUnclearError(e).as_user_facing_error(ctx))
                .into_iter()
                .collect(),
        }
    }
}

impl<E: std::error::Error> From<E> for PotentiallyUnclearError<E> {
    fn from(value: E) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::{AsUserFacingError, UFEContext, UFEConverter};

    #[allow(dead_code)]
    fn check_converting_compiles() {
        fn get_ctx() -> &'static UFEContext {
            todo!()
        }

        #[derive(Debug, thiserror::Error)]
        #[error("Foobar")]
        struct Foobar;

        impl AsUserFacingError for Foobar {
            fn as_user_facing_error(&self, _ctx: &crate::UFEContext) -> crate::UserFacingError {
                crate::UserFacingError {
                    error: crate::ErrorCause::default(),
                    related: vec![],
                }
            }
        }

        let conv = UFEConverter::for_ufe::<Foobar>();

        let e: &dyn Error = &Foobar;

        let ctx = get_ctx();
        (conv.convert)(e.source().unwrap(), ctx).unwrap();
    }
}
