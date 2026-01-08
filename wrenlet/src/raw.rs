use std::{ffi::CStr, ptr::NonNull};

use sys::WrenHandle;

use crate::{error::InterpretError, values::Type};

impl From<sys::WrenType> for Type {
    fn from(value: sys::WrenType) -> Self {
        match value {
            sys::WrenType::WREN_TYPE_BOOL => Type::Bool,
            sys::WrenType::WREN_TYPE_NUM => Type::Num,
            sys::WrenType::WREN_TYPE_FOREIGN => Type::Foreign,
            sys::WrenType::WREN_TYPE_LIST => Type::List,
            sys::WrenType::WREN_TYPE_MAP => Type::Map,
            sys::WrenType::WREN_TYPE_NULL => Type::Null,
            sys::WrenType::WREN_TYPE_STRING => Type::String,
            sys::WrenType::WREN_TYPE_UNKNOWN => Type::Unknown,

            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum InterpretResult {
    #[default]
    Success,
    Compile,
    Runtime,
}

impl From<sys::WrenInterpretResult> for InterpretResult {
    fn from(value: sys::WrenInterpretResult) -> Self {
        match value {
            sys::WrenInterpretResult::WREN_RESULT_SUCCESS => InterpretResult::Success,
            sys::WrenInterpretResult::WREN_RESULT_COMPILE_ERROR => InterpretResult::Compile,
            sys::WrenInterpretResult::WREN_RESULT_RUNTIME_ERROR => InterpretResult::Runtime,

            _ => unreachable!(),
        }
    }
}

pub struct WrenPtr(NonNull<sys::WrenVM>);

impl WrenPtr {
    pub(crate) const fn as_ptr(&self) -> *mut sys::WrenVM {
        self.0.as_ptr()
    }

    pub(crate) unsafe fn abort_fiber(&self, slot: i32) {
        unsafe { sys::wrenAbortFiber(self.as_ptr(), slot) }
    }

    pub(crate) unsafe fn call(&self, method: HandlePtr) -> InterpretResult {
        unsafe { sys::wrenCall(self.as_ptr(), method.as_ptr()) }.into()
    }

    pub(crate) unsafe fn collect_garbage(&self) {
        unsafe { sys::wrenCollectGarbage(self.as_ptr()) };
    }

    pub(crate) unsafe fn ensure_slots(&self, num_slots: i32) {
        unsafe { sys::wrenEnsureSlots(self.as_ptr(), num_slots) };
    }

    pub(crate) unsafe fn free_vm(&self) {
        unsafe { sys::wrenFreeVM(self.as_ptr()) };
    }

    pub(crate) unsafe fn get_list_count(&self, slot: i32) -> i32 {
        unsafe { sys::wrenGetListCount(self.as_ptr(), slot) }
    }

    pub(crate) unsafe fn get_list_element(&self, list_slot: i32, index: i32, element_slot: i32) {
        unsafe { sys::wrenGetListElement(self.as_ptr(), list_slot, index, element_slot) };
    }

    pub(crate) unsafe fn get_map_contains_key(&self, map_slot: i32, key_slot: i32) -> bool {
        unsafe { sys::wrenGetMapContainsKey(self.as_ptr(), map_slot, key_slot) }
    }

    pub(crate) unsafe fn get_map_count(&self, slot: i32) -> i32 {
        unsafe { sys::wrenGetMapCount(self.as_ptr(), slot) }
    }

    pub(crate) unsafe fn get_map_value(&self, map_slot: i32, key_slot: i32, value_slot: i32) {
        unsafe { sys::wrenGetMapValue(self.as_ptr(), map_slot, key_slot, value_slot) };
    }

    pub(crate) unsafe fn get_slot_bool(&self, slot: i32) -> bool {
        unsafe { sys::wrenGetSlotBool(self.as_ptr(), slot) }
    }

    pub(crate) unsafe fn get_slot_bytes(&self, slot: i32) -> &[u8] {
        let mut length = 0;

        let ptr = unsafe { sys::wrenGetSlotBytes(self.as_ptr(), slot, &mut length) };

        unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), length as usize) }
    }

    pub(crate) unsafe fn get_slot_count(&self) -> i32 {
        unsafe { sys::wrenGetSlotCount(self.as_ptr()) }
    }

    pub(crate) unsafe fn get_slot_double(&self, slot: i32) -> f64 {
        unsafe { sys::wrenGetSlotDouble(self.as_ptr(), slot) }
    }

    pub(crate) unsafe fn get_slot_foreign<T>(&self, slot: i32) -> *mut T {
        unsafe { sys::wrenGetSlotForeign(self.as_ptr(), slot) }.cast()
    }

    pub(crate) unsafe fn get_slot_handle(&self, slot: i32) -> HandlePtr {
        let ptr = unsafe { sys::wrenGetSlotHandle(self.as_ptr(), slot) };

        let Some(ptr) = NonNull::new(ptr) else {
            unreachable!()
        };

        HandlePtr(ptr)
    }

    pub(crate) unsafe fn get_slot_type(&self, slot: i32) -> Type {
        unsafe { sys::wrenGetSlotType(self.as_ptr(), slot) }.into()
    }

