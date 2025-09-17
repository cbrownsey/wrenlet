//! The safe Wren interface.
//!
//! # Ownership
//! The responsibility of freeing the underlying [`WrenVM`] is managed by a
//! reference count stored in the [`WrenHeader`], and so [`wrenFreeVM`] may
//! ultimately be called by the drop implementation of [`Wren`] or any handle
//! object.
//!
//! # Safety
//! No reference to the associated userdata may be held while the Wren VM is
//! running code. To ensure this, code may only be run if a mutable reference
//! to the `Wren` is taken.
//!
//! The slot array may be modified at any time, even under an immutable
//! reference. Therefore, code may not assume that a slot's value will not
//! change.
//!
//! [`WrenVM`]: sys::WrenVM
//! [`wrenFreeVM`]: sys::wrenFreeVM

use std::{
    alloc::{Layout, handle_alloc_error},
    ffi::CString,
    io::Stdout,
    marker::PhantomData,
    mem::MaybeUninit,
};

use crate::{
    Builder,
    error::Error,
    module::Empty,
    raw::{HandlePtr, InterpretError, WrenPtr},
    value::{FromWren, IntoWren, WrenArguments},
};

/// An instance of a Wren virtual machine with associated user data.
pub struct Wren<U, M = Empty, W = Stdout>(WrenPtr, PhantomData<(U, M, W)>);

impl<U: std::fmt::Debug, M, W> std::fmt::Debug for Wren<U, M, W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Wren").field(self.user_data()).finish()
    }
}

impl Default for Wren<()> {
    fn default() -> Self {
        Self::new()
    }
}

impl Wren<()> {
    pub fn new() -> Wren<()> {
        Builder::new().build()
    }

    pub fn builder() -> Builder<(), Empty, Stdout> {
        Builder::new()
    }
}

impl<U, M, W> Wren<U, M, W> {
    pub(crate) unsafe fn from_ptr(ptr: *mut sys::WrenVM) -> Self {
        Self(unsafe { WrenPtr::from_raw(ptr.cast()) }, PhantomData)
    }

    /// Interprets the given `source` as Wren code in the context of the given `module`.
    pub fn interpret(&mut self, module: &str, source: &str) -> Result<(), Error> {
        let module = CString::new(module).unwrap();
        let source = CString::new(source).unwrap();

        let result = unsafe { self.0.interpret(&module, &source) };

        match result {
            Ok(()) => Ok(()),
            Err(InterpretError::Compile) => Err(Error::Compile),
            Err(InterpretError::Runtime) => Err(Error::Runtime),
        }
    }

    /// Creates a compiled call handle which can be used to invoke a method on some object.
    pub fn make_call_handle(&self, signature: &str) -> CallHandle {
        let signature = CString::new(signature).unwrap();

        let call_handle = self.0.make_call_handle(&signature);

        unsafe { WrenHeader::claim(self.header_ptr()) };

        CallHandle(self.0, call_handle)
    }

    pub fn call<'a, T>(
        &'a mut self,
        handle: CallHandle,
        reciever: impl IntoWren,
        args: impl WrenArguments,
    ) -> Result<T, Error>
    where
        T: FromWren<'a>,
    {
        assert_eq!(self.0, handle.0);

        reciever.put_value(&self.0, 0)?;

        args.prepare(&self.0)?;

        match unsafe { self.0.call(handle.1) } {
            Ok(()) => T::get_value(&self.0, 0),
            Err(crate::raw::InterpretError::Runtime) => todo!(),
            Err(crate::raw::InterpretError::Compile) => todo!(),
        }
    }

    pub fn get_variable<'s, T>(&'s self, module: &str, name: &str) -> Result<T, Error>
    where
        T: FromWren<'s>,
    {
        let module = CString::new(module).unwrap();
        let name = CString::new(name).unwrap();

        unsafe { self.0.ensure_slots(1) };
        unsafe { self.0.get_variable(&module, &name, 0) };

        T::get_value(&self.0, 0)
    }

    fn take_error(&mut self) -> Option<Error> {
        todo!()
    }

    fn error_mut(&mut self) -> &mut Option<Error> {
        todo!()
    }

    fn data_ptr(&self) -> *mut WrenData<U, M, W> {
        self.0.get_user_data::<WrenData<U, M, W>>()
    }

    /// Gets a reference to the userdata stored in the virtual machine.
    pub fn user_data(&self) -> &U {
        // Safety: The associated data remains valid until this `Wren` is dropped.
        &unsafe { WrenData::associated_init(self.data_ptr()) }.0
    }

    /// Gets a mutable reference to the userdata stored in the virtual machine.
    pub fn user_data_mut(&mut self) -> &mut U {
        // Safety: The associated data remains valid until this `Wren` is dropped.
        &mut unsafe { WrenData::associated_init_mut(self.data_ptr()) }.0
    }

    /// Gets a reference to the module loader used by this virtual machine.
    pub fn loader(&self) -> &M {
        // Safety: The associated data remains valid until this `Wren` is dropped.
        &unsafe { WrenData::associated_init(self.data_ptr()) }.1
    }

    /// Gets a mutable reference to the module loader used by this virtual machine.
    pub fn loader_mut(&mut self) -> &mut M {
        // Safety: The associated data remains valid until this `Wren` is dropped.
        &mut unsafe { WrenData::associated_init_mut(self.data_ptr()) }.1
    }

    /// Gets a reference to the output sink of this virtual machine.
    pub fn writer(&self) -> &W {
        // Safety: The associated data remains valid until this `Wren` is dropped.
        &unsafe { WrenData::associated_init(self.data_ptr()) }.2
    }

    /// Gets a mutable reference to the output sink of this virtual machine.
    pub fn writer_mut(&mut self) -> &mut W {
        // Safety: The associated data remains valid until this `Wren` is dropped.
        &mut unsafe { WrenData::associated_init_mut(self.data_ptr()) }.2
    }

    fn header_ptr(&self) -> *mut WrenHeader {
        self.0.get_user_data::<WrenHeader>()
    }

    unsafe fn header(&self) -> &WrenHeader {
        unsafe { &*self.0.get_user_data::<WrenHeader>() }
    }

    unsafe fn header_mut(&mut self) -> &mut WrenHeader {
        unsafe { &mut *self.0.get_user_data::<WrenHeader>() }
    }
}

