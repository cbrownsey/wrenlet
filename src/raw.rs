use std::{alloc::Layout, ffi::CStr, ptr::NonNull};

const SLOT_FROM_USIZE_MSG: &str = "Attempted to get a slot index from an invalid usize.";

/// The type of a value stored in a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum WrenType {
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
pub enum InterpretError {
    Compile = 1,
    Runtime,
}

impl InterpretError {
    fn from_raw(result: sys::WrenInterpretResult) -> Result<(), InterpretError> {
        match result {
            sys::WrenInterpretResult::WREN_RESULT_SUCCESS => Ok(()),
            sys::WrenInterpretResult::WREN_RESULT_COMPILE_ERROR => Err(InterpretError::Compile),
            sys::WrenInterpretResult::WREN_RESULT_RUNTIME_ERROR => Err(InterpretError::Runtime),
            _ => unreachable!(),
        }
    }
}

/// A raw pointer to the underlying C [`WrenVM`].
///
/// This struct represents a sharable pointer to an underlying [`WrenVM`]. The existence of this struct guarantees that
/// the contained pointer references a live virtual machine.
///
/// [`WrenVM`]: wren_sys::WrenVM
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct WrenPtr(NonNull<sys::WrenVM>);

impl WrenPtr {
    pub unsafe fn from_raw(ptr: *mut sys::WrenVM) -> WrenPtr {
        WrenPtr(unsafe { NonNull::new_unchecked(ptr) })
    }

    /// Disposes of all resources in use by the VM.
    ///
    /// # Safety
    /// This function may only be called if there are no other copies of this `WrenPtr`. This `WrenPtr` may not be used
    /// after this function is called.
    pub unsafe fn free(&self) {
        unsafe { sys::wrenFreeVM(self.0.as_ptr()) };
    }

    /// Immediately run the garbage collector to free unused allocations.
    pub unsafe fn collect_garbage(&self) {
        unsafe { sys::wrenCollectGarbage(self.0.as_ptr()) };
    }

    /// Runs a string of Wren code in a new fiber in the context of the given module.
    pub unsafe fn interpret(&self, module: &CStr, source: &CStr) -> Result<(), InterpretError> {
        let result =
            unsafe { sys::wrenInterpret(self.0.as_ptr(), module.as_ptr(), source.as_ptr()) };

        InterpretError::from_raw(result)
    }

    /// Creates a handle which can be used to invoke a method with the given
    /// signature, using a reciever and arguments already set up on the stack.
    ///
    /// The returned handle should be released with [`WrenPtr::release_handle`].
    pub fn make_call_handle(&self, signature: &CStr) -> HandlePtr {
        let ptr = unsafe { sys::wrenMakeCallHandle(self.0.as_ptr(), signature.as_ptr()) };

        debug_assert!(!ptr.is_null());

        HandlePtr(unsafe { NonNull::new_unchecked(ptr) })
    }

    /// Calls the given method, using the reciever and arguments previously set
    /// up on the stack.
    ///
    /// After this function returns, the return value of the function will be
    /// in slot zero.
    ///
    /// # Safety
    /// The given handle must have been created with a call to
    /// [`WrenPtr::make_call_handle`] on an instance of this [`WrenPtr`]. The
    /// reciever to this function must be in slot zero, and the remaining
    /// arguments must be in the following slots.
    pub unsafe fn call(&self, handle: HandlePtr) -> Result<(), InterpretError> {
        let result = unsafe { sys::wrenCall(self.0.as_ptr(), handle.0.as_ptr()) };

        InterpretError::from_raw(result)
    }

    /// Releases the given handle reference.
    ///
    /// # Safety
    /// Once this function is called, the given handle pointer is invalidated,
    /// and must not be used again.
    pub unsafe fn release_handle(&self, handle: HandlePtr) {
        unsafe { sys::wrenReleaseHandle(self.0.as_ptr(), handle.0.as_ptr()) };
    }

