#![allow(clippy::not_unsafe_ptr_arg_deref)]

extern crate kaiju_core as core;
extern crate kaiju_vm_core as vm_core;
extern crate libc;
#[macro_use]
extern crate lazy_static;

use core::error::*;
use std::collections::HashMap;
use std::ffi::CString;
use std::mem::{size_of, transmute};
use std::ptr::{copy_nonoverlapping, null, null_mut};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use vm_core::processor::{OpAction, Processor};
use vm_core::state::Value;
use vm_core::vm::Vm;

lazy_static! {
    static ref HANDLE_GEN: AtomicUsize = AtomicUsize::new(0);
    static ref VMS: Mutex<HashMap<Handle, Vm>> = Mutex::new(HashMap::new());
    static ref PROCESS_OP: Mutex<Option<(usize, KaijuFuncProcessOp)>> = Mutex::new(None);
    static ref VM: Mutex<Option<&'static mut Vm>> = Mutex::new(None);
    static ref OP_ACTION: Mutex<OpAction> = Mutex::new(OpAction::None);
}

type KaijuFuncProcessOp = fn(
    context: *mut libc::c_void,
    op: *const libc::c_char,
    params: *const usize,
    params_count: usize,
    targets: *const usize,
    targets_count: usize,
);

type Handle = usize;

struct ExternalProcessor {}

