struct ForeignClassVTable {
    module: &'static str,
    name: &'static str,
}

struct ForeignMethod {
    is_static: bool,
    name: &'static str,
    arity: u8,
    function: extern "C" fn(*mut sys::WrenVM),
}

trait WrenForeignMethod {
    extern "C" fn call(vm: *mut sys::WrenVM);
}

// fn test() {}

// expands into

fn test() {}

struct __WrenForeignMethod__test;

impl WrenForeignMethod for __WrenForeignMethod__test {
    extern "C" fn call(vm: *mut sys::WrenVM) {
        test()
    }
}
