//! Values which can be passed to or from [`Wren`].
//!
//! [`Wren`]: crate::Wren
#![allow(private_interfaces)]

use std::{
    any::TypeId,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use ::sealed::sealed;

use crate::{
    error::Error,
    raw::{HandlePtr, WrenPtr, WrenType},
    wren::WrenHeader,
};

#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum Value<'s> {
    #[default]
    Null,
    Bool(bool),
    Num(f64),
    String(&'s [u8]),
}

pub struct Handle(WrenPtr, HandlePtr);

#[sealed]
impl IntoWren for Handle {
    fn put_value(&self, wren: &WrenPtr, slot: usize) -> Result<(), Error> {
        assert_eq!(self.0, *wren);

        unsafe { wren.set_slot_handle(slot, self.1) };

        Ok(())
    }
}

#[sealed]
impl FromWren<'_> for Handle {
    fn get_value(wren: &WrenPtr, slot: usize) -> Result<Self, Error> {
        unsafe { wren.ensure_slots(slot + 1) };

        let handle = unsafe { wren.get_slot_handle(slot) };

        unsafe { WrenHeader::claim(wren.get_user_data()) };

        Ok(Handle(*wren, handle))
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { self.0.release_handle(self.1) };

        unsafe { WrenHeader::release(self.0.get_user_data()) };
    }
}

#[sealed]
pub trait FromWren<'s>: Sized {
    fn get_value(wren: &'s WrenPtr, slot: usize) -> Result<Self, Error>;
}

#[sealed]
impl<'s> FromWren<'s> for Value<'s> {
    fn get_value(wren: &'s WrenPtr, slot: usize) -> Result<Self, Error> {
        if slot >= wren.get_slot_count() {
            return Ok(Self::Null);
        }

        match unsafe { wren.get_slot_type(slot) } {
            WrenType::Null => Ok(Value::Null),
            WrenType::Bool => {
                let value = unsafe { wren.get_slot_bool(slot) };

                Ok(Value::Bool(value))
            }
            WrenType::Num => {
                let value = unsafe { wren.get_slot_double(slot) };

                Ok(Value::Num(value))
            }
            WrenType::String => {
                let value = unsafe { wren.get_slot_string(slot) };

                Ok(Value::String(unsafe { &*value }))
            }
            WrenType::List => todo!(),
            WrenType::Map => todo!(),
            WrenType::Unknown => todo!(),
            WrenType::Foreign => todo!(),
        }
    }
}

#[sealed]
impl FromWren<'_> for () {
    fn get_value(wren: &WrenPtr, slot: usize) -> Result<Self, Error> {
        let Value::Null = Value::get_value(wren, slot)? else {
            todo!()
        };

        Ok(())
    }
}

#[sealed]
impl FromWren<'_> for bool {
    fn get_value(wren: &WrenPtr, slot: usize) -> Result<Self, Error> {
        let Value::Bool(value) = Value::get_value(wren, slot)? else {
            todo!()
        };

        Ok(value)
    }
}

#[sealed]
impl FromWren<'_> for f64 {
    fn get_value(wren: &'_ WrenPtr, slot: usize) -> Result<Self, Error> {
        let Value::Num(value) = Value::get_value(wren, slot)? else {
            todo!();
        };

        Ok(value)
    }
}

#[sealed]
impl<'s> FromWren<'s> for &'s [u8] {
    fn get_value(wren: &'s WrenPtr, slot: usize) -> Result<Self, Error> {
        let Value::String(value) = Value::get_value(wren, slot)? else {
            todo!();
        };

        Ok(value)
    }
}

#[sealed]
impl<'s> FromWren<'s> for &'s str {
    fn get_value(wren: &'s WrenPtr, slot: usize) -> Result<Self, Error> {
        let bytes = <&[u8]>::get_value(wren, slot)?;

        Ok(std::str::from_utf8(bytes).unwrap())
    }
}

#[sealed]
impl<'s> FromWren<'s> for std::borrow::Cow<'s, str> {
    fn get_value(wren: &'s WrenPtr, slot: usize) -> Result<Self, Error> {
        let bytes = <&[u8]>::get_value(wren, slot)?;

        Ok(String::from_utf8_lossy(bytes))
    }
}

#[sealed]
impl<'s> FromWren<'s> for String {
    fn get_value(wren: &'s WrenPtr, slot: usize) -> Result<Self, Error> {
        let text = std::borrow::Cow::get_value(wren, slot)?;

        Ok(text.to_string())
    }
}

#[sealed]
pub trait IntoWren {
    fn put_value(&self, wren: &WrenPtr, slot: usize) -> Result<(), Error>;
}

#[sealed]
impl IntoWren for () {
    fn put_value(&self, wren: &WrenPtr, slot: usize) -> Result<(), Error> {
        unsafe { wren.ensure_slots(slot + 1) };

        unsafe { wren.set_slot_null(slot) };

        Ok(())
    }
}

#[sealed]
impl IntoWren for bool {
    fn put_value(&self, wren: &WrenPtr, slot: usize) -> Result<(), Error> {
        unsafe { wren.ensure_slots(slot + 1) };

        unsafe { wren.set_slot_bool(slot, *self) };

        Ok(())
    }
}

