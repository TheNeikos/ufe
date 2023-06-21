# User Facing Errors

This crate allows you to progressively enhance your error chains for
user-friendly errors.

## Getting Started

You start with the usual `std::error::Error` based program (Note: crates like
`anyhow` will also work here).

Then, add both `ufe` as well as `linkme` to your dependencies:

```bash
cargo add ufe
cargo add linkme
```

Then, anytime you want to print an error, you use one of the methods found in
[`crate::render`]. Most often that is going to be `render_to_terminal`.

You call it with a `&dyn std::error::Error`, and it will automatically traverse
the error chain.


The next step would be to implement [`AsUserFacingError`] and return an
user-friendly error. See below for some tips and tricks on how such errors
could look like.

```rust
use ufe::{
    AsUserFacingError, 
    ErrorCause, 
    PotentiallyUnclearError,
    UFEContext,
    UserFacingError, 
};

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
```

You will then need to register that this is an UserFacingError, this is how its done:

```rust
# use ufe::{
#     AsUserFacingError, 
#     ErrorCause, 
#     PotentiallyUnclearError,
#     UFEContext,
#     UFEConverter,
#     UserFacingError, 
# };
# #[derive(Debug, thiserror::Error)]
# #[error("Could not read the frobnicate")]
# struct SpecialError {
#     error: std::io::Error,
# }
# 
# impl AsUserFacingError for SpecialError {
#     fn as_user_facing_error(&self, ctx: &UFEContext) -> UserFacingError {
#         UserFacingError {
#             error: ErrorCause::default()
#                 .summary(self.to_string())
#                 .extended_reason("When you try to frobnicate, please ensure that the frub is available during the whole operation.".to_string()),
#             related: vec![PotentiallyUnclearError::from_error(&self.error).as_user_facing_error(ctx)],
#         }
#     }
# }
#[linkme::distributed_slice(ufe::UFE_SUPPORTED)]
pub static SPECIAL_ERROR: UFEConverter = UFEConverter::for_ufe::<SpecialError>();


fn main() {
    // Now, `SPECIAL_ERROR` is in `UFE_SUPPORTED` and can be used when
    // generating user-facing errors
    // You cannot directly access it, but since nothing is registered by
    // default, we only have one
    assert_eq!(ufe::UFE_SUPPORTED.len(), 1);
}
```

Now, the good part is, this works cross-crate!
Either:

- A dependency uses `ufe` and has registered their own converter, which you can then use
- A dependency does _not_ use `ufe`, but you can register your own [`UFEConverter`] with [`UFEConverter::custom`]. Check the method for details.
