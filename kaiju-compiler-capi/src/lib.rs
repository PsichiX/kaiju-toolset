#![allow(clippy::not_unsafe_ptr_arg_deref)]

extern crate kaiju_compiler_core as compiler_core;
extern crate kaiju_core as core;
extern crate libc;
extern crate relative_path;

pub mod fm_module_reader;

use crate::fm_module_reader::*;
use compiler_core::*;
use core::assembly::*;
use core::program::*;
use core::validator::*;
use std::ffi::CString;
use std::ptr::{copy_nonoverlapping, null};

#[no_mangle]
pub extern "C" fn kaiju_compile_program_pst(
    input: *const libc::c_char,
    opsdesc: *const libc::c_char,
    pretty: bool,
    serve_file: fn(*mut libc::c_void, *const libc::c_char, *mut usize) -> *const libc::c_uchar,
    serve_context: *mut libc::c_void,
    result_file: fn(*mut libc::c_void, *const libc::c_uchar, usize),
    result_context: *mut libc::c_void,
    error: fn(*mut libc::c_void, *const libc::c_char),
    error_context: *mut libc::c_void,
) -> bool {
    if (serve_file as *const libc::c_void).is_null()
        || (result_file as *const libc::c_void).is_null()
        || (error as *const libc::c_void).is_null()
    {
        if !(error as *const libc::c_void).is_null() {
            let err = CString::new("Some callbacks are null pointers!").unwrap();
            error(error_context, err.as_ptr());
        }
        result_file(result_context, null(), 0);
        return false;
    }
    let mut size = 0;
    let buffer = serve_file(serve_context, opsdesc, &mut size);
    if !buffer.is_null() {
        match compile_ops_descriptor(&string_from_raw(buffer, size as usize)) {
            Ok(desc) => {
                match compile_program::<EmptyDeepValidator, _>(
                    &string_from_raw_unsized(input as *const u8),
                    FmModuleReader::new(serve_context, serve_file),
                    &desc,
                ) {
                    Ok(program) => match program.to_json(pretty) {
                        Ok(json) => {
                            let bytes = json.as_bytes();
                            result_file(result_context, bytes.as_ptr(), bytes.len());
                            true
                        }
                        Err(err) => {
                            let err = CString::new(format!("{}", err)).unwrap();
                            error(error_context, err.as_ptr());
                            result_file(result_context, null(), 0);
                            false
                        }
                    },
                    Err(err) => {
                        let err = CString::new(err.message).unwrap();
                        error(error_context, err.as_ptr());
                        result_file(result_context, null(), 0);
                        false
                    }
                }
            }
            Err(err) => {
                let opsdesc = string_from_raw_unsized(opsdesc as *const u8);
                let err = CString::new(format!("{}: {}", opsdesc, err.pretty)).unwrap();
                error(error_context, err.as_ptr());
                result_file(result_context, null(), 0);
                false
            }
        }
    } else {
        let opsdesc = string_from_raw_unsized(opsdesc as *const u8);
        let err = CString::new(format!("Could not read file: {}", opsdesc)).unwrap();
        error(error_context, err.as_ptr());
        result_file(result_context, null(), 0);
        false
    }
}

#[no_mangle]
pub extern "C" fn kaiju_compile_program_bin(
    input: *const libc::c_char,
    opsdesc: *const libc::c_char,
    serve_file: fn(*mut libc::c_void, *const libc::c_char, *mut usize) -> *const libc::c_uchar,
    serve_context: *mut libc::c_void,
    result_file: fn(*mut libc::c_void, *const libc::c_uchar, usize),
    result_context: *mut libc::c_void,
    error: fn(*mut libc::c_void, *const libc::c_char),
    error_context: *mut libc::c_void,
) -> bool {
    if (serve_file as *const libc::c_void).is_null()
        || (result_file as *const libc::c_void).is_null()
        || (error as *const libc::c_void).is_null()
    {
        if !(error as *const libc::c_void).is_null() {
            let err = CString::new("Some callbacks are null pointers!").unwrap();
            error(error_context, err.as_ptr());
        }
        result_file(result_context, null(), 0);
        return false;
    }
    let mut size = 0;
    let buffer = serve_file(serve_context, opsdesc, &mut size);
    if !buffer.is_null() {
        match compile_ops_descriptor(&string_from_raw(buffer, size as usize)) {
            Ok(desc) => {
                match compile_program::<EmptyDeepValidator, _>(
                    &string_from_raw_unsized(input as *const u8),
                    FmModuleReader::new(serve_context, serve_file),
                    &desc,
                ) {
                    Ok(program) => match encode_assembly(&program, &desc) {
                        Ok(bytes) => {
                            result_file(result_context, bytes.as_ptr(), bytes.len());
                            true
                        }
                        Err(err) => {
                            let err = CString::new(err.message).unwrap();
                            error(error_context, err.as_ptr());
                            result_file(result_context, null(), 0);
                            false
                        }
                    },
                    Err(err) => {
                        let err = CString::new(err.message).unwrap();
                        error(error_context, err.as_ptr());
                        result_file(result_context, null(), 0);
                        false
                    }
                }
            }
            Err(err) => {
                let opsdesc = string_from_raw_unsized(opsdesc as *const u8);
                let err = CString::new(format!("{}: {}", opsdesc, err.pretty)).unwrap();
                error(error_context, err.as_ptr());
                result_file(result_context, null(), 0);
                false
            }
        }
    } else {
        let opsdesc = string_from_raw_unsized(opsdesc as *const u8);
        let err = CString::new(format!("Could not read file: {}", opsdesc)).unwrap();
        error(error_context, err.as_ptr());
        result_file(result_context, null(), 0);
        false
    }
}

pub fn bytes_from_raw(source: *const libc::c_uchar, size: usize) -> Vec<u8> {
    if source.is_null() || size == 0 {
        return vec![];
    }
    let mut result = vec![0; size];
    let target = result.as_mut_ptr();
    unsafe {
        copy_nonoverlapping(source, target, size);
    }
    result
}

pub fn string_from_raw(source: *const libc::c_uchar, size: usize) -> String {
    let bytes = bytes_from_raw(source, size);
    unsafe {
        let cstring = CString::from_vec_unchecked(bytes);
        cstring.into_string().unwrap()
    }
}

pub fn string_from_raw_unsized(mut source: *const libc::c_uchar) -> String {
    if source.is_null() {
        return "".to_owned();
    }
    let mut bytes = vec![];
    unsafe {
        while *source != 0 {
            bytes.push(*source);
            source = source.add(1);
        }
        let cstring = CString::from_vec_unchecked(bytes);
        cstring.into_string().unwrap()
    }
}