    /// Gets the number of slots available on the [`WrenVM`].
    ///
    /// [`WrenVM`]: sys::WrenVM
    pub fn get_slot_count(&self) -> usize {
        let slots = unsafe { sys::wrenGetSlotCount(self.0.as_ptr()) };

        usize::try_from(slots).expect(SLOT_FROM_USIZE_MSG)
    }

    /// Reserves capacity for at least `total` many elements in the slot
    /// array, growing it if needed.
    ///
    /// This method will never shrink the slot array.
    ///
    /// This method also initialises the newly allocated slots to null values.
    ///
    /// # Safety
    /// This method must not be called from a finalizer method.
    pub unsafe fn ensure_slots(&self, total: usize) {
        let Ok(slots) = i32::try_from(total) else {
            todo!()
        };

        let old = self.get_slot_count();
        unsafe { sys::wrenEnsureSlots(self.0.as_ptr(), slots) };
        let new = self.get_slot_count();
        for i in old..new {
            unsafe { self.set_slot_null(i) };
        }
    }

    /// Gets the type of the value in `slot`.
    ///
    /// # Safety
    /// The given slot must be both valid and initialised.
    pub unsafe fn get_slot_type(&self, slot: usize) -> WrenType {
        let Ok(slot) = i32::try_from(slot) else {
            todo!()
        };

        let ty = unsafe { sys::wrenGetSlotType(self.0.as_ptr(), slot) };

        match ty {
            sys::WrenType::WREN_TYPE_NULL => WrenType::Null,
            sys::WrenType::WREN_TYPE_BOOL => WrenType::Bool,
            sys::WrenType::WREN_TYPE_NUM => WrenType::Num,
            sys::WrenType::WREN_TYPE_STRING => WrenType::String,
            sys::WrenType::WREN_TYPE_LIST => WrenType::List,
            sys::WrenType::WREN_TYPE_MAP => WrenType::Map,
            sys::WrenType::WREN_TYPE_FOREIGN => WrenType::Foreign,
            sys::WrenType::WREN_TYPE_UNKNOWN => WrenType::Unknown,
            _ => unreachable!(),
        }
    }

    /// Gets the boolean value stored in the given `slot`.
    ///
    /// # Safety
    /// The given slot must be valid and contain a boolean value.
    pub unsafe fn get_slot_bool(&self, slot: usize) -> bool {
        debug_assert_eq!(unsafe { self.get_slot_type(slot) }, WrenType::Bool);

        let slot = i32::try_from(slot).expect(SLOT_FROM_USIZE_MSG);

        unsafe { sys::wrenGetSlotBool(self.0.as_ptr(), slot) }
    }

    /// Gets the double floating point value stored in the given `slot`.
    ///
    /// # Safety
    /// The given slot must be valid and contain a double value.
    pub unsafe fn get_slot_double(&self, slot: usize) -> f64 {
        debug_assert_eq!(unsafe { self.get_slot_type(slot) }, WrenType::Num);

        let Ok(slot) = i32::try_from(slot) else {
            todo!();
        };

        unsafe { sys::wrenGetSlotDouble(self.0.as_ptr(), slot) }
    }

    /// Reads a pointer to the string stored in the given `slot`.
    ///
    /// # Safety
    /// The given slot must be valid and contain a string, and the given
    /// pointer must not be dereferenced after the given `slot` is modified.
    pub unsafe fn get_slot_string(&self, slot: usize) -> *const [u8] {
        debug_assert_eq!(unsafe { self.get_slot_type(slot) }, WrenType::String);

        let Ok(slot) = i32::try_from(slot) else {
            todo!();
        };

        let mut length = 0;
        let ptr = unsafe { sys::wrenGetSlotBytes(self.0.as_ptr(), slot, &mut length) };

        core::ptr::slice_from_raw_parts(ptr.cast::<u8>(), length as usize)
    }

