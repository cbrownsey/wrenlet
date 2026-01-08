use std::borrow::Cow;

use crate::Wren;

pub trait ModuleLoader {
    fn load(&mut self, name: &str) -> Option<Cow<'_, [u8]>>;
}

pub struct Empty;

impl ModuleLoader for Empty {
    fn load(&mut self, _name: &str) -> Option<Cow<'_, [u8]>> {
        None
    }
}