impl Processor for ExternalProcessor {
    fn process_op(
        op: &String,
        params: &[usize],
        targets: &[usize],
        vm: &mut Vm,
    ) -> SimpleResult<OpAction> {
        if let Some((context, on_process_op)) = *PROCESS_OP.lock().unwrap() {
            let csop = CString::new(op.as_str()).unwrap();
            {
                *VM.lock().unwrap() = Some(unsafe { transmute::<&mut Vm, &'static mut Vm>(vm) });
            }
            on_process_op(
                context as *mut libc::c_void,
                csop.as_ptr(),
                params.as_ptr(),
                params.len(),
                targets.as_ptr(),
                targets.len(),
            );
            {
                *VM.lock().unwrap() = None;
            }
            let mut action = OP_ACTION.lock().unwrap();
            let a = *action;
            *action = OpAction::None;
            Ok(a)
        } else {
            Err(SimpleError::new("There is no active processor".to_owned()))
        }
    }
}

#[repr(C)]
pub struct KaijuInfoState {
    pub stack_size: usize,
    pub memory_size: usize,
    pub all_size: usize,
    pub stack_free: usize,
    pub memory_free: usize,
    pub all_free: usize,
}

#[no_mangle]
pub extern "C" fn kaiju_start_program(
    bytes: *const libc::c_uchar,
    size: usize,
    entry: *const libc::c_char,
    memsize: usize,
    stacksize: usize,
    error: fn(*mut libc::c_void, *const libc::c_char),
    error_context: *mut libc::c_void,
) -> Handle {
    if bytes.is_null()
        || size == 0
        || entry.is_null()
        || memsize == 0
        || stacksize == 0
        || (error as *const libc::c_void).is_null()
    {
        if !(error as *const libc::c_void).is_null() {
            let err = CString::new("Some of parameters are zeros or null pointers!").unwrap();
            error(error_context, err.as_ptr());
        }
        return 0;
    }
    let bytes = bytes_from_raw(bytes, size as usize);
    match Vm::from_bytes(bytes, stacksize as usize, memsize as usize) {
        Ok(mut vm) => match vm.start(&string_from_raw_unsized(entry as *const libc::c_uchar)) {
            Ok(_) => {
                let handle = {
                    let gen = HANDLE_GEN.load(Ordering::Relaxed);
                    let handle = gen + 1;
                    HANDLE_GEN.store(handle, Ordering::Relaxed);
                    handle
                };
                VMS.lock().unwrap().insert(handle, vm);
                handle
            }
            Err(err) => {
                let err = CString::new(err.message).unwrap();
                error(error_context, err.as_ptr());
                0
            }
        },
        Err(err) => {
            let err = CString::new(err.message).unwrap();
            error(error_context, err.as_ptr());
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn kaiju_run_program(
    bytes: *const libc::c_uchar,
    size: usize,
    entry: *const libc::c_char,
    memsize: usize,
    stacksize: usize,
    on_process_op: fn(
        *mut libc::c_void,
        *const libc::c_char,
        *const usize,
        usize,
        *const usize,
        usize,
    ),
    processor_context: *mut libc::c_void,
    error: fn(*mut libc::c_void, *const libc::c_char),
    error_context: *mut libc::c_void,
) -> bool {
    if bytes.is_null()
        || size == 0
        || entry.is_null()
        || memsize == 0
        || stacksize == 0
        || (on_process_op as *const libc::c_void).is_null()
        || (error as *const libc::c_void).is_null()
    {
        if !(error as *const libc::c_void).is_null() {
            let err = CString::new("Some of parameters are zeros or null pointers!").unwrap();
            error(error_context, err.as_ptr());
        }
        return false;
    }
    let bytes = bytes_from_raw(bytes, size as usize);
    match Vm::from_bytes(bytes, stacksize as usize, memsize as usize) {
        Ok(mut vm) => {
            {
                *PROCESS_OP.lock().unwrap() = Some((processor_context as usize, on_process_op));
            }
            let result = match vm
                .run::<ExternalProcessor>(&string_from_raw_unsized(entry as *const libc::c_uchar))
            {
                Ok(_) => true,
                Err(err) => {
                    let err = CString::new(err.message).unwrap();
                    error(error_context, err.as_ptr());
                    false
                }
            };
            {
                *PROCESS_OP.lock().unwrap() = None;
            }
            result
        }
        Err(err) => {
            let err = CString::new(err.message).unwrap();
            error(error_context, err.as_ptr());
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn kaiju_resume_program(
    handle: Handle,
    on_process_op: fn(
        *mut libc::c_void,
        *const libc::c_char,
        *const usize,
        usize,
        *const usize,
        usize,
    ),
    processor_context: *mut libc::c_void,
    error: fn(*mut libc::c_void, *const libc::c_char),
    error_context: *mut libc::c_void,
) -> bool {
    if (on_process_op as *const libc::c_void).is_null() || (error as *const libc::c_void).is_null()
    {
        if !(error as *const libc::c_void).is_null() {
            let err = CString::new("Some of parameters are null pointers!").unwrap();
            error(error_context, err.as_ptr());
        }
        return false;
    }
    let mut vms = VMS.lock().unwrap();
    match vms.get_mut(&handle) {
        Some(vm) => {
            if !vm.can_resume() {
                vms.remove(&handle);
                return false;
            }
            {
                *PROCESS_OP.lock().unwrap() = Some((processor_context as usize, on_process_op));
            }
            let result = match vm.resume::<ExternalProcessor>() {
                Ok(_) => true,
                Err(err) => {
                    let err = CString::new(err.message).unwrap();
                    error(error_context, err.as_ptr());
                    false
                }
            };
            {
                *PROCESS_OP.lock().unwrap() = None;
            }
            result
        }
        None => {
            let err = CString::new(format!("There is no VM with handle: {}", handle)).unwrap();
            error(error_context, err.as_ptr());
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn kaiju_consume_program(
    handle: Handle,
    on_process_op: fn(
        *mut libc::c_void,
        *const libc::c_char,
        *const usize,
        usize,
        *const usize,
        usize,
    ),
    processor_context: *mut libc::c_void,
    error: fn(*mut libc::c_void, *const libc::c_char),
    error_context: *mut libc::c_void,
) -> bool {
    if (on_process_op as *const libc::c_void).is_null() || (error as *const libc::c_void).is_null()
    {
        if !(error as *const libc::c_void).is_null() {
            let err = CString::new("Some of parameters are null pointers!").unwrap();
            error(error_context, err.as_ptr());
        }
        return false;
    }
    let mut vms = VMS.lock().unwrap();
    match vms.get_mut(&handle) {
        Some(vm) => {
            if !vm.can_resume() {
                vms.remove(&handle);
                return false;
            }
            {
                *PROCESS_OP.lock().unwrap() = Some((processor_context as usize, on_process_op));
            }
            let result = match vm.consume::<ExternalProcessor>() {
                Ok(_) => true,
                Err(err) => {
                    let err = CString::new(err.message).unwrap();
                    error(error_context, err.as_ptr());
                    false
                }
            };
            vms.remove(&handle);
            {
                *PROCESS_OP.lock().unwrap() = None;
            }
            result
        }
        None => {
            let err = CString::new(format!("There is no VM with handle: {}", handle)).unwrap();
            error(error_context, err.as_ptr());
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn kaiju_cancel_program(handle: Handle) {
    VMS.lock().unwrap().remove(&handle);
}

#[no_mangle]
pub extern "C" fn kaiju_fork_program(
    handle: Handle,
    entry: *const libc::c_char,
    memsize: usize,
    stacksize: usize,
    error: fn(*mut libc::c_void, *const libc::c_char),
    error_context: *mut libc::c_void,
) -> Handle {
    if entry.is_null() || memsize == 0 || stacksize == 0 || (error as *const libc::c_void).is_null()
    {
        if !(error as *const libc::c_void).is_null() {
            let err = CString::new("Some of parameters are zeros or null pointers!").unwrap();
            error(error_context, err.as_ptr());
        }
        return 0;
    }
    let mut vms = VMS.lock().unwrap();
    match vms.get(&handle) {
        Some(vm) => match vm.fork_advanced(stacksize as usize, memsize as usize) {
            Ok(mut vm) => match vm.start(&string_from_raw_unsized(entry as *const libc::c_uchar)) {
                Ok(_) => {
                    let handle = {
                        let gen = HANDLE_GEN.load(Ordering::Relaxed);
                        let handle = gen + 1;
                        HANDLE_GEN.store(handle, Ordering::Relaxed);
                        handle
                    };
                    vms.insert(handle, vm);
                    handle
                }
                Err(err) => {
                    let err = CString::new(err.message).unwrap();
                    error(error_context, err.as_ptr());
                    0
                }
            },
            Err(err) => {
                let err = CString::new(err.message).unwrap();
                error(error_context, err.as_ptr());
                0
            }
        },
        None => {
            let err = CString::new(format!("There is no VM with handle: {}", handle)).unwrap();
            error(error_context, err.as_ptr());
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn kaiju_with_program(
    handle: Handle,
    on_perform: fn(*mut libc::c_void),
    perform_context: *mut libc::c_void,
    error: fn(*mut libc::c_void, *const libc::c_char),
    error_context: *mut libc::c_void,
) -> bool {
    if (on_perform as *const libc::c_void).is_null() || (error as *const libc::c_void).is_null() {
        if !(error as *const libc::c_void).is_null() {
            let err = CString::new("Some of parameters are null pointers!").unwrap();
            error(error_context, err.as_ptr());
        }
        return false;
    }
    if VM.lock().unwrap().is_some() {
        return false;
    }
    let mut vms = VMS.lock().unwrap();
    match vms.get_mut(&handle) {
        Some(vm) => {
            {
                *VM.lock().unwrap() = Some(unsafe { transmute::<&mut Vm, &'static mut Vm>(vm) });
            }
            on_perform(perform_context);
            {
                *VM.lock().unwrap() = None;
            }
            true
        }
        None => {
            let err = CString::new(format!("There is no VM with handle: {}", handle)).unwrap();
            error(error_context, err.as_ptr());
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn kaiju_state_size() -> usize {
    if let Some(ref vm) = *VM.lock().unwrap() {
        return vm.state().all_size();
    }
    0
}

#[no_mangle]
pub extern "C" fn kaiju_state_ptr(address: usize) -> *const libc::c_void {
    if let Some(ref vm) = *VM.lock().unwrap() {
        let mem = vm.state().map_all();
        if address < mem.len() {
            return unsafe { mem.as_ptr().add(address) as *const libc::c_void };
        }
    }
    null()
}

#[no_mangle]
pub extern "C" fn kaiju_state_ptr_mut(address: usize) -> *mut libc::c_void {
    if let Some(ref mut vm) = *VM.lock().unwrap() {
        let mem = vm.state_mut().map_all_mut();
        if address < mem.len() {
            return unsafe { mem.as_mut_ptr().add(address) as *mut libc::c_void };
        }
    }
    null_mut()
}

#[no_mangle]
pub extern "C" fn kaiju_state_info(out_info: *mut KaijuInfoState) -> bool {
    if let Some(ref vm) = *VM.lock().unwrap() {
        unsafe {
            *out_info = KaijuInfoState {
                stack_size: vm.state().stack_size(),
                memory_size: vm.state().memory_size(),
                all_size: vm.state().all_size(),
                stack_free: vm.state().stack_free(),
                memory_free: vm.state().memory_free(),
                all_free: vm.state().all_free(),
            };
        }
        return true;
    }
    false
}

#[no_mangle]
pub extern "C" fn kaiju_state_alloc_stack(size: usize, out_address: *mut usize) -> bool {
    if let Some(ref mut vm) = *VM.lock().unwrap() {
        if let Ok(val) = vm.state_mut().alloc_stack_value(size) {
            unsafe {
                *out_address = val.address;
            }
            return true;
        }
    }
    false
}

#[no_mangle]
pub extern "C" fn kaiju_state_pop_stack(size: usize) -> bool {
    if let Some(ref mut vm) = *VM.lock().unwrap() {
        let pos = vm.state().stack_pos();
        if pos >= size && vm.state_mut().stack_reset(pos - size).is_ok() {
            return true;
        }
    }
    false
}

#[no_mangle]
pub extern "C" fn kaiju_state_stack_address(out_address: *mut usize) -> bool {
    if let Some(ref vm) = *VM.lock().unwrap() {
        unsafe {
            *out_address = vm.state().stack_pos();
        }
        return true;
    }
    false
}

#[no_mangle]
pub extern "C" fn kaiju_state_alloc_memory(size: usize, out_address: *mut usize) -> bool {
    if let Some(ref mut vm) = *VM.lock().unwrap() {
        let bs = size_of::<usize>();
        let val = vm.state_mut().alloc_memory_value(size + bs);
        if val.is_err() {
            return false;
        }
        let val = val.unwrap();
        if vm.state_mut().store_data(val.address, &size).is_err() {
            vm.state_mut().dealloc_memory_value(&val).unwrap_or(());
            return false;
        }
        unsafe {
            *out_address = val.address + bs;
        }
        return true;
    }
    false
}

#[no_mangle]
pub extern "C" fn kaiju_state_dealloc_memory(address: usize) -> bool {
    if let Some(ref mut vm) = *VM.lock().unwrap() {
        let bs = size_of::<usize>();
        let size = vm.state().load_data::<usize>(address - bs);
        if size.is_err() {
            return false;
        }
        let size = size.unwrap() + bs;
        if vm
            .state_mut()
            .dealloc_memory_value(&Value::new(address - bs, size))
            .is_ok()
        {
            return true;
        }
    }
    false
}

#[no_mangle]
pub extern "C" fn kaiju_context_go_to(label: *const libc::c_char) -> bool {
    if let Some(ref vm) = *VM.lock().unwrap() {
        if let Some(pos) = vm.find_label(&string_from_raw_unsized(label as *const libc::c_uchar)) {
            *OP_ACTION.lock().unwrap() = OpAction::GoTo(pos);
            return true;
        }
    }
    false
}

#[no_mangle]
pub extern "C" fn kaiju_context_return() {
    *OP_ACTION.lock().unwrap() = OpAction::Return;
}

fn bytes_from_raw(source: *const libc::c_uchar, size: usize) -> Vec<u8> {
    if source.is_null() || size == 0 {
        return vec![];
    }
    let mut result = vec![0; size];
    let target = result.as_mut_ptr();
    unsafe { copy_nonoverlapping(source, target, size) };
    result
}

fn string_from_raw_unsized(mut source: *const libc::c_uchar) -> String {
    if source.is_null() {
        return "".to_owned();
    }
    let mut bytes = vec![];
    unsafe {
        while *source != 0 {
            bytes.push(*source);
            source = source.add(1);
        }
    }
    let cstring = unsafe { CString::from_vec_unchecked(bytes) };
    cstring.into_string().unwrap()
}
