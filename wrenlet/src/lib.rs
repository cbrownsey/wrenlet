#![feature(default_field_values, unsafe_cell_access)]
#![allow(dead_code, unused_imports)]

pub mod modules;

mod c_functions;
mod erased_rc;
mod error;
mod foreigns;
mod raw;
mod values;
mod wren;

pub use wren::{Wren, WrenConfig};

#[cfg(test)]
mod tests {
    use crate::modules::Empty;

    use super::*;

    #[test]
    fn foreign_classes() {
        const SOURCE: &str = r#"
            foreign class Test {
                construct new() {}
            }

            Test.new()
        "#;

        let mut wren = Wren::new_with(WrenConfig {
            user_data: (),
            module_loader: Empty,
            writer: Vec::new(),
            ..
        });

        wren.interpret("core", SOURCE).unwrap();

        let (_, _, buf) = wren.into_parts();

        assert_eq!(buf, b"Hello, World!");
    }
}