    pub(crate) unsafe fn get_user_data<T>(&self) -> *mut T {
        unsafe { sys::wrenGetUserData(self.as_ptr()) }.cast()
    }

    pub(crate) unsafe fn get_variable(&self, module: &CStr, name: &CStr, slot: i32) {
        unsafe { sys::wrenGetVariable(self.as_ptr(), module.as_ptr(), name.as_ptr(), slot) };
    }

    pub(crate) unsafe fn has_module(&self, module: &CStr) -> bool {
        unsafe { sys::wrenHasModule(self.as_ptr(), module.as_ptr()) }
    }

    pub(crate) unsafe fn has_variable(&self, module: &CStr, name: &CStr) -> bool {
        unsafe { sys::wrenHasVariable(self.as_ptr(), module.as_ptr(), name.as_ptr()) }
    }

    pub(crate) unsafe fn insert_in_list(&self, list_slot: i32, index: i32, element_slot: i32) {
        unsafe { sys::wrenInsertInList(self.as_ptr(), list_slot, index, element_slot) };
    }

    pub(crate) unsafe fn interpret(&self, module: &CStr, source: &CStr) -> InterpretResult {
        unsafe { sys::wrenInterpret(self.as_ptr(), module.as_ptr(), source.as_ptr()) }.into()
    }

    pub(crate) unsafe fn make_call_handle(&self, signature: &CStr) -> HandlePtr {
        let ptr = unsafe { sys::wrenMakeCallHandle(self.as_ptr(), signature.as_ptr()) };

        let Some(ptr) = NonNull::new(ptr) else {
            unreachable!()
        };

        HandlePtr(ptr)
    }

    pub(crate) unsafe fn new(conf: sys::WrenConfiguration) -> WrenPtr {
        let mut conf = conf;
        let ptr = unsafe { sys::wrenNewVM(&mut conf) };

        let Some(ptr) = NonNull::new(ptr) else {
            todo!()
        };

        WrenPtr(ptr)
    }

    pub(crate) unsafe fn release_handle(&self, handle: HandlePtr) {
        unsafe { sys::wrenReleaseHandle(self.as_ptr(), handle.as_ptr()) };
    }

    pub(crate) unsafe fn remove_map_value(
        &self,
        map_slot: i32,
        key_slot: i32,
        removed_value_slot: i32,
    ) {
        unsafe { sys::wrenRemoveMapValue(self.as_ptr(), map_slot, key_slot, removed_value_slot) };
    }

    pub(crate) unsafe fn set_list_element(&self, list_slot: i32, index: i32, element_slot: i32) {
        unsafe { sys::wrenSetListElement(self.as_ptr(), list_slot, index, element_slot) };
    }

    pub(crate) unsafe fn set_map_value(&self, map_slot: i32, key_slot: i32, value_slot: i32) {
        unsafe { sys::wrenSetMapValue(self.as_ptr(), map_slot, key_slot, value_slot) };
    }

    pub(crate) unsafe fn set_slot_bool(&self, slot: i32, value: bool) {
        unsafe { sys::wrenSetSlotBool(self.as_ptr(), slot, value) };
    }

    pub(crate) unsafe fn set_slot_bytes(&self, slot: i32, bytes: &[u8]) {
        unsafe { sys::wrenSetSlotBytes(self.as_ptr(), slot, bytes.as_ptr().cast(), bytes.len()) };
    }

    pub(crate) unsafe fn set_slot_double(&self, slot: i32, value: f64) {
        unsafe { sys::wrenSetSlotDouble(self.as_ptr(), slot, value) };
    }

    pub(crate) unsafe fn set_slot_handle(&self, slot: i32, handle: HandlePtr) {
        unsafe { sys::wrenSetSlotHandle(self.as_ptr(), slot, handle.as_ptr()) };
    }

    pub(crate) unsafe fn set_slot_new_foreign<T>(
        &self,
        slot: i32,
        class_slot: i32,
        size: usize,
    ) -> *mut T {
        unsafe { sys::wrenSetSlotNewForeign(self.as_ptr(), slot, class_slot, size) }.cast()
    }

    pub(crate) unsafe fn set_slot_new_list(&self, slot: i32) {
        unsafe { sys::wrenSetSlotNewList(self.as_ptr(), slot) };
    }

    pub(crate) unsafe fn set_slot_new_map(&self, slot: i32) {
        unsafe { sys::wrenSetSlotNewMap(self.as_ptr(), slot) };
    }

    pub(crate) unsafe fn set_slot_null(&self, slot: i32) {
        unsafe { sys::wrenSetSlotNull(self.as_ptr(), slot) };
    }

    pub(crate) unsafe fn set_user_data(&self, ptr: *mut impl Sized) {
        unsafe { sys::wrenSetUserData(self.as_ptr(), ptr.cast()) };
    }

    pub(crate) unsafe fn from_ptr(ptr: *mut sys::WrenVM) -> Self {
        Self(unsafe { NonNull::new_unchecked(ptr) })
    }
}

pub struct HandlePtr(NonNull<sys::WrenHandle>);

impl HandlePtr {
    const fn as_ptr(&self) -> *mut sys::WrenHandle {
        self.0.as_ptr()
    }
}
