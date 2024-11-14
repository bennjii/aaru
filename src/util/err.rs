/// Converts errors from their error type (of the submodule) to that of
/// an aaru::Error variant.
///
/// ```rust,ignore
/// use aaru::codec::error::CodecError;
/// aaru::impl_err!(CodecError, Codec);
/// ```
pub mod err_macro {
    #[macro_export]
    macro_rules! impl_err {
        ($from:ty, $to:ident, $variant:ident) => {
            impl From<$from> for $to {
                fn from(value: $from) -> Self {
                    $crate::$to::$variant(value)
                }
            }
        };
        ($from:ty, $variant:ident) => {
            impl From<$from> for $crate::Error {
                fn from(value: $from) -> Self {
                    $crate::Error::$variant(value)
                }
            }
        };
    }

    pub use impl_err;
}
