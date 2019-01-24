use crate::string_from_raw;
use compiler_core::module_reader::ModuleReader;
use relative_path::{RelativePath, RelativePathBuf};
use std::ffi::CString;

fn dir_path(path: &RelativePath) -> RelativePathBuf {
    let mut path = path.to_relative_path_buf();
    path.pop();
    path
}

pub struct FmModuleReader {
    context: *mut libc::c_void,
    serve: fn(*mut libc::c_void, *const libc::c_char, *mut usize) -> *const libc::c_uchar,
    path_stack: Vec<RelativePathBuf>,
}

impl FmModuleReader {
    pub fn new(
        context: *mut libc::c_void,
        serve: fn(*mut libc::c_void, *const libc::c_char, *mut usize) -> *const libc::c_uchar,
    ) -> Self {
        Self {
            context,
            serve,
            path_stack: vec![],
        }
    }
}

impl ModuleReader for FmModuleReader {
    fn load_module_source(&self, path: &str) -> Option<String> {
        if let Ok(path) = CString::new(path) {
            let mut size = 0;
            let buffer = (self.serve)(self.context, path.as_ptr(), &mut size);
            if buffer.is_null() {
                None
            } else {
                Some(string_from_raw(buffer, size as usize))
            }
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
