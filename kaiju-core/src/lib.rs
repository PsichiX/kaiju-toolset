#![recursion_limit = "1024"]

extern crate pest;
#[macro_use]
extern crate pest_derive;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate byteorder;
extern crate serde_json;

pub mod assembly;
pub mod ast;
pub mod error;
pub mod parser;
pub mod program;
pub mod utils;
pub mod validator;
pub mod vm;
