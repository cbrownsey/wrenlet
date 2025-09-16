#![allow(private_interfaces)]

use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use ::sealed::sealed;

use crate::{
    error::Error,
    raw::{HandlePtr, WrenPtr},
};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Value<'s> {
    #[default]
    Null,
    Bool(bool),
    Num(f64),
    String(&'s [u8]),
}

pub struct Handle(WrenPtr, HandlePtr);

#[sealed]
impl IntoWren for Handle {
    fn into_wren(&self, wren: &WrenPtr, slot: usize) -> Result<(), Error> {
        assert_eq!(self.0, *wren);

        unsafe { wren.set_slot_handle(slot, self.1) };

        Ok(())
    }
}

#[sealed]
impl FromWren<'_> for Handle {
    fn from_wren(wren: &WrenPtr, slot: usize) -> Result<Self, Error> {
        match unsafe { wren.get_slot_type(slot)}

        let ptr = unsafe { wren.get_slot_handle(slot) };


    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        todo!()
    }
}

#[sealed]
pub trait FromWren<'s>: Sized {
    fn from_wren(wren: &'s WrenPtr, slot: usize) -> Result<Self, Error>;
}

#[sealed]
impl<'s> FromWren<'s> for Value<'s> {
    fn from_wren(wren: &'s WrenPtr, slot: usize) -> Result<Self, Error> {
        assert!(wren.get_slot_count() > slot);

        match unsafe { wren.get_slot_type(slot) } {
            crate::raw::WrenType::Null => Ok(Value::Null),
            crate::raw::WrenType::Bool => {
                let value = unsafe { wren.get_slot_bool(slot) };

                Ok(Value::Bool(value))
            }
            crate::raw::WrenType::Num => {
                let value = unsafe { wren.get_slot_double(slot) };

                Ok(Value::Num(value))
            }
            crate::raw::WrenType::String => {
                let value = unsafe { wren.get_slot_string(slot) };

                Ok(Value::String(unsafe { &*value }))
            }
            crate::raw::WrenType::List => todo!(),
            crate::raw::WrenType::Map => todo!(),
            crate::raw::WrenType::Unknown => todo!(),
            crate::raw::WrenType::Foreign => todo!(),
        }
    }
}

#[sealed]
impl FromWren<'_> for () {
    fn from_wren(wren: &WrenPtr, slot: usize) -> Result<Self, Error> {
        let Value::Null = Value::from_wren(wren, slot)? else {
            todo!()
        };

        Ok(())
    }
}

#[sealed]
impl FromWren<'_> for bool {
    fn from_wren(wren: &WrenPtr, slot: usize) -> Result<Self, Error> {
        let Value::Bool(value) = Value::from_wren(wren, slot)? else {
            todo!()
        };

        Ok(value)
    }
}

#[sealed]
impl FromWren<'_> for f64 {
    fn from_wren(wren: &'_ WrenPtr, slot: usize) -> Result<Self, Error> {
        let Value::Num(value) = Value::from_wren(wren, slot)? else {
            todo!();
        };

        Ok(value)
    }
}

#[sealed]
impl<'s> FromWren<'s> for &'s [u8] {
    fn from_wren(wren: &'s WrenPtr, slot: usize) -> Result<Self, Error> {
        let Value::String(value) = Value::from_wren(wren, slot)? else {
            todo!();
        };

        Ok(value)
    }
}

#[sealed]
impl<'s> FromWren<'s> for &'s str {
    fn from_wren(wren: &'s WrenPtr, slot: usize) -> Result<Self, Error> {
        let bytes = <&[u8]>::from_wren(wren, slot)?;

        Ok(std::str::from_utf8(bytes).unwrap())
    }
}

#[sealed]
impl<'s> FromWren<'s> for std::borrow::Cow<'s, str> {
    fn from_wren(wren: &'s WrenPtr, slot: usize) -> Result<Self, Error> {
        let bytes = <&[u8]>::from_wren(wren, slot)?;

        Ok(String::from_utf8_lossy(bytes))
    }
}

#[sealed]
impl<'s> FromWren<'s> for String {
    fn from_wren(wren: &'s WrenPtr, slot: usize) -> Result<Self, Error> {
        let text = std::borrow::Cow::from_wren(wren, slot)?;

        Ok(text.to_string())
    }
}

#[sealed]
pub trait IntoWren {
    fn into_wren(&self, wren: &WrenPtr, slot: usize) -> Result<(), Error>;
}

#[sealed]
impl IntoWren for () {
    fn into_wren(&self, wren: &WrenPtr, slot: usize) -> Result<(), Error> {
        unsafe { wren.ensure_slots(slot + 1) };

        unsafe { wren.set_slot_null(slot) };

        Ok(())
    }
}

#[sealed]
impl IntoWren for bool {
    fn into_wren(&self, wren: &WrenPtr, slot: usize) -> Result<(), Error> {
        unsafe { wren.ensure_slots(slot + 1) };

        unsafe { wren.set_slot_bool(slot, *self) };

        Ok(())
    }
}

#[sealed]
impl IntoWren for f64 {
    fn into_wren(&self, wren: &WrenPtr, slot: usize) -> Result<(), Error> {
        unsafe { wren.ensure_slots(slot + 1) };

        unsafe { wren.set_slot_double(slot, *self) };

        Ok(())
    }
}

#[sealed]
impl IntoWren for [u8] {
    fn into_wren(&self, wren: &WrenPtr, slot: usize) -> Result<(), Error> {
        unsafe { wren.ensure_slots(slot + 1) };

        unsafe { wren.set_slot_bytes(slot, self) };

        Ok(())
    }
}

#[sealed]
impl IntoWren for str {
    fn into_wren(&self, wren: &WrenPtr, slot: usize) -> Result<(), Error> {
        self.as_bytes().into_wren(wren, slot)
    }
}

/// A tuple of values which can be passed into a Wren function call.
///
/// This trait is implemented for tuples of length one to sixteen inclusive.
/// Passing no arguments is not supported, as all Wren methods require a
/// reciever class.
#[sealed]
pub trait WrenArguments {}

#[sealed]
impl<A: IntoWren> WrenArguments for (A,) {}

#[sealed]
impl<A: IntoWren, B: IntoWren> WrenArguments for (A, B) {}

#[sealed]
impl<A: IntoWren, B: IntoWren, C: IntoWren> WrenArguments for (A, B, C) {}

#[sealed]
impl<A: IntoWren, B: IntoWren, C: IntoWren, D: IntoWren> WrenArguments for (A, B, C, D) {}

#[sealed]
impl<A: IntoWren, B: IntoWren, C: IntoWren, D: IntoWren, E: IntoWren> WrenArguments
    for (A, B, C, D, E)
{
}

#[sealed]
impl<A: IntoWren, B: IntoWren, C: IntoWren, D: IntoWren, E: IntoWren, F: IntoWren> WrenArguments
    for (A, B, C, D, E, F)
{
}

#[sealed]
impl<A: IntoWren, B: IntoWren, C: IntoWren, D: IntoWren, E: IntoWren, F: IntoWren, G: IntoWren>
    WrenArguments for (A, B, C, D, E, F, G)
{
}

#[sealed]
impl<
    A: IntoWren,
    B: IntoWren,
    C: IntoWren,
    D: IntoWren,
    E: IntoWren,
    F: IntoWren,
    G: IntoWren,
    H: IntoWren,
> WrenArguments for (A, B, C, D, E, F, G, H)
{
}
