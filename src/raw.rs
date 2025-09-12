use std::{
    ffi::{CStr, c_void},
    ptr::NonNull,
};

/// The type of a value stored in a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
enum WrenType {
    #[default]
    Null,
    Bool,
    Num,
    String,
    List,
    Map,
    Unknown,
    Foreign,
}

impl From<sys::WrenType> for WrenType {
    fn from(value: sys::WrenType) -> Self {
        match value {
            sys::WrenType::WREN_TYPE_NULL => WrenType::Null,
            sys::WrenType::WREN_TYPE_BOOL => WrenType::Bool,
            sys::WrenType::WREN_TYPE_NUM => WrenType::Num,
            sys::WrenType::WREN_TYPE_STRING => WrenType::String,
            sys::WrenType::WREN_TYPE_LIST => WrenType::List,
            sys::WrenType::WREN_TYPE_MAP => WrenType::Map,
            sys::WrenType::WREN_TYPE_UNKNOWN => WrenType::Unknown,
            sys::WrenType::WREN_TYPE_FOREIGN => WrenType::Foreign,
            _ => unreachable!(),
        }
    }
}

/// The type of error encountered when interpreting a string of source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum InterpretError {
    Compile = 1,
    Runtime,
}

/// A raw pointer to the underlying C [`WrenVM`].
///
/// This struct represents a sharable pointer to an underlying [`WrenVM`]. The existence of this struct guarantees that
/// the contained pointer references a live virtual machine.
///
/// [`WrenVM`]: wren_sys::WrenVM
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WrenPtr(NonNull<sys::WrenVM>);

impl WrenPtr {
    unsafe fn from_ptr_unchecked(ptr: *mut sys::WrenVM) -> WrenPtr {
        WrenPtr(unsafe { NonNull::new_unchecked(ptr) })
    }

    /// Disposes of all resources in use by the VM.
    ///
    /// # Safety
    /// This function may only be called if there are no other copies of this `WrenPtr`. This `WrenPtr` may not be used
    /// after this function is called.
    unsafe fn free(&self) {
        unsafe { sys::wrenFreeVM(self.0.as_ptr()) };
    }

    unsafe fn interpret(&self, module: &CStr, source: &CStr) -> Result<(), InterpretError> {
        let result =
            unsafe { sys::wrenInterpret(self.0.as_ptr(), module.as_ptr(), source.as_ptr()) };

        match result {
            sys::WrenInterpretResult::WREN_RESULT_SUCCESS => Ok(()),
            sys::WrenInterpretResult::WREN_RESULT_COMPILE_ERROR => Err(InterpretError::Compile),
            sys::WrenInterpretResult::WREN_RESULT_RUNTIME_ERROR => Err(InterpretError::Runtime),
            _ => unreachable!(),
        }
    }

    unsafe fn set_user_data(&self, ptr: *mut c_void) {
        unsafe { sys::wrenSetUserData(self.0.as_ptr(), ptr) };
    }

    unsafe fn collect_garbage(&self) {
        unsafe { sys::wrenCollectGarbage(self.0.as_ptr()) };
    }

    fn get_user_data(&self) -> *mut c_void {
        unsafe { sys::wrenGetUserData(self.0.as_ptr()) }
    }
}
