// use std::{env, path::PathBuf};

fn main() {
    // let bindings = bindgen::builder()
    //     .use_core()
    //     .allowlist_item("(W|w)ren.*")
    //     .default_enum_style(bindgen::EnumVariation::NewType {
    //         is_bitfield: false,
    //         is_global: false,
    //     })
    //     .header("wren/wren-0.4.0.h")
    //     .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
    //     .generate()
    //     .expect("Unable to generate bindings.");

    // let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    // bindings
    //     .write_to_file(out_path.join("bindings.rs"))
    //     .expect("Couldn't write bindings.");

    let mut cc = cc::Build::new();

    cc.file("wren/wren-0.4.0.c").warnings(false);

    if cfg!(feature = "meta") {
        cc.define("WREN_OPT_META", Some("0"));
    }

    if cfg!(feature = "random") {
        cc.define("WREN_OPT_RANDOM", Some("0"));
    }

    cc.compile("wren");
}
