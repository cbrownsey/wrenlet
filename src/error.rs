//! Error values which may be returned by this library.

#[derive(Debug, Clone)]
pub enum Error {
    Runtime,
    Compile,
    MismatchedValue(MismatchedValueError),
}

#[derive(Debug, Clone)]
pub struct MismatchedValueError {
    expected: &'static [crate::raw::WrenType],
    found: crate::raw::WrenType,
}
