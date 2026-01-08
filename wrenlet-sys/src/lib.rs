#![allow(
    dead_code,
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case
)]

// include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(not(feature = "wren_v0_4_0"))]
compile_error!("Some version of Wren must be enabled.");

#[cfg(feature = "wren_v0_4_0")]
include!("wren040.rs");
