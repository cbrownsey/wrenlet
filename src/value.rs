use crate::wren::RawWren;

/// An error related to the handling of Wren values.
pub struct ValueError(ValueErrorInner);

#[derive(Debug, Clone, Copy)]
enum ValueErrorInner {
    MismatchedTypes {
        expected: &'static [ValueType],
        found: ValueType,
    },
}

/// A value borrowed from a [`Wren`] instance.
///
/// [`Wren`]: crate::Wren
#[derive(Debug, Clone, Copy, Default)]
pub enum Value<'s> {
    #[default]
    Null,
    Bool(bool),
    Num(f64),
    String(&'s [u8]),
    List,
    Map,
    Foreign,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub(crate) enum ValueType {
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

impl From<sys::WrenType> for ValueType {
    fn from(value: sys::WrenType) -> Self {
        match value {
            sys::WrenType::WREN_TYPE_NULL => ValueType::Null,
            sys::WrenType::WREN_TYPE_BOOL => ValueType::Bool,
            sys::WrenType::WREN_TYPE_NUM => ValueType::Num,
            sys::WrenType::WREN_TYPE_STRING => ValueType::String,
            sys::WrenType::WREN_TYPE_LIST => ValueType::List,
            sys::WrenType::WREN_TYPE_MAP => ValueType::Map,
            sys::WrenType::WREN_TYPE_FOREIGN => ValueType::Foreign,
            sys::WrenType::WREN_TYPE_UNKNOWN => ValueType::Unknown,
            _ => unreachable!(),
        }
    }
}

impl<'s> Value<'s> {
    fn type_of(&self) -> ValueType {
        match self {
            Value::Null => ValueType::Null,
            Value::Bool(_) => ValueType::Bool,
            Value::Num(_) => ValueType::Num,
            Value::String(_) => ValueType::String,
            Value::List => ValueType::List,
            Value::Map => ValueType::Map,
            Value::Foreign => ValueType::Foreign,
            Value::Unknown => ValueType::Unknown,
        }
    }

    pub fn is_owned(&self) -> bool {
        matches!(self, Value::Null | Value::Bool(_) | Value::Num(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntoWrenError;

impl std::fmt::Display for IntoWrenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "an error occurred storing a value into Wren.")
    }
}

impl std::error::Error for IntoWrenError {}

/// A value which can be inserted into a [`Wren`] instance.
///
/// [`Wren`]: crate::Wren
pub trait IntoWren: sealed::Sealed {
    fn into_slot(self, wren: &mut RawWren, slot: usize) -> Result<(), IntoWrenError>;
}

impl IntoWren for Value<'_> {
    fn into_slot(self, wren: &mut RawWren, slot: usize) -> Result<(), IntoWrenError> {
        wren.ensure_slots(slot + 1);

        unsafe { wren.set_slot(slot, self) };

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FromWrenError;

impl std::fmt::Display for FromWrenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "an error occurred retrieving a value from Wren.")
    }
}

impl std::error::Error for FromWrenError {}

/// A value which can be retrieved from a [`Wren`] instance.
///
/// [`Wren`]: crate::Wren
pub trait FromWren<'a>: Sized + sealed::Sealed {
    fn from_slot(wren: &'a RawWren, slot: usize) -> Result<Self, FromWrenError>;
}

impl<'a> FromWren<'a> for Value<'a> {
    fn from_slot(wren: &'a RawWren, slot: usize) -> Result<Self, FromWrenError> {
        Ok(wren.get_slot(slot).unwrap_or(Value::Null))
    }
}

mod sealed {
    pub trait Sealed {}

    impl Sealed for super::Value<'_> {}

    // impl Sealed for () {}
    // impl Sealed for bool {}
    // impl Sealed for f64 {}

    // impl Sealed for &[u8] {}
    // impl Sealed for &str {}
    // impl Sealed for std::borrow::Cow<'_, str> {}
    // impl Sealed for String {}
}
