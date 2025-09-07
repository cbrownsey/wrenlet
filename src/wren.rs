use std::{
    alloc::Layout,
    cell::{Cell, RefCell, UnsafeCell},
    ffi::{CStr, CString},
    marker::PhantomData,
    ptr::NonNull,
};
use wren_sys as sys;

use crate::{
    WrenBuilder,
    value::{FromWren, FromWrenError, Value},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Runtime(RuntimeError),
    Compile(CompileError),
}

impl Error {
    pub fn new_runtime(message: String) -> Error {
        Error::Runtime(RuntimeError {
            message,
            stack_trace: Vec::new(),
        })
    }

    pub fn new_compiletime(module: String, line: i32, message: String) -> Error {
        Error::Compile(CompileError {
            module,
            line,
            message,
        })
    }

    pub fn add_stacktrace(&mut self, module: String, line: i32, method: String) {
        let Error::Runtime(RuntimeError { stack_trace, .. }) = self else {
            todo!();
        };

        stack_trace.push(StackTrace {
            module,
            line,
            method,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeError {
    message: String,
    stack_trace: Vec<StackTrace>,
}

#[derive(Debug, Clone, PartialEq)]
struct StackTrace {
    module: String,
    line: i32,
    method: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompileError {
    module: String,
    line: i32,
    message: String,
}

/// The Wren virtual machine.
#[repr(transparent)]
pub struct Wren<T = ()> {
    raw: RawWren,
    _marker: PhantomData<WrenInner<T>>,
}

impl Default for Wren {
    fn default() -> Self {
        Self::new()
    }
}

impl Wren {
    /// Creates a new `Wren` virtual machine, with no associated user data.
    pub fn new() -> Wren {
        Wren::builder().build()
    }

    /// Create a new builder instance to configure a new `Wren` virtual machine.
    pub fn builder() -> WrenBuilder {
        WrenBuilder::new()
    }
}

impl<U> Wren<U> {
    pub(crate) unsafe fn from_ptr(ptr: *mut sys::WrenVM) -> Wren<U> {
        Wren {
            raw: unsafe { RawWren::from_ptr(ptr) },
            _marker: PhantomData,
        }
    }

    /// Runs a string of Wren source code in a new fiber on the virtual machine, in the context of
    /// the resolved module.
    pub fn interpret(&mut self, module: &str, source: &str) -> Result<(), Error> {
        let module = CString::new(module).unwrap();
        let source = CString::new(source).unwrap();

        let result =
            unsafe { wren_sys::wrenInterpret(self.vm_ptr(), module.as_ptr(), source.as_ptr()) };

        match result {
            wren_sys::WrenInterpretResult::WREN_RESULT_SUCCESS => Ok(()),
            wren_sys::WrenInterpretResult::WREN_RESULT_COMPILE_ERROR => {
                let Some(error) = self.raw.take_error() else {
                    panic!();
                };

                debug_assert!(matches!(error, Error::Compile(_)));

                Err(error)
            }
            wren_sys::WrenInterpretResult::WREN_RESULT_RUNTIME_ERROR => {
                let Some(error) = self.raw.take_error() else {
                    panic!()
                };

                debug_assert!(matches!(error, Error::Runtime(_)));

                Err(error)
            }
            _ => unreachable!(),
        }
    }

    pub fn make_call_handle(&self, signature: &str) -> CallHandle {
        let signature = CString::new(signature).unwrap();

        let ptr = unsafe { sys::wrenMakeCallHandle(self.vm_ptr(), signature.as_ptr()) };

        // unsafe { self.raw.increment_count() };

        CallHandle(self.raw.clone(), RawHandle(NonNull::new(ptr).unwrap()))
    }

    /// Checks if this `Wren` has a module loaded with the given name.
    pub fn has_module(&self, module: &str) -> bool {
        let module = CString::new(module).unwrap();

        self.raw.has_module(&module)
    }

    /// Checks if this `Wren` has a module loaded with the given variable in it.
    pub fn has_variable(&self, module: &str, variable: &str) -> bool {
        let module = CString::new(module).unwrap();
        let variable = CString::new(variable).unwrap();

        self.raw.has_variable(&module, &variable)
    }

    pub fn get_variable<'a, T: FromWren<'a>>(
        &'a self,
        module: &str,
        variable: &str,
    ) -> Result<T, Error> {
        todo!()
    }

    pub(crate) fn vm_ptr(&self) -> *mut wren_sys::WrenVM {
        self.raw.vm_ptr()
    }

    pub(crate) fn get_slot_count(&self) -> usize {
        self.raw.get_slot_count()
    }

    pub(crate) fn ensure_slots(&self, slots: usize) {
        self.raw.ensure_slots(slots);
    }

    pub(crate) fn get_slot(&self, slot: usize) -> Option<Value<'_>> {
        self.raw.get_slot(slot)
    }

    pub(crate) fn set_slot<'a>(&mut self, slot: usize, value: Value<'a>) {
        unsafe { self.raw.set_slot(slot, value) };
    }

    /// Get a reference to the user data contained in this virtual machine.
    pub fn user_data(&self) -> &U {
        unsafe { self.raw.user_data() }
    }

    /// Get a mutable reference to the user data contained in this virtual machine.
    pub fn user_data_mut(&mut self) -> &mut U {
        unsafe { self.raw.user_data_mut() }
    }
}

impl<T> Drop for Wren<T> {
    fn drop(&mut self) {
        unsafe { self.raw.release() };
    }
}

pub struct CallHandle(RawWren, RawHandle);

#[derive(Debug, Clone)]
pub(crate) struct RawHandle(NonNull<sys::WrenHandle>);

#[derive(Debug, Clone)]
pub struct RawWren(NonNull<sys::WrenVM>);

/// A reference to a Wren virtual machine.
impl RawWren {
    pub(crate) unsafe fn from_ptr(ptr: *mut sys::WrenVM) -> RawWren {
        debug_assert!(!ptr.is_null());

        RawWren(unsafe { NonNull::new_unchecked(ptr) })
    }

    /// Checks if a module with the given name is loaded in the VM.
    pub(crate) fn has_module(&self, module: &CStr) -> bool {
        unsafe { sys::wrenHasModule(self.vm_ptr(), module.as_ptr()) }
    }

    /// Checks if a module is loaded, and if so, if there is a variable with the given name in it.
    pub(crate) fn has_variable(&self, module: &CStr, variable: &CStr) -> bool {
        self.has_module(module)
            && unsafe { sys::wrenHasVariable(self.vm_ptr(), module.as_ptr(), variable.as_ptr()) }
    }

    pub(crate) fn get_variable<'a, T: FromWren<'a>>(
        &self,
        module: &CStr,
        variable: &CStr,
    ) -> Result<T, FromWrenError> {
        todo!()
    }

    pub(crate) fn vm_ptr(&self) -> *mut sys::WrenVM {
        self.0.as_ptr()
    }

    pub(crate) fn get_slot_count(&self) -> usize {
        let count = unsafe { sys::wrenGetSlotCount(self.vm_ptr()) };
        usize::try_from(count).unwrap()
    }

    pub(crate) fn ensure_slots(&self, slots: usize) {
        let new_slots = self.get_slot_count()..slots;

        let slots = i32::try_from(slots).unwrap();

        unsafe { sys::wrenEnsureSlots(self.vm_ptr(), slots) };

        for slot in new_slots {
            unsafe { self.set_slot(slot, Value::Null) };
        }
    }

    pub(crate) fn get_slot(&self, slot: usize) -> Option<Value<'_>> {
        if slot >= self.get_slot_count() {
            return None;
        }

        let slot = i32::try_from(slot).unwrap();
        let ty = unsafe { sys::wrenGetSlotType(self.vm_ptr(), slot) };

        match ty {
            sys::WrenType::WREN_TYPE_NULL => Some(Value::Null),
            sys::WrenType::WREN_TYPE_BOOL => Some(Value::Bool(unsafe {
                sys::wrenGetSlotBool(self.vm_ptr(), slot)
            })),
            sys::WrenType::WREN_TYPE_NUM => Some(Value::Num(unsafe {
                sys::wrenGetSlotDouble(self.vm_ptr(), slot)
            })),
            sys::WrenType::WREN_TYPE_STRING => {
                let mut length = 0;
                let bytes = unsafe { sys::wrenGetSlotBytes(self.vm_ptr(), slot, &mut length) };
                let length = usize::try_from(length).unwrap();

                Some(Value::String(unsafe {
                    std::slice::from_raw_parts(bytes.cast::<u8>(), length)
                }))
            }
            sys::WrenType::WREN_TYPE_LIST => todo!(),
            sys::WrenType::WREN_TYPE_MAP => todo!(),
            sys::WrenType::WREN_TYPE_FOREIGN => todo!(),
            sys::WrenType::WREN_TYPE_UNKNOWN => todo!(),
            _ => unreachable!(),
        }
    }

    pub(crate) unsafe fn set_slot<'v>(&self, slot: usize, value: Value<'v>) {
        assert!(slot < self.get_slot_count());

        let slot = i32::try_from(slot).unwrap();

        match value {
            Value::Null => unsafe { sys::wrenSetSlotNull(self.vm_ptr(), slot) },
            Value::Bool(value) => unsafe {
                sys::wrenSetSlotBool(self.vm_ptr(), slot, value);
            },
            Value::Num(value) => unsafe { sys::wrenSetSlotDouble(self.vm_ptr(), slot, value) },
            Value::String(items) => unsafe {
                sys::wrenSetSlotBytes(self.vm_ptr(), slot, items.as_ptr().cast(), items.len())
            },
            Value::List => todo!(),
            Value::Map => todo!(),
            Value::Foreign => todo!(),
            Value::Unknown => todo!(),
        }
    }

    unsafe fn set_inner_ptr(&self, ptr: *mut sys::WrenVM) {
        unsafe { sys::wrenSetUserData(self.vm_ptr(), ptr.cast()) };
    }

    fn inner_ptr<T>(&self) -> *mut WrenInner<T> {
        let ptr = unsafe { sys::wrenGetUserData(self.vm_ptr()) };

        ptr.cast()
    }

    unsafe fn user_data<T>(&self) -> &T {
        let ptr = unsafe { &raw const (*self.inner_ptr::<T>()).user_data };

        unsafe { &*ptr }
    }

    unsafe fn user_data_mut<T>(&mut self) -> &mut T {
        let ptr = unsafe { &raw mut (*self.inner_ptr::<T>()).user_data };

        unsafe { &mut *ptr }
    }

    fn header(&self) -> &WrenHeader {
        let ptr = unsafe { sys::wrenGetUserData(self.vm_ptr()) }.cast::<WrenHeader>();

        // Safety: This is only safe since there is no way to materialize a
        // mutable reference to a `WrenInner` or `WrenHeader`.
        unsafe { &*ptr }
    }

    /// Releases this reference to the underlying `WrenVM`. If there are no other references to this `WrenVM`, then it
    /// is freed.
    pub(crate) unsafe fn release(&mut self) {
        unsafe { self.decrement_count() };

        if self.ref_count() == 0 {
            unsafe { sys::wrenFreeVM(self.vm_ptr()) };
        }
    }

    pub(crate) fn set_foreign_layout(&self, layout: Layout) {
        self.header().foreign_layout.set(Some(layout));
    }

    pub(crate) fn take_foreign_layout(&self) -> Option<Layout> {
        self.header().foreign_layout.take()
    }

    pub(crate) fn set_compile_error(&self, module: String, line: i32, message: String) {
        *self.header().error.borrow_mut() = Some(Error::Compile(CompileError {
            module,
            line,
            message,
        }));
    }

    pub(crate) fn set_runtime_error(&self, message: String) {
        *self.header().error.borrow_mut() = Some(Error::Runtime(RuntimeError {
            message,
            stack_trace: Vec::new(),
        }))
    }

    pub(crate) fn add_stacktrace(&self, module: String, line: i32, method: String) {
        let mut borrow = self.header().error.borrow_mut();
        let Some(Error::Runtime(RuntimeError { stack_trace, .. })) = borrow.as_mut() else {
            panic!("called `add_stacktrace` without a runtime error in place.")
        };

        stack_trace.push(StackTrace {
            module,
            line,
            method,
        })
    }

    pub(crate) fn take_error(&self) -> Option<Error> {
        self.header().error.borrow_mut().take()
    }

    pub(crate) fn allocate_slots(&self, slots: usize) -> usize {
        let count = self.header().slots_allocated.get() + slots;

        self.header().slots_allocated.set(count);

        count
    }

    pub(crate) fn deallocate_slots(&self, slots: usize) -> usize {
        let count = self.header().slots_allocated.get().saturating_sub(slots);

        self.header().slots_allocated.set(count);

        count
    }

    pub(crate) fn clear_slots(&self) {
        self.header().slots_allocated.set(0);
    }

    pub(crate) unsafe fn decrement_count(&self) {
        self.header()
            .ref_count
            .update(|n| unsafe { n.unchecked_sub(1) });
    }

    pub(crate) unsafe fn increment_count(&self) {
        self.header()
            .ref_count
            .update(|n| unsafe { n.unchecked_add(1) });
    }

    pub(crate) fn ref_count(&self) -> usize {
        self.header().ref_count.get()
    }
}

pub(crate) struct WrenHeader {
    pub(crate) foreign_layout: Cell<Option<Layout>>,
    pub(crate) ref_count: Cell<usize>,
    pub(crate) slots_allocated: Cell<usize>,
    pub(crate) error: RefCell<Option<Error>>,
}

#[repr(C)]
// The ?Sized bound exists only to ensure that `user_data` is at the end of the struct.
pub(crate) struct WrenInner<T: ?Sized> {
    pub(crate) header: WrenHeader,
    pub(crate) user_data: T,
}
