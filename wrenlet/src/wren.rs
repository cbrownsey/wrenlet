use std::{
    cell::UnsafeCell,
    ffi::{CStr, CString},
    io::Stdout,
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ptr::NonNull,
};

use crate::{
    erased_rc::{self, ErasedRc},
    error::InterpretError,
    modules::{Empty, ModuleLoader},
    raw::WrenPtr,
    values::{FromWren, IntoWren, Type},
};

pub struct Wren<U, M = Empty, W = Stdout> {
    pub(crate) wren: WrenPtr,
    _marker: PhantomData<(U, M, W)>,
}

impl Default for Wren<()> {
    fn default() -> Self {
        Self::new()
    }
}

impl Wren<()> {
    pub fn new() -> Wren<()> {
        Wren::new_with(WrenConfig {
            user_data: (),
            module_loader: Empty,
            writer: std::io::stdout(),
            ..
        })
    }
}

impl<U, M, W> Wren<U, M, W> {
    /// Creates a new `Wren` instance from the given pointer.
    ///
    /// # Safety
    /// This function may only be called when there are no extant references to the associated userdata.
    pub(crate) unsafe fn from_vm_ptr(ptr: *mut sys::WrenVM) -> ManuallyDrop<Wren<U, M, W>> {
        ManuallyDrop::new(Wren {
            wren: unsafe { WrenPtr::from_ptr(ptr) },
            _marker: PhantomData,
        })
    }

    pub(crate) fn slot_count(&self) -> usize {
        unsafe { self.wren.get_slot_count() }.try_into().unwrap()
    }

    pub(crate) fn ensure_slots(&self, slots: usize) {
        let slots: i32 = slots.try_into().unwrap();
        let old_count = unsafe { self.wren.get_slot_count() };

        unsafe { self.wren.ensure_slots(slots) };

        for idx in old_count..slots {
            unsafe { self.wren.set_slot_null(idx) };
        }
    }

    pub(crate) fn iter_slot_types(&self) -> impl Iterator<Item = Type> + use<'_> {
        let wren_ptr = self.wren;
        (0..self.slot_count() as i32).map(move |idx| wren_ptr.get_slot_type(idx))
    }

    /// Retrieves the underlying pointer to the raw [`WrenVM`].
    ///
    /// [`WrenVM`]: sys::WrenVM
    pub(crate) fn vm_ptr(&self) -> *mut sys::WrenVM {
        self.wren.as_ptr()
    }

    /// Creates a new instance of `Wren` with the given WrenConfig.
    pub fn new_with(config: WrenConfig<U, M, W>) -> Self
    where
        M: ModuleLoader,
        W: std::io::Write,
    {
        let mut conf = MaybeUninit::uninit();

        unsafe { sys::wrenInitConfiguration(conf.as_mut_ptr()) };

        let peripherals = (config.user_data, config.module_loader, config.writer);

        let user_data = ErasedRc::new(UnsafeCell::new(MaybeUninit::new(peripherals)));
        let user_data_ptr = unsafe { user_data.into_raw::<UnsafeCell<MaybeUninit<(U, M, W)>>>() }
            .cast::<std::ffi::c_void>();

        let conf = sys::WrenConfiguration {
            initialHeapSize: config.initial_heap_size,
            minHeapSize: config.min_heap_size,
            heapGrowthPercent: config.heap_growth_percent as i32,

            writeFn: Some(crate::c_functions::write_fn::<U, M, W>),
            errorFn: Some(crate::c_functions::error_fn::<U, M, W>),
            loadModuleFn: Some(crate::c_functions::load_module_fn::<U, M, W>),

            userData: user_data_ptr,

            ..unsafe { conf.assume_init() }
        };

        Wren {
            wren: unsafe { WrenPtr::new(conf) },
            _marker: PhantomData,
        }
    }

    /// Interpret the given source code in the context of a module.
    pub fn interpret(&mut self, module: &str, source: &str) -> Result<(), InterpretError> {
        let module = CString::new(module).unwrap();
        let source = CString::new(source).unwrap();

        let result = unsafe { self.wren.interpret(&module, &source) };

        match result {
            crate::raw::InterpretResult::Success => Ok(()),
            crate::raw::InterpretResult::Compile => Err(InterpretError::Compiletime),
            crate::raw::InterpretResult::Runtime => Err(InterpretError::Runtime),
        }
    }

    pub fn has_module(&self, module: &str) -> bool {
        let module = CString::new(module).unwrap();

        unsafe { self.wren.has_module(&module) }
    }

    pub fn has_variable(&self, module: &str, name: &str) -> bool {
        let module = CString::new(module).unwrap();
        let name = CString::new(name).unwrap();

        (unsafe { self.wren.has_module(&module) })
            && unsafe { self.wren.has_variable(&module, &name) }
    }

    /// Sets the given slot in the slot array to the value given.
    ///
    /// This function will set the `T::SCRATCH_SLOTS` slots after the given slot to null.
    pub fn set_slot<T>(&mut self, slot: usize, value: T) -> Result<(), crate::error::IntoWrenError>
    where
        T: IntoWren<U, M, W>,
    {
        self.ensure_slots(slot + T::SCRATCH_SPACE);

        value.into_wren(self, slot)
    }

    pub fn get_slot<T>(&mut self, slot: usize) -> Result<T, crate::error::FromWrenError>
    where
        T: FromWren<U, M, W>,
    {
        self.ensure_slots(slot + T::SCRATCH_SPACE);

        T::from_wren(wren, slot)
    }

    #[must_use]
    pub fn into_parts(mut self) -> (U, M, W) {
        let peripherals = unsafe { self.take_peripherals() };
        unsafe { self.release() };
        std::mem::forget(self);
        peripherals
    }

    fn peripherals_ptr(&self) -> *mut UnsafeCell<MaybeUninit<(U, M, W)>> {
        unsafe { self.wren.get_user_data() }
    }

    fn peripherals(&self) -> &(U, M, W) {
        unsafe {
            (&*self.peripherals_ptr())
                .as_ref_unchecked()
                .assume_init_ref()
        }
    }

    fn peripherals_mut(&mut self) -> &mut (U, M, W) {
        unsafe {
            (&*self.peripherals_ptr())
                .as_mut_unchecked()
                .assume_init_mut()
        }
    }

    pub fn user_data(&self) -> &U {
        &self.peripherals().0
    }

    pub fn user_data_mut(&mut self) -> &mut U {
        &mut self.peripherals_mut().0
    }

    pub fn module_loader(&self) -> &M {
        &self.peripherals().1
    }

    pub fn module_loader_mut(&mut self) -> &mut M {
        &mut self.peripherals_mut().1
    }

    pub fn writer(&self) -> &W {
        &self.peripherals().2
    }

    pub fn writer_mut(&mut self) -> &mut W {
        &mut self.peripherals_mut().2
    }

    unsafe fn take_peripherals(&mut self) -> (U, M, W) {
        unsafe {
            (&*self.peripherals_ptr())
                .as_ref_unchecked()
                .assume_init_read()
        }
    }

    unsafe fn release(&mut self) {
        let erased_rc = unsafe { ErasedRc::from_raw(self.peripherals_ptr()) };

        if erased_rc.ref_count() == 1 {
            unsafe { self.wren.free_vm() };
        }
    }
}

