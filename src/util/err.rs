/// Converts errors from their error type (of the submodule) to that of
/// an aaru::Error variant.
///
/// ```rust
/// use aaru::{CodecError};
/// aaru::impl_err!(CodecError, Codec);
/// ```
pub mod err_macro {
    macro_rules! impl_err {
        ($from:ty, $variant:ident) => {
            use aaru::Error;

            impl From<$from> for Error {
                fn from(value: $from) -> Self {
                    Error::$variant(value)
                }
            }
        };
    }

    pub(crate) use impl_err;
}