impl<U, M, W> Drop for Wren<U, M, W> {
    fn drop(&mut self) {
        let ptr = self.data_ptr();

        unsafe { WrenData::drop_associated(ptr) };

        unsafe { WrenHeader::release(ptr.cast()) };
    }
}

/// A compiled identifier for a Wren method signature.
///
/// This object holds a reference to the Wren virtual machine, and so should be
/// dropped.
pub struct CallHandle(WrenPtr, HandlePtr);

impl Drop for CallHandle {
    fn drop(&mut self) {
        let ptr = self.0.get_user_data::<WrenHeader>();

        unsafe { WrenHeader::release(ptr) };
    }
}

#[repr(C)]
pub(crate) struct WrenData<U, M, W> {
    pub header: WrenHeader,
    pub associated: MaybeUninit<(U, M, W)>,
}

impl<U, M, W> WrenData<U, M, W> {
    pub fn allocate(user_data: U, loader: M, writer: W) -> *mut WrenData<U, M, W> {
        let layout = Layout::new::<Self>();

        let ptr = unsafe { std::alloc::alloc(layout) }.cast::<Self>();

        if ptr.is_null() {
            handle_alloc_error(layout);
        }

        unsafe {
            ptr.write(Self {
                header: WrenHeader::new(layout),
                associated: MaybeUninit::new((user_data, loader, writer)),
            })
        };

        ptr
    }

    pub unsafe fn header<'a>(this: *mut Self) -> &'a WrenHeader {
        unsafe { &(*this).header }
    }

    pub unsafe fn header_mut<'a>(this: *mut Self) -> &'a mut WrenHeader {
        unsafe { &mut (*this).header }
    }

    pub unsafe fn associated<'a>(this: *mut Self) -> &'a MaybeUninit<(U, M, W)> {
        unsafe { &(*this).associated }
    }

    pub unsafe fn associated_mut<'a>(this: *mut Self) -> &'a mut MaybeUninit<(U, M, W)> {
        unsafe { &mut (*this).associated }
    }

    pub unsafe fn associated_init<'a>(this: *mut Self) -> &'a (U, M, W) {
        unsafe { Self::associated(this).assume_init_ref() }
    }

    pub unsafe fn associated_init_mut<'a>(this: *mut Self) -> &'a mut (U, M, W) {
        unsafe { Self::associated_mut(this).assume_init_mut() }
    }

    pub unsafe fn drop_associated(this: *mut Self) {
        unsafe { Self::associated_mut(this).assume_init_drop() };
    }
}

pub(crate) struct WrenHeader {
    pub inner_layout: Layout,
    pub ref_count: usize,
    pub foreign_classes: Box<[crate::foreigns::ForeignMethod]>,
}

impl WrenHeader {
    pub fn new(inner_layout: Layout) -> WrenHeader {
        WrenHeader {
            inner_layout,
            ref_count: 1,
            foreign_classes: Box::from([]),
        }
    }

    pub unsafe fn release(this: *mut WrenHeader) {
        let ref_count = unsafe { WrenHeader::ref_count(this) };

        let ref_count = unsafe { ref_count.unchecked_sub(1) };

        unsafe { (*this).ref_count = ref_count };

        if ref_count == 0 {
            unsafe { std::ptr::drop_in_place(this) }

            let layout = unsafe { (*this).inner_layout };

            unsafe { std::alloc::dealloc(this.cast::<u8>(), layout) };
        }
    }

    pub unsafe fn claim(this: *mut WrenHeader) {
        let ref_count = unsafe { WrenHeader::ref_count(this) };

        let ref_count = ref_count.checked_add(1).unwrap();

        unsafe { (*this).ref_count = ref_count };
    }

    pub unsafe fn deallocate(this: *mut WrenHeader) {
        let layout = unsafe { (*this).inner_layout };

        let ptr = this.cast::<u8>();

        unsafe { std::alloc::dealloc(ptr, layout) };
    }

    pub unsafe fn ref_count(this: *mut WrenHeader) -> usize {
        unsafe { (*this).ref_count }
    }

    /// Increments the reference count in the given header.
    ///
    /// # Safety
    /// - `this` must point to a valid WrenHeader,
    pub unsafe fn increment_ref_count(this: *mut WrenHeader) {
        let ref_count = unsafe { WrenHeader::ref_count(this) };

        unsafe {
            (*this).ref_count = ref_count
                .checked_add(1)
                .expect("reference count in wren virtual machine overflowed.");
        }
    }

    /// Decrements the reference count in the given header.
    ///
    /// # Safety
    /// - `this` must point to a valid WrenHeader,
    /// - the underlying reference count must be greater than zero.
    pub unsafe fn decrement_ref_count(this: *mut WrenHeader) {
        let ref_count = unsafe { WrenHeader::ref_count(this) };

        unsafe { (*this).ref_count = ref_count.unchecked_sub(1) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_wren_data() {
        let data = WrenData::allocate((), (), Vec::<u8>::with_capacity(15));

        unsafe { WrenData::drop_associated(data) };

        unsafe { WrenHeader::release(data.cast()) };
    }
}