    /// Gets the number of elements stored in the list in the given `slot`.
    ///
    /// # Safety
    /// The given slot must contain a list value.
    pub unsafe fn get_list_count(&self, slot: usize) -> usize {
        let slot = i32::try_from(slot).expect(SLOT_FROM_USIZE_MSG);

        let count = unsafe { sys::wrenGetListCount(self.0.as_ptr(), slot) };

        usize::try_from(count).unwrap()
    }

    /// Reads the value at `index` in the list at `slot`, and stores it in the slot `into`.
    ///
    /// # Safety
    /// - The given `slot` must contain a list,
    /// - `index` must be strictly less than the length of that list,
    /// - `into` must be a valid slot.
    pub unsafe fn get_list_element(&self, slot: usize, index: usize, into: usize) {
        todo!()
    }

    /// Gets the number of entries in the map stored in `slot`.
    ///
    /// # Safety
    /// The given `slot` must contain a map value.
    pub unsafe fn get_map_count(&self, slot: usize) -> usize {
        todo!()
    }

    /// Returns `true` if the map stored in `slot` contains the key stored in `key`.
    ///
    /// # Safety
    /// - The given `slot` must contain a map value,
    /// - `key` must contain an initialised value.
    pub unsafe fn map_contains_key(&self, slot: usize, key: usize) -> bool {
        todo!()
    }

    /// Retrieves a value from the map in `slot` with the key in `key`.
    /// Storing the result in `value`.
    ///
    /// # Safety
    /// - The given `slot` must contain a map value,
    /// - `key` must contain an initialised value,
    /// - `value` must be a slot which exists.
    pub unsafe fn get_map_value(&self, slot: usize, key: usize, value: usize) {
        todo!()
    }

    /// Reads a pointer to the foreign value stored in the given `slot`.
    ///
    /// # Safety
    /// The given slot must be valid and contain a foreign value, and the given
    /// pointer must not be dereferenced after the given `slot` is modified.
    pub unsafe fn get_slot_foreign<T>(&self, slot: usize) -> *mut T {
        debug_assert_eq!(unsafe { self.get_slot_type(slot) }, WrenType::Foreign);

        let slot = i32::try_from(slot).expect(SLOT_FROM_USIZE_MSG);

        unsafe { sys::wrenGetSlotForeign(self.0.as_ptr(), slot) }.cast::<T>()
    }

    /// Creates a handle for the value stored in `slot`.
    ///
    /// The returned [`HandlePtr`] should be released with
    /// [`WrenPtr::release_handle`].
    pub unsafe fn get_slot_handle(&self, slot: usize) -> HandlePtr {
        let slot = i32::try_from(slot).expect(SLOT_FROM_USIZE_MSG);

        let ptr = unsafe { sys::wrenGetSlotHandle(self.0.as_ptr(), slot) };

        HandlePtr(unsafe { NonNull::new_unchecked(ptr) })
    }

    /// Sets the given `slot` to a null value.
    pub unsafe fn set_slot_null(&self, slot: usize) {
        let slot = i32::try_from(slot).expect(SLOT_FROM_USIZE_MSG);

        unsafe { sys::wrenSetSlotNull(self.0.as_ptr(), slot) };
    }

    /// Sets the given `slot` to the boolean `value`.
    pub unsafe fn set_slot_bool(&self, slot: usize, value: bool) {
        let slot = i32::try_from(slot).expect(SLOT_FROM_USIZE_MSG);

        unsafe { sys::wrenSetSlotBool(self.0.as_ptr(), slot, value) };
    }

    /// Sets the given `slot` to the double `value`.
    pub unsafe fn set_slot_double(&self, slot: usize, value: f64) {
        let slot = i32::try_from(slot).expect(SLOT_FROM_USIZE_MSG);

        unsafe { sys::wrenSetSlotDouble(self.0.as_ptr(), slot, value) };
    }

