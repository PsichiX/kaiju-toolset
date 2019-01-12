use std::io::Error as IoError;
use std::result::Result as StdResult;

#[derive(Debug)]
pub struct CompilationError {
    pub message: String,
    pub location: (usize, usize),
    pub line: (usize, usize),
    pub column: (usize, usize),
    pub pretty: String,
}

pub type CompilationResult<T> = StdResult<T, CompilationError>;

#[derive(Debug)]
pub struct SimpleError {
    pub message: String,
}

impl SimpleError {
    #[inline]
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

pub type SimpleResult<T> = StdResult<T, SimpleError>;

impl From<IoError> for SimpleError {
    fn from(error: IoError) -> Self {
        Self::new(format!("{}", error))
    }
}
