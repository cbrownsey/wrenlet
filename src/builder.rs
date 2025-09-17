use std::{
    alloc::{Layout, handle_alloc_error},
    io::Stdout,
    mem::MaybeUninit,
};

use crate::{
    module::{Empty, ModuleLoader},
    wren::{Wren, WrenData, WrenHeader},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Builder<U, M, W> {
    user_data: U,
    loader: M,
    writer: W,
}

impl Builder<(), Empty, Stdout> {
    pub fn new() -> Self {
        Builder {
            user_data: (),
            loader: Empty,
            writer: std::io::stdout(),
        }
    }
}

impl<U, M, W> Builder<U, M, W> {
    pub fn with_data<T>(self, user_data: T) -> Builder<T, M, W> {
        let Builder { loader, writer, .. } = self;

        Builder {
            user_data,
            loader,
            writer,
        }
    }

    pub fn with_loader<T>(self, loader: T) -> Builder<U, T, W>
    where
        T: ModuleLoader,
    {
        let Builder {
            user_data, writer, ..
        } = self;

        Builder {
            user_data,
            loader,
            writer,
        }
    }

    pub fn with_output<T>(self, writer: T) -> Builder<U, M, T>
    where
        T: std::io::Write,
    {
        let Builder {
            user_data, loader, ..
        } = self;

        Builder {
            user_data,
            loader,
            writer,
        }
    }

    pub fn build(self) -> Wren<U, M, W>
    where
        M: ModuleLoader,
        W: std::io::Write,
    {
        let mut conf = MaybeUninit::uninit();

        unsafe { sys::wrenInitConfiguration(conf.as_mut_ptr()) };

        let mut conf = unsafe { conf.assume_init() };

        let user_data = WrenData::allocate(self.user_data, self.loader, self.writer);

        conf.userData = user_data.cast::<core::ffi::c_void>();

        conf.writeFn = Some(c_functions::write_fn::<U, M, W>);
        conf.errorFn = Some(c_functions::error_fn::<U, M, W>);
        conf.bindForeignClassFn = Some(c_functions::bind_foreign_class_fn);
        conf.bindForeignMethodFn = Some(c_functions::bind_foreign_method_fn);

        let ptr = unsafe { sys::wrenNewVM(&mut conf) };
        unsafe { Wren::from_ptr(ptr) }
    }
}

mod c_functions {
    use std::{ffi::CStr, mem::ManuallyDrop};

    use crate::{raw::WrenPtr, wren::Wren};

    pub unsafe extern "C" fn write_fn<U, M, W>(vm: *mut sys::WrenVM, text: *const i8) {
        let _wren = ManuallyDrop::new(unsafe { Wren::<U, M, W>::from_ptr(vm) });

        let text = unsafe { CStr::from_ptr(text) };

        print!("{}", text.to_string_lossy());
    }

    pub unsafe extern "C" fn error_fn<U, M, W>(
        vm: *mut sys::WrenVM,
        error_type: sys::WrenErrorType,
        module: *const i8,
        line: i32,
        message: *const i8,
    ) {
        let _wren = ManuallyDrop::new(unsafe { Wren::<U, M, W>::from_ptr(vm) });

        match error_type {
            sys::WrenErrorType::WREN_ERROR_COMPILE => {
                let module = unsafe { CStr::from_ptr(module) };
                let message = unsafe { CStr::from_ptr(message) };

                println!(
                    "[{} line {line}] [Error] {}",
                    module.to_string_lossy(),
                    message.to_string_lossy()
                );
            }
            sys::WrenErrorType::WREN_ERROR_RUNTIME => {
                let message = unsafe { CStr::from_ptr(message) };

                println!("[Runtime Error] {}", message.to_string_lossy());
            }
            sys::WrenErrorType::WREN_ERROR_STACK_TRACE => {
                let module = unsafe { CStr::from_ptr(module) };
                let method = unsafe { CStr::from_ptr(message) };

                println!(
                    "[{} line {line}] in {}",
                    module.to_string_lossy(),
                    method.to_string_lossy()
                );
            }
            _ => unreachable!(),
        }
    }

    pub unsafe extern "C" fn bind_foreign_class_fn(
        vm: *mut sys::WrenVM,
        module: *const i8,
        class_name: *const i8,
    ) -> sys::WrenForeignClassMethods {
        let _wren = unsafe { WrenPtr::from_raw(vm) };

        assert!(!module.is_null());
        assert!(!class_name.is_null());

        let module = unsafe { CStr::from_ptr(module) };
        let class_name = unsafe { CStr::from_ptr(class_name) };

        dbg!(module, class_name);

        sys::WrenForeignClassMethods {
            allocate: None,
            finalize: None,
        }
    }

    pub unsafe extern "C" fn bind_foreign_method_fn(
        vm: *mut sys::WrenVM,
        module: *const i8,
        class_name: *const i8,
        is_static: bool,
        signature: *const i8,
    ) -> Option<unsafe extern "C" fn(*mut sys::WrenVM)> {
        let _wren = unsafe { WrenPtr::from_raw(vm) };

        assert!(!module.is_null());
        assert!(!class_name.is_null());
        assert!(!signature.is_null());

        let module = unsafe { CStr::from_ptr(module) };
        let class_name = unsafe { CStr::from_ptr(class_name) };
        let signature = unsafe { CStr::from_ptr(signature) };

        dbg!(module, class_name, is_static, signature);

        None
    }
}
