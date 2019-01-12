pub trait ModuleReader {
    fn load_module_source(&self, relative_path: &str) -> Option<String>;
    fn push_module_path(&mut self, path: &str);
    fn pop_module_path(&mut self);
    fn compose_path(&self, relative_path: &str) -> String;
}
