use std::{
    ffi::{CStr, CString},
    mem::ManuallyDrop,
};

use crate::{Wren, modules::ModuleLoader};

fn write<U, M, W: std::io::Write>(vm: &mut Wren<U, M, W>, text: &CStr) {
    vm.writer_mut().write_all(text.to_bytes()).unwrap();
}

fn error<U, M, W>(_vm: &mut Wren<U, M, W>, error: ErrorType<'_>) {
    match error {
        ErrorType::Compile {
            module,
            line,
            message,
        } => println!("[{module:#?} line {line}] [Error] {message:#?}"),
        ErrorType::Runtime { message } => println!("[Runtime Error] {message:#?}"),
        ErrorType::StackTrace {
            module,
            line,
            method,
        } => println!("[{module:#?} line {line}] in {method:#?}"),
    }
}

fn load_module<U, M, W>(vm: &mut Wren<U, M, W>, name: &CStr) -> Option<CString>
where
    M: ModuleLoader,
{
    let name = name.to_str().unwrap();

    let x = vm.module_loader_mut().load(name)?;

    Some(CString::new(x.to_owned()).unwrap())
}

pub unsafe extern "C" fn write_fn<U, M, W>(vm: *mut sys::WrenVM, text: *const i8)
where
    W: std::io::Write,
{
    let mut vm = unsafe { Wren::from_vm_ptr(vm) };
    let text = unsafe { CStr::from_ptr(text) };

    write::<U, M, W>(&mut vm, text);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum ErrorType<'s> {
    Compile {
        module: &'s CStr,
        line: i32,
        message: &'s CStr,
    },
    Runtime {
        message: &'s CStr,
    },
    StackTrace {
        module: &'s CStr,
        line: i32,
        method: &'s CStr,
    },
}

pub unsafe extern "C" fn error_fn<U, M, W>(
    vm: *mut sys::WrenVM,
    error_type: sys::WrenErrorType,
    module: *const i8,
    line: i32,
    message: *const i8,
) {
    let mut vm = unsafe { Wren::from_vm_ptr(vm) };

    let err = match error_type {
        sys::WrenErrorType::WREN_ERROR_COMPILE => ErrorType::Compile {
            module: unsafe { CStr::from_ptr(module) },
            line,
            message: unsafe { CStr::from_ptr(message) },
        },
        sys::WrenErrorType::WREN_ERROR_RUNTIME => ErrorType::Runtime {
            message: unsafe { CStr::from_ptr(message) },
        },
        sys::WrenErrorType::WREN_ERROR_STACK_TRACE => ErrorType::StackTrace {
            module: unsafe { CStr::from_ptr(module) },
            line,
            method: unsafe { CStr::from_ptr(message) },
        },

        _ => unreachable!(),
    };

    error::<U, M, W>(&mut vm, err);
}

pub unsafe extern "C" fn load_module_fn<U, M, W>(
    vm: *mut sys::WrenVM,
    name: *const i8,
) -> sys::WrenLoadModuleResult
where
    M: ModuleLoader,
{
    let mut vm = unsafe { Wren::from_vm_ptr(vm) };
    let name = unsafe { CStr::from_ptr(name) };

    let module = load_module::<U, M, W>(&mut vm, name);

    unsafe extern "C" fn on_complete(
        _vm: *mut sys::WrenVM,
        _module: *const i8,
        result: sys::WrenLoadModuleResult,
    ) {
        let _ = unsafe { CString::from_raw(result.source.cast_mut()) };
    }

    match module {
        Some(source) => sys::WrenLoadModuleResult {
            source: source.into_raw(),
            onComplete: Some(on_complete),
            userData: std::ptr::null_mut(),
        },
        None => sys::WrenLoadModuleResult {
            source: std::ptr::null(),
            onComplete: None,
            userData: std::ptr::null_mut(),
        },
    }
}