    /// Sets the given `slot` to the string pointed to by `value`.
    ///
    /// The bytes pointed to by `value` are copied to a new string in the
    /// `Wren`'s heap. So the pointer may be invalidated after this function
    /// returns.
    ///
    /// # Safety
    /// - `slot` must be a valid slot,
    /// - `value` must point to initialised bytes.
    pub unsafe fn set_slot_bytes(&self, slot: usize, value: *const [u8]) {
        let slot = i32::try_from(slot).expect(SLOT_FROM_USIZE_MSG);

        unsafe { sys::wrenSetSlotBytes(self.0.as_ptr(), slot, value.cast::<i8>(), value.len()) };
    }

    /// Stores a new empty list in `slot`.
    pub unsafe fn set_slot_new_list(&self, slot: usize) {
        todo!()
    }

    /// Stores a new empty map in `slot`.
    pub unsafe fn set_slot_new_map(&self, slot: usize) {
        todo!()
    }

    /// Creates a new uninitialised instance foreign class in `class_slot` in
    /// `slot` with the given `layout`.
    ///
    /// This does not invoke the foreign class's constructor on the new
    /// instance. If you need that to happen, call the constructor from Wren,
    /// which will then call the allocator foreign method. In there, call
    /// this to create the object and then the constructor will be invoked when
    /// the allocator returns.
    pub unsafe fn set_slot_new_foreign<T>(
        &self,
        slot: usize,
        class_slot: usize,
        size: usize,
    ) -> *mut T {
        todo!()
    }

    /// Stores the value captured in `handle` in the given slot.
    ///
    /// This does not release the given handle.
    pub unsafe fn set_slot_handle(&self, slot: usize, handle: HandlePtr) {
        todo!()
    }

    pub unsafe fn insert_in_list(&self, slot: usize, index: isize, value_slot: usize) {
        todo!()
    }

    pub unsafe fn set_map_value(&self, slot: usize, key_slot: usize, value_slot: usize) {
        todo!()
    }

    pub unsafe fn remove_map_value(&self, slot: usize, key_slot: usize, into_slot: usize) {
        todo!()
    }

    /// Checks if the given `module` is loaded.
    pub fn has_module(&self, module: &CStr) -> bool {
        unsafe { sys::wrenHasModule(self.0.as_ptr(), module.as_ptr()) }
    }

    /// Checks if the top level variable `name` exists in the given `module`.
    ///
    /// # Safety
    /// The given `module` must be already be imported.
    pub unsafe fn has_variable(&self, module: &CStr, name: &CStr) -> bool {
        debug_assert!(self.has_module(module));

        unsafe { sys::wrenHasVariable(self.0.as_ptr(), module.as_ptr(), name.as_ptr()) }
    }

    /// Stores the variable with `name` in `module` in the given `slot`.
    ///
    /// # Safety
    /// - The given module and variable must both exist,
    /// - The given `slot` must be a valid slot.
    pub unsafe fn get_variable(&self, module: &CStr, name: &CStr, slot: usize) {
        debug_assert!(self.has_module(module));
        debug_assert!(unsafe { self.has_variable(module, name) });

        let slot = i32::try_from(slot).expect(SLOT_FROM_USIZE_MSG);

        unsafe { sys::wrenGetVariable(self.0.as_ptr(), module.as_ptr(), name.as_ptr(), slot) };
    }

    /// Sets the current fiber to be aborted, using the value stored in `slot`
    /// as the runtime error object.
    pub unsafe fn abort_fiber(&self, slot: usize) {
        todo!()
    }

    /// Sets the user data pointer of the underlying [`WrenVM`].
    pub unsafe fn set_user_data(&self, ptr: *mut impl Sized) {
        unsafe { sys::wrenSetUserData(self.0.as_ptr(), ptr.cast()) };
    }

    /// Gets the user data pointer of the underlying [`WrenVM`].
    pub fn get_user_data<T>(&self) -> *mut T {
        unsafe { sys::wrenGetUserData(self.0.as_ptr()) }.cast()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandlePtr(NonNull<sys::WrenHandle>);
