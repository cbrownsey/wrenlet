use std::borrow::Cow;

pub trait ModuleLoader {
    fn resolve(&self, importer: &str, module: &str) -> Option<Cow<'_, str>>;

    fn load(&self, module: &str) -> Option<Cow<'_, str>>;
}

pub struct Empty;

impl ModuleLoader for Empty {
    fn resolve(&self, _importer: &str, _module: &str) -> Option<Cow<'_, str>> {
        None
    }

    fn load(&self, _module: &str) -> Option<Cow<'_, str>> {
        None
    }
}
