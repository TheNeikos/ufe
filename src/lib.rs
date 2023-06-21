#![deny(
    unreachable_pub,
    unsafe_code,
    missing_docs,
    missing_debug_implementations
)]
#![doc = include_str!("../README.md")]

use std::{marker::PhantomData, ops::Range};

/// All the methods to render an [`UserFacingError`]
pub mod render;

/// Types that can be represented as [`UserFacingError`]s
pub trait AsUserFacingError {
    /// Turn the instance into an [`UserFacingError`]
    fn as_user_facing_error(&self, ctx: &UFEContext) -> UserFacingError;
}

/// Internal Information for generating [`UserFacingError`]s
pub struct UFEContext {
    _private: PhantomData<()>,
}

impl std::fmt::Debug for UFEContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UFEContext").finish_non_exhaustive()
    }
}

#[derive(Debug)]
/// An Object that can turn an unknown [`std::error::Error`] into a potential [`UserFacingError`].
///
/// There are two ways to create an [`UFEConverter`]:
///
/// - [`UFEConverter::for_ufe`] which is the safest way for types that implement
///   [`AsUserFacingError`]
/// - [`UFEConverter::custom`] which allows you to add an implementation for types that do not
///   support it per default
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
    /// Create a converter for a type that implements [`AsUserFacingError`]
    pub const fn for_ufe<T: AsUserFacingError + std::error::Error + 'static>() -> Self {
        UFEConverter {
            convert: convert::<T>,
        }
    }

    /// Create a converter for any kind of [`std::error::Error`]
    ///
    /// The given function can be used to downcast to a specific type.
    /// So for example, if another crate has an error you wish to embellish or display differently,
    /// you could downcast it here with `std::error::Error::downcast_ref` and then create a
    /// [`UserFacingError`].
    ///
    /// # Warning
    ///
    /// Do **not** simply always return a [`Some`] as this will cause it to be overriden for every
    /// single error type! Probably not what you want.
    pub const fn custom(
        f: fn(e: &(dyn std::error::Error + 'static), ctx: &UFEContext) -> Option<UserFacingError>,
    ) -> Self {
        UFEConverter { convert: f }
    }
}

/// The global slice of all known [`UFEConverter`]s that will be used during error generation
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
    /// The cause of the error
    pub error: ErrorCause,
    /// Errors that are related by either being the source or adjacent to this error
    pub related: Vec<UserFacingError>,
}

/// A label in a piece of text, shown to the user
#[derive(Debug)]
pub struct FileLabel {
    /// The byte-indexed slice where this label gets applied to
    pub range: Range<usize>,
    /// The message shown to the user
    pub message: String,
}

#[derive(Debug)]
/// A file and labels that highlight parts of it
pub struct FileHighlight {
    /// The path where the file was found
    pub path: String,
    /// The content of the file when it errored
    pub content: String,
    /// The list of labels that indicate the error
    pub labels: Vec<FileLabel>,
}

#[derive(Debug, Default, derive_setters::Setters)]
#[setters(strip_option)]
#[non_exhaustive]
/// The cause of an error
///
/// # Note
///
/// This struct is annotated to be extended in the future. You should only construct it by using
/// the default constructor [`ErrorCause::default`]. You should then customize it by using the
/// provided setters.
pub struct ErrorCause {
    /// A succinct explanation of what went wrong
    ///
    /// Aim to be precise but do not use overly specific language. This summary will always be
    /// shown to users.
    pub summary: String,
    /// An extended explanation of why the error (might) have happened. Try to not speculate. If
    /// there are things you can check during construction of the error, do so.
    pub extended_reason: Option<String>,
    /// If one or multiple files are associated to this error, mark them here.
    pub file_highlights: Vec<FileHighlight>,
}

/// A helper struct to turn any reference to an [`std::error::Error`] into either the best
/// [`UserFacingError`] possible or, if it has been previously registered into [`static@UFE_SUPPORTED`], then it will
/// use that converter.
///
/// # Note
///
/// Currently the only kind of reference that is supported is a somewhat unwieldy
/// `&(dyn std::error::Error + 'static)`. This is the only kind of construct that allows you to
/// downcast errors in Rust. You will find it for example in the return type of
/// [`std::error::Error::source`]. Generally, when you have a concrete type `E` a normal reference
/// to it _is_ going to work.
#[derive(Debug)]
pub struct PotentiallyUnclearError<E>(E);

impl<'a> PotentiallyUnclearError<&'a (dyn std::error::Error + 'static)> {
    /// Create a potentially unclear error from a type that implements [`std::error::Error`]
    ///
    /// This will automatically pick the correct UserFacingError implementation if it has been
    /// registered
    pub fn from_error<E: std::error::Error + 'static>(e: &'a E) -> Self {
        Self(e as &dyn std::error::Error)
    }
}

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