impl<U, M, W> Drop for Wren<U, M, W> {
    fn drop(&mut self) {
        unsafe { self.take_peripherals() };
        unsafe { self.release() };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct WrenConfig<U, M, W> {
    pub initial_heap_size: usize = 1024 * 1024 * 10,
    pub min_heap_size: usize = 1024 * 1024,
    pub heap_growth_percent: u32 = 50,

    pub user_data: U,
    pub module_loader: M,
    pub writer: W,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_test() {
        let _ = Wren::new();
    }

    #[test]
    fn basic_execution() {
        let mut wren = Wren::new_with(WrenConfig {
            user_data: (),
            module_loader: Empty,
            writer: Vec::new(),
            ..
        });

        wren.interpret("main", r#"System.write("Hello, World!")"#)
            .unwrap();

        let (_, _, buf) = wren.into_parts();

        assert_eq!(buf, b"Hello, World!");
    }

    #[test]
    fn minimal_module_loading() {
        const MODULE_SOURCE: &str = r#"
            class Say {
                static hello() {
                    System.write("Hello, World!")
                }
            }
        "#;

        struct StaticModule {
            name: &'static str,
            source: &'static str,
        }

        impl ModuleLoader for StaticModule {
            fn load(&mut self, name: &str) -> Option<std::borrow::Cow<'_, [u8]>> {
                assert_eq!(name, self.name);

                let x: &'static [u8] = self.source.as_bytes();

                Some(std::borrow::Cow::Borrowed(x))
            }
        }

        let mut wren = Wren::new_with(WrenConfig {
            user_data: (),
            module_loader: StaticModule {
                name: "module",
                source: MODULE_SOURCE,
            },
            writer: Vec::new(),
            ..
        });

        wren.interpret(
            "main",
            r#"
            import "module" for Say

            Say.hello()
        "#,
        )
        .unwrap();

        let (_, _, buf) = wren.into_parts();

        assert_eq!(buf, b"Hello, World!");
    }

    #[test]
    fn peripherals() {
        let mut wren = Wren::new();

        let _user_data = wren.user_data();
        let _user_data = wren.user_data_mut();

        let _module_loader = wren.module_loader();
        let _module_loader = wren.module_loader_mut();

        let _writer = wren.writer();
        let _writer = wren.writer_mut();
    }
}
