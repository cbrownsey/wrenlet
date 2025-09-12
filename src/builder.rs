use std::{
    alloc::{Layout, handle_alloc_error},
    cell::{Cell, RefCell, UnsafeCell},
    mem::MaybeUninit,
    ptr::NonNull,
};

use crate::{
    Wren,
    wren::{WrenHeader, WrenInner},
};

/// A Wren virtual machine builder. Providing fine control of how the Wren virtual machine should be instantiated.
pub struct WrenBuilder<T = ()> {
    user_data: T,
}

impl Default for WrenBuilder {
    fn default() -> Self {
        WrenBuilder::new()
    }
}

impl WrenBuilder {
    /// Creates a new, empty builder.
    pub fn new() -> WrenBuilder {
        WrenBuilder { user_data: () }
    }
}

impl<T> WrenBuilder<T> {
    pub fn user_data<U>(self, user_data: U) -> WrenBuilder<U> {
        let Self { .. } = self;

        WrenBuilder { user_data }
    }

    pub fn build(self) -> Wren<T> {
        let Self { user_data } = self;

        let mut conf = MaybeUninit::uninit();

        unsafe { sys::wrenInitConfiguration(conf.as_mut_ptr()) };

        let mut conf = unsafe { conf.assume_init() };

        conf.writeFn = Some(c_functions::write_fn);
        conf.errorFn = Some(c_functions::error_fn::<T>);
        conf.reallocateFn = Some(c_functions::reallocate_fn::<T>);
        conf.bindForeignMethodFn = Some(c_functions::bind_foreign_method_fn);
        conf.bindForeignClassFn = Some(c_functions::bind_foreign_class_fn);

        conf.userData = Box::into_raw(Box::new(WrenInner {
            header: WrenHeader {
                foreign_layout: Cell::new(None),
                ref_count: Cell::new(1),
                slots_allocated: Cell::new(0),
                error: RefCell::new(None),
            },
            user_data,
        }))
        .cast();

        let vm = unsafe { sys::wrenNewVM(&mut conf) };

        debug_assert!(!vm.is_null());

        unsafe { Wren::from_ptr(vm) }
    }
}

mod c_functions {
    use std::{
        alloc::Layout,
        ffi::{CStr, c_void},
        io::Write,
        marker::PhantomData,
    };

    use crate::{builder::WrenAllocation, wren::Error, wren::RawWren};

    pub extern "C" fn reallocate_fn<T>(
        ptr: *mut c_void,
        new_size: usize,
        _user_data: *mut c_void,
    ) -> *mut c_void {
        let _ = PhantomData::<T>;

        match (ptr.is_null(), new_size != 0) {
            // Deallocate existing allocation.
            (false, false) => {
                unsafe {
                    WrenAllocation::from_ptr(ptr).deallocate();
                };

                std::ptr::null_mut()
            }
            // Resize existing allocation.
            (false, true) => {
                let mut alloc = unsafe { WrenAllocation::from_ptr(ptr) };

                unsafe { alloc.reallocate(Layout::from_size_align_unchecked(new_size, 8)) };

                alloc.into_raw()
            }
            // Impossible combination.
            (true, false) => std::ptr::null_mut(),
            // Allocate new chunk.
            (true, true) => {
                let data_layout = Layout::from_size_align(new_size, 8).unwrap();

                let alloc = WrenAllocation::new(data_layout);

                alloc.into_raw()
            }
        }
    }

    pub extern "C" fn write_fn(_vm: *mut sys::WrenVM, text: *const i8) {
        let text = unsafe { CStr::from_ptr(text) };

        let text = text.to_bytes();

        let _ = std::io::stdout().write_all(text);
    }

