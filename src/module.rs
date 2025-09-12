use std::borrow::Cow;

trait ModuleLoader {
    fn resolve(&self, importer: &str, module: &str) -> Option<Cow<'_, str>>;

    fn load(&self, module: &str) -> Option<Cow<'_, str>>;
}
