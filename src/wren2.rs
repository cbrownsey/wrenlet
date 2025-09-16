//! The safe Wren interface.
//!
//! # Ownership
//! The responsibility of freeing the underlying [`WrenVM`] is managed by a
//! reference count stored in the [`WrenHeader`], and so [`wrenFreeVM`] may
//! ultimately be called by the drop implementation of [`Wren`] or any handle
//! object.
//!
//! [`WrenVM`]: sys::WrenVM
//! [`wrenFreeVM`]: sys::wrenFreeVM

use std::{alloc::Layout, ffi::CString, io::Stdout, marker::PhantomData, mem::MaybeUninit};

use crate::{
    Builder,
    error::Error,
    module::Empty,
    raw::{HandlePtr, InterpretError, WrenPtr},
    value::{FromWren, WrenArguments},
};

/// An instance of a Wren virtual machine with associated user data.
pub struct Wren<U, M = Empty, W = Stdout>(WrenPtr, PhantomData<(U, M, W)>);

impl<U, M, W> std::fmt::Debug for Wren<U, M, W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Wren")
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

        dbg!(call_handle);
        todo!()
    }

    pub fn call<T>(&mut self, handle: CallHandle, args: impl WrenArguments) -> Result<T, Error> {
        todo!()
    }

    pub fn get_variable<'s, T>(&'s self, module: &str, name: &str) -> Result<T, Error>
    where
        T: FromWren<'s>,
    {
        let module = CString::new(module).unwrap();
        let name = CString::new(name).unwrap();

        unsafe { self.0.ensure_slots(1) };
        unsafe { self.0.get_variable(&module, &name, 0) };

        T::from_wren(&self.0, 0)
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
        let data = unsafe { &(*self.data_ptr()).user_data };

        // Safety: The user data is only dropped when
        unsafe { data.assume_init_ref() }
    }

    /// Gets a mutable reference to the userdata stored in the virtual machine.
    pub fn user_data_mut(&mut self) -> &mut U {
        let data = unsafe { &mut (*self.data_ptr()).user_data };

        unsafe { data.assume_init_mut() }
    }

    /// Gets a reference to the module loader used by this virtual machine.
    pub fn loader(&self) -> &M {
        let data = unsafe { &(*self.data_ptr()).loader };

        unsafe { data.assume_init_ref() }
    }

    /// Gets a mutable reference to the module loader used by this virtual machine.
    pub fn loader_mut(&mut self) -> &mut M {
        let data = unsafe { &mut (*self.data_ptr()).loader };

        unsafe { data.assume_init_mut() }
    }

    /// Gets a reference to the output sink of this virtual machine.
    pub fn writer(&self) -> &W {
        let data = unsafe { &(*self.data_ptr()).writer };

        unsafe { data.assume_init_ref() }
    }

    /// Gets a mutable reference to the output sink of this virtual machine.
    pub fn writer_mut(&mut self) -> &mut W {
        let data = unsafe { &mut (*self.data_ptr()).writer };

        unsafe { data.assume_init_mut() }
    }

    fn header(&self) -> &WrenHeader {
        unsafe { &*self.0.get_user_data::<WrenHeader>() }
    }

    fn header_mut(&self) -> &mut WrenHeader {
        unsafe { &mut *self.0.get_user_data::<WrenHeader>() }
    }
}

impl<U, M, W> Drop for Wren<U, M, W> {
    fn drop(&mut self) {
        let data_ptr = self.data_ptr();

        unsafe { WrenData::loader_mut(data_ptr).assume_init_drop() };
        unsafe { WrenData::writer_mut(data_ptr).assume_init_drop() };
        unsafe { WrenData::user_data_mut(data_ptr).assume_init_drop() };

        let ref_count = self.header_mut().ref_count;

        if ref_count - 1 == 0 {}
    }
}

/// A compiled identifier for a Wren method signature.
///
/// This object holds a reference to the Wren virtual machine, and so should be
/// dropped.
pub struct CallHandle(WrenPtr, HandlePtr);

impl Drop for CallHandle {
    fn drop(&mut self) {
        todo!()
    }
}

#[repr(C)]
pub(crate) struct WrenData<U, M, W> {
    pub header: WrenHeader,
    pub loader: MaybeUninit<M>,
    pub writer: MaybeUninit<W>,
    pub user_data: MaybeUninit<U>,
}

impl<U, M, W> WrenData<U, M, W> {
    pub unsafe fn user_data<'a>(this: *mut Self) -> &'a MaybeUninit<U> {
        unsafe { &(*this).user_data }
    }

    pub unsafe fn user_data_mut<'a>(this: *mut Self) -> &'a mut MaybeUninit<U> {
        unsafe { &mut (*this).user_data }
    }

    pub unsafe fn loader<'a>(this: *mut Self) -> &'a MaybeUninit<M> {
        unsafe { &(*this).loader }
    }

    pub unsafe fn loader_mut<'a>(this: *mut Self) -> &'a mut MaybeUninit<M> {
        unsafe { &mut (*this).loader }
    }

    pub unsafe fn writer<'a>(this: *mut Self) -> &'a MaybeUninit<W> {
        unsafe { &(*this).writer }
    }

    pub unsafe fn writer_mut<'a>(this: *mut Self) -> &'a mut MaybeUninit<W> {
        unsafe { &mut (*this).writer }
    }
}

pub(crate) struct WrenHeader {
    pub inner_layout: Layout,
    pub ref_count: usize,
}

impl WrenHeader {
    pub unsafe fn release(this: *mut WrenHeader) {
        let ref_count = unsafe { WrenHeader::ref_count(this) };

        let ref_count = unsafe { ref_count.unchecked_sub(1) };

        if ref_count == 0 {}
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
