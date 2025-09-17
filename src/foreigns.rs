use std::{alloc::Layout, any::TypeId, os::raw::c_void};

#[derive(Debug, Clone, Hash)]
pub struct ForeignClass {
    pub module: &'static str,
    pub name: &'static str,
    pub type_id: TypeId,
    pub layout: Layout,
    pub drop_fn: extern "C" fn(*mut c_void),
    pub methods: Box<[ForeignMethod]>,
}

impl ForeignClass {
    pub fn new_for<T: 'static>(
        module: &'static str,
        name: &'static str,
        methods: impl IntoIterator<Item = ForeignMethod>,
    ) -> ForeignClass {
        extern "C" fn drop_fn<T>(ptr: *mut c_void) {
            unsafe { std::ptr::drop_in_place::<T>(ptr.cast()) };
        }

        Self {
            module,
            name,
            type_id: TypeId::of::<T>(),
            layout: Layout::new::<T>(),
            drop_fn: drop_fn::<T>,
            methods: methods.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, Hash)]
pub struct ForeignMethod {
    name: &'static str,
    is_static: bool,
    implementation: extern "C" fn(*mut sys::WrenVM),
}
