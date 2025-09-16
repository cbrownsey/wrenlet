use std::{
    alloc::{Layout, handle_alloc_error},
    io::Stdout,
    mem::MaybeUninit,
};

use crate::{
    module::{Empty, ModuleLoader},
    wren2::{Wren, WrenData, WrenHeader},
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

        let inner_layout = Layout::new::<WrenData<U, M, W>>();

        let user_data = WrenData {
            header: WrenHeader {
                inner_layout,
                ref_count: 1,
            },
            loader: MaybeUninit::new(self.loader),
            writer: MaybeUninit::new(self.writer),
            user_data: MaybeUninit::new(self.user_data),
        };

        let user_data_ptr = unsafe { std::alloc::alloc(inner_layout) }.cast::<WrenData<U, M, W>>();

        if user_data_ptr.is_null() {
            handle_alloc_error(inner_layout);
        }

        unsafe { user_data_ptr.write(user_data) };

        conf.userData = user_data_ptr.cast::<core::ffi::c_void>();

        conf.writeFn = Some(c_functions::write_fn::<U, M, W>);
        conf.errorFn = Some(c_functions::error_fn::<U, M, W>);

        let ptr = unsafe { sys::wrenNewVM(&mut conf) };
        unsafe { Wren::from_ptr(ptr) }
    }
}

mod c_functions {
    use std::ffi::CStr;

    use crate::wren2::Wren;

    pub unsafe extern "C" fn write_fn<U, M, W>(vm: *mut sys::WrenVM, text: *const i8) {
        let _wren = unsafe { Wren::<U, M, W>::from_ptr(vm) };

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
        let _wren = unsafe { Wren::<U, M, W>::from_ptr(vm) };

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
}
