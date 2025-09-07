use std::borrow::Cow;

use crate::wren::RawWren;

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

impl<'s> Value<'s> {
    pub fn is_owned(&self) -> bool {
        matches!(self, Value::Null | Value::Bool(_) | Value::Num(_))
    }
}

pub struct IntoWrenError;

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

impl IntoWren for () {
    fn into_slot(self, wren: &mut RawWren, slot: usize) -> Result<(), IntoWrenError> {
        Value::Null.into_slot(wren, slot)
    }
}

impl IntoWren for bool {
    fn into_slot(self, wren: &mut RawWren, slot: usize) -> Result<(), IntoWrenError> {
        Value::Bool(self).into_slot(wren, slot)
    }
}

impl IntoWren for f64 {
    fn into_slot(self, wren: &mut RawWren, slot: usize) -> Result<(), IntoWrenError> {
        Value::Num(self).into_slot(wren, slot)
    }
}

impl IntoWren for &[u8] {
    fn into_slot(self, wren: &mut RawWren, slot: usize) -> Result<(), IntoWrenError> {
        Value::String(self).into_slot(wren, slot)
    }
}

impl IntoWren for &str {
    fn into_slot(self, wren: &mut RawWren, slot: usize) -> Result<(), IntoWrenError> {
        Value::String(self.as_bytes()).into_slot(wren, slot)
    }
}

pub struct FromWrenError;

pub trait FromWren<'a>: Sized + sealed::Sealed + 'a {
    fn from_slot(wren: &'a RawWren, slot: usize) -> Result<Self, FromWrenError>;
}

impl<'a> FromWren<'a> for Value<'a> {
    fn from_slot(wren: &'a RawWren, slot: usize) -> Result<Self, FromWrenError> {
        wren.get_slot(slot).ok_or(FromWrenError)
    }
}

impl FromWren<'_> for () {
    fn from_slot(wren: &RawWren, slot: usize) -> Result<Self, FromWrenError> {
        match Value::from_slot(wren, slot)? {
            Value::Null => Ok(()),
            _ => Err(FromWrenError),
        }
    }
}

impl FromWren<'_> for bool {
    fn from_slot(wren: &RawWren, slot: usize) -> Result<Self, FromWrenError> {
        match Value::from_slot(wren, slot)? {
            Value::Bool(b) => Ok(b),
            _ => Err(FromWrenError),
        }
    }
}

impl FromWren<'_> for f64 {
    fn from_slot(wren: &RawWren, slot: usize) -> Result<Self, FromWrenError> {
        match Value::from_slot(wren, slot)? {
            Value::Num(v) => Ok(v),
            _ => Err(FromWrenError),
        }
    }
}

impl<'a> FromWren<'a> for &'a [u8] {
    fn from_slot(wren: &'a RawWren, slot: usize) -> Result<Self, FromWrenError> {
        match Value::from_slot(wren, slot)? {
            Value::String(s) => Ok(s),
            _ => Err(FromWrenError),
        }
    }
}

impl<'a> FromWren<'a> for &'a str {
    fn from_slot(wren: &'a RawWren, slot: usize) -> Result<Self, FromWrenError> {
        let v = <&[u8] as FromWren>::from_slot(wren, slot)?;

        std::str::from_utf8(v).map_err(|_| FromWrenError)
    }
}

impl<'a> FromWren<'a> for Cow<'a, str> {
    fn from_slot(wren: &'a RawWren, slot: usize) -> Result<Self, FromWrenError> {
        let v = <&[u8]>::from_slot(wren, slot)?;

        Ok(String::from_utf8_lossy(v))
    }
}

impl<'a> FromWren<'a> for String {
    fn from_slot(wren: &'a RawWren, slot: usize) -> Result<Self, FromWrenError> {
        Ok(Cow::<'_, str>::from_slot(wren, slot)?.to_string())
    }
}

mod sealed {
    pub trait Sealed {}

    impl Sealed for super::Value<'_> {}

    impl Sealed for () {}
    impl Sealed for bool {}
    impl Sealed for f64 {}

    impl Sealed for &[u8] {}
    impl Sealed for &str {}
    impl Sealed for std::borrow::Cow<'_, str> {}
    impl Sealed for String {}
}
