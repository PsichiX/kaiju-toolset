use crate::compiler_core::module_reader::ModuleReader;
use relative_path::{RelativePath, RelativePathBuf};
use std::fs::read_to_string;

fn dir_path(path: &RelativePath) -> RelativePathBuf {
    let mut path = path.to_relative_path_buf();
    path.pop();
    path
}

#[derive(Default)]
pub struct FsModuleReader {
    path_stack: Vec<RelativePathBuf>,
}

impl ModuleReader for FsModuleReader {
    fn load_module_source(&self, path: &str) -> Option<String> {
        if let Ok(source) = read_to_string(path) {
            Some(source)
        } else {
            None
        }
    }

    fn push_module_path(&mut self, path: &str) {
        self.path_stack.push(dir_path(&RelativePath::new(path)));
    }

    fn pop_module_path(&mut self) {
        self.path_stack.pop();
    }

    fn compose_path(&self, relative_path: &str) -> String {
        let relative_path = RelativePathBuf::from(relative_path);
        if let Some(path) = self.path_stack.last() {
            let mut path = path.clone();
            path.push(relative_path);
            path
        } else {
            relative_path.to_relative_path_buf()
        }
        .normalize()
        .as_str()
        .to_owned()
    }
}