#[sealed]
impl IntoWren for f64 {
    fn put_value(&self, wren: &WrenPtr, slot: usize) -> Result<(), Error> {
        unsafe { wren.ensure_slots(slot + 1) };

        unsafe { wren.set_slot_double(slot, *self) };

        Ok(())
    }
}

#[sealed]
impl IntoWren for [u8] {
    fn put_value(&self, wren: &WrenPtr, slot: usize) -> Result<(), Error> {
        unsafe { wren.ensure_slots(slot + 1) };

        unsafe { wren.set_slot_bytes(slot, self) };

        Ok(())
    }
}

#[sealed]
impl IntoWren for str {
    fn put_value(&self, wren: &WrenPtr, slot: usize) -> Result<(), Error> {
        self.as_bytes().put_value(wren, slot)
    }
}

#[sealed]
impl<T: IntoWren + ?Sized> IntoWren for &T {
    fn put_value(&self, wren: &WrenPtr, slot: usize) -> Result<(), Error> {
        (*self).put_value(wren, slot)
    }
}

/// A tuple of values which can be passed into a Wren function call.
///
/// This trait is implemented for tuples of length one to sixteen inclusive.
/// Passing no arguments is not supported, as all Wren methods require a
/// reciever class.
#[sealed]
pub trait WrenArguments {
    fn prepare(&self, wren: &WrenPtr) -> Result<(), Error>;
}

#[sealed]
impl WrenArguments for () {
    fn prepare(&self, _wren: &WrenPtr) -> Result<(), Error> {
        Ok(())
    }
}

#[sealed]
impl<A: IntoWren> WrenArguments for (A,) {
    fn prepare(&self, wren: &WrenPtr) -> Result<(), Error> {
        self.0.put_value(wren, 1)?;

        Ok(())
    }
}

#[sealed]
impl<A: IntoWren, B: IntoWren> WrenArguments for (A, B) {
    fn prepare(&self, wren: &WrenPtr) -> Result<(), Error> {
        self.0.put_value(wren, 1)?;
        self.1.put_value(wren, 2)?;

        Ok(())
    }
}

#[sealed]
impl<A: IntoWren, B: IntoWren, C: IntoWren> WrenArguments for (A, B, C) {
    fn prepare(&self, wren: &WrenPtr) -> Result<(), Error> {
        self.0.put_value(wren, 1)?;
        self.1.put_value(wren, 2)?;
        self.2.put_value(wren, 3)?;

        Ok(())
    }
}

#[sealed]
impl<A: IntoWren, B: IntoWren, C: IntoWren, D: IntoWren> WrenArguments for (A, B, C, D) {
    fn prepare(&self, wren: &WrenPtr) -> Result<(), Error> {
        self.0.put_value(wren, 1)?;
        self.1.put_value(wren, 2)?;
        self.2.put_value(wren, 3)?;
        self.3.put_value(wren, 4)?;

        Ok(())
    }
}

#[sealed]
impl<A: IntoWren, B: IntoWren, C: IntoWren, D: IntoWren, E: IntoWren> WrenArguments
    for (A, B, C, D, E)
{
    fn prepare(&self, wren: &WrenPtr) -> Result<(), Error> {
        self.0.put_value(wren, 1)?;
        self.1.put_value(wren, 2)?;
        self.2.put_value(wren, 3)?;
        self.3.put_value(wren, 4)?;
        self.4.put_value(wren, 5)?;

        Ok(())
    }
}

#[sealed]
impl<A: IntoWren, B: IntoWren, C: IntoWren, D: IntoWren, E: IntoWren, F: IntoWren> WrenArguments
    for (A, B, C, D, E, F)
{
    fn prepare(&self, wren: &WrenPtr) -> Result<(), Error> {
        self.0.put_value(wren, 1)?;
        self.1.put_value(wren, 2)?;
        self.2.put_value(wren, 3)?;
        self.3.put_value(wren, 4)?;
        self.4.put_value(wren, 5)?;
        self.5.put_value(wren, 6)?;

        Ok(())
    }
}

#[sealed]
impl<A: IntoWren, B: IntoWren, C: IntoWren, D: IntoWren, E: IntoWren, F: IntoWren, G: IntoWren>
    WrenArguments for (A, B, C, D, E, F, G)
{
    fn prepare(&self, wren: &WrenPtr) -> Result<(), Error> {
        self.0.put_value(wren, 1)?;
        self.1.put_value(wren, 2)?;
        self.2.put_value(wren, 3)?;
        self.3.put_value(wren, 4)?;
        self.4.put_value(wren, 5)?;
        self.5.put_value(wren, 6)?;
        self.6.put_value(wren, 7)?;

        Ok(())
    }
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
    fn prepare(&self, wren: &WrenPtr) -> Result<(), Error> {
        self.0.put_value(wren, 1)?;
        self.1.put_value(wren, 2)?;
        self.2.put_value(wren, 3)?;
        self.3.put_value(wren, 4)?;
        self.4.put_value(wren, 5)?;
        self.5.put_value(wren, 6)?;
        self.6.put_value(wren, 7)?;
        self.7.put_value(wren, 8)?;

        Ok(())
    }
}