    pub extern "C" fn error_fn<T>(
        vm: *mut sys::WrenVM,
        ty: sys::WrenErrorType,
        module: *const i8,
        line: i32,
        message: *const i8,
    ) {
        let mut vm = unsafe { RawWren::from_ptr(vm) };

        match ty {
            sys::WrenErrorType::WREN_ERROR_COMPILE => {
                assert!(!module.is_null());
                assert!(!message.is_null());

                let module = unsafe { CStr::from_ptr(module) }
                    .to_string_lossy()
                    .to_string();
                let message = unsafe { CStr::from_ptr(message) }
                    .to_string_lossy()
                    .to_string();

                unsafe { vm.set_compile_error(module, line, message) };
            }
            sys::WrenErrorType::WREN_ERROR_RUNTIME => {
                assert!(!message.is_null());

                let message = unsafe { CStr::from_ptr(message) }
                    .to_string_lossy()
                    .to_string();

                unsafe { vm.set_runtime_error(message) };
            }
            sys::WrenErrorType::WREN_ERROR_STACK_TRACE => {
                assert!(!module.is_null());
                assert!(!message.is_null());

                let module = unsafe { CStr::from_ptr(module) }
                    .to_string_lossy()
                    .to_string();
                let method = unsafe { CStr::from_ptr(message) }
                    .to_string_lossy()
                    .to_string();

                unsafe { vm.add_stacktrace(module, line, method) };
            }
            _ => unreachable!(),
        }
    }

    pub extern "C" fn bind_foreign_method_fn(
        _vm: *mut sys::WrenVM,
        _module: *const i8,
        _class: *const i8,
        _is_static: bool,
        _signature: *const i8,
    ) -> sys::WrenForeignMethodFn {
        None
    }

    pub extern "C" fn bind_foreign_class_fn(
        _vm: *mut sys::WrenVM,
        module: *const i8,
        class: *const i8,
    ) -> sys::WrenForeignClassMethods {
        let module = if !module.is_null() {
            Some(unsafe { CStr::from_ptr(module) })
        } else {
            None
        };

        let class = if !class.is_null() {
            Some(unsafe { CStr::from_ptr(class) })
        } else {
            None
        };

        dbg!(module, class);

        sys::WrenForeignClassMethods {
            allocate: None,
            finalize: None,
        }
    }
}

struct WrenAllocation(std::ptr::NonNull<u8>);

impl WrenAllocation {
    fn new(layout: Layout) -> WrenAllocation {
        let (full_layout, offset) = Layout::new::<Layout>().extend(layout).unwrap();

        let ptr = unsafe { std::alloc::alloc(full_layout) };

        if ptr.is_null() {
            handle_alloc_error(full_layout);
        }

        let ptr = unsafe { ptr.byte_add(offset) };

        let alloc = unsafe { WrenAllocation::from_ptr(ptr) };

        unsafe { alloc.layout_ptr().write(layout) };

        alloc
    }

    unsafe fn reallocate(&mut self, layout: Layout) {
        // An annoying one two is used to avoid having to think too hard about layouts.

        let mut new = WrenAllocation::new(layout);

        let to_copy = std::cmp::min(layout.size(), self.layout().size());

        unsafe {
            new.data_ptr()
                .copy_from_nonoverlapping(self.data_ptr(), to_copy)
        };

        std::mem::swap(self, &mut new);

        unsafe { new.deallocate() };
    }

    unsafe fn deallocate(self) {
        let layout = self.layout();

        let (layout, offset) = Layout::new::<Layout>().extend(layout).unwrap();

        let ptr = unsafe { self.data_ptr().byte_sub(offset) };

        unsafe { std::alloc::dealloc(ptr, layout) };
    }

    unsafe fn from_ptr(ptr: *mut impl Sized) -> WrenAllocation {
        unsafe { WrenAllocation(NonNull::new_unchecked(ptr.cast())) }
    }

    fn layout_ptr(&self) -> *mut Layout {
        unsafe {
            self.0
                .as_ptr()
                .cast::<Layout>()
                .byte_sub(std::mem::size_of::<Layout>())
        }
    }

    fn layout(&self) -> Layout {
        unsafe { self.layout_ptr().read() }
    }

    fn data_ptr(&self) -> *mut u8 {
        self.0.as_ptr()
    }

    fn into_raw(self) -> *mut std::ffi::c_void {
        self.0.as_ptr().cast()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wren_allocation_basics() {
        let alloc = WrenAllocation::new(Layout::new::<u128>());

        assert_eq!(alloc.layout(), Layout::new::<u128>());

        let alloc = WrenAllocation::new(Layout::new::<u8>());

        assert_eq!(alloc.layout(), Layout::new::<u8>());

        let alloc = WrenAllocation::new(Layout::new::<u64>());

        assert_eq!(alloc.layout(), Layout::new::<u64>());
    }
}
