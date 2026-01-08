use crate::Wren;
use crate::error::{FromWrenError, IntoWrenError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Type {
    #[default]
    Null,
    Bool,
    Num,
    String,
    List,
    Map,
    Foreign,

    Unknown,
}

pub trait FromWren<U, M, W>: Sized {
    const SCRATCH_SPACE: usize;

    fn from_wren(wren: &mut Wren<U, M, W>, slot: usize) -> Result<Self, FromWrenError>;
}

/// Writes a value into the given slot in a [`Wren`] instance.
///
/// [`Wren`]: crate::Wren
pub trait IntoWren<U, M, W> {
    /// The number of additional scratch spaces required to put this value into Wren.
    ///
    /// This does not include the final slot. Writing an `f64` into wren does not require any
    /// additional scratch slots beyond the target slot.
    const SCRATCH_SPACE: usize;

    fn into_wren(&self, wren: &mut Wren<U, M, W>, slot: usize) -> Result<(), IntoWrenError>;
}

impl<U, M, W> FromWren<U, M, W> for () {
    const SCRATCH_SPACE: usize = 0;

    fn from_wren(wren: &mut Wren<U, M, W>, slot: usize) -> Result<(), FromWrenError> {
        todo!()
    }
}

impl<U, M, W> IntoWren<U, M, W> for () {
    const SCRATCH_SPACE: usize = 0;

    fn into_wren(&self, wren: &mut Wren<U, M, W>, slot: usize) -> Result<(), IntoWrenError> {
        todo!()
    }
}

impl<U, M, W> FromWren<U, M, W> for bool {
    const SCRATCH_SPACE: usize = 0;

    fn from_wren(wren: &mut Wren<U, M, W>, slot: usize) -> Result<bool, FromWrenError> {
        todo!()
    }
}

impl<U, M, W> IntoWren<U, M, W> for bool {
    const SCRATCH_SPACE: usize = 0;

    fn into_wren(&self, wren: &mut Wren<U, M, W>, slot: usize) -> Result<(), IntoWrenError> {
        todo!()
    }
}

impl<U, M, W> FromWren<U, M, W> for f64 {
    const SCRATCH_SPACE: usize = 0;

    fn from_wren(wren: &mut Wren<U, M, W>, slot: usize) -> Result<f64, FromWrenError> {
        todo!()
    }
}

impl<U, M, W> IntoWren<U, M, W> for f64 {
    const SCRATCH_SPACE: usize = 0;

    fn into_wren(&self, wren: &mut Wren<U, M, W>, slot: usize) -> Result<(), IntoWrenError> {
        todo!()
    }
}

impl<T, U, M, W> FromWren<U, M, W> for Option<T>
where
    T: FromWren<U, M, W>,
{
    const SCRATCH_SPACE: usize = T::SCRATCH_SPACE;

    fn from_wren(wren: &mut Wren<U, M, W>, slot: usize) -> Result<Option<T>, FromWrenError> {
        todo!()
    }
}

impl<T, U, M, W> IntoWren<U, M, W> for Option<T>
where
    T: IntoWren<U, M, W>,
{
    const SCRATCH_SPACE: usize = T::SCRATCH_SPACE;

    fn into_wren(&self, wren: &mut Wren<U, M, W>, slot: usize) -> Result<(), IntoWrenError> {
        todo!()
    }
}
