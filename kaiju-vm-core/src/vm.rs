use crate::processor::{OpAction, Processor};
use crate::state::{State, Value};
use byteorder::{BigEndian, ReadBytesExt};
use compiler_core::compile_program;
use compiler_core::module_reader::ModuleReader;
use core::assembly::{encode_assembly, OpIndex};
use core::error::*;
use core::program::OpsDescriptor;
use core::validator::DeepValidator;
use core::vm::{Data, VmAssembly};
use std::ffi::CString;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::mem::size_of;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExecutionStage {
    Uninitialized,
    Running,
    Complete,
}

#[derive(Debug, Clone)]
pub struct CallStackFrame {
    function: usize,
    address: usize,
    params_stackpos: usize,
    result: Option<Value>,
    locals: Option<Value>,
    op_target_addresses: Vec<usize>,
    op_param_addresses: Vec<usize>,
    op_stackpos: usize,
}

impl CallStackFrame {
    #[inline]
    pub(crate) fn new(
        function: usize,
        address: usize,
        params_stackpos: usize,
        result: Option<Value>,
        locals: Option<Value>,
    ) -> Self {
        Self {
            function,
            address,
            params_stackpos,
            result,
            locals,
            op_target_addresses: vec![],
            op_param_addresses: vec![],
            op_stackpos: 0,
        }
    }

    #[inline]
    pub fn function(&self) -> usize {
        self.function
    }

    #[inline]
    pub fn address(&self) -> usize {
        self.address
    }

    #[inline]
    pub fn params_stackpos(&self) -> usize {
        self.params_stackpos
    }

    #[inline]
    pub fn result(&self) -> &Option<Value> {
        &self.result
    }

    #[inline]
    pub fn locals(&self) -> &Option<Value> {
        &self.locals
    }

    #[inline]
    pub fn op_target_addresses(&self) -> &[usize] {
        &self.op_target_addresses
    }

    #[inline]
    pub fn op_param_addresses(&self) -> &[usize] {
        &self.op_param_addresses
    }

    pub(crate) fn collect_params_targets(&mut self) -> (Vec<usize>, Vec<usize>) {
        let params = self.op_param_addresses.clone();
        let targets = self.op_target_addresses.clone();
        self.op_param_addresses.clear();
        self.op_target_addresses.clear();
        (params, targets)
    }

    pub(crate) fn duplicate(&self) -> Self {
        Self {
            function: self.function,
            address: self.address,
            params_stackpos: self.params_stackpos,
            result: self.result,
            locals: self.locals,
            op_target_addresses: vec![],
            op_param_addresses: vec![],
            op_stackpos: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Vm {
    assembly: VmAssembly,
    state: State,
    stage: ExecutionStage,
    callstack: Vec<CallStackFrame>,
    data: Option<Value>,
    globals: Option<Value>,
    pointers: Vec<usize>,
}

impl Vm {
    #[allow(clippy::new_ret_no_self)]
    #[inline]
    pub fn new(assembly: VmAssembly, stack_size: usize, memory_size: usize) -> SimpleResult<Self> {
        if stack_size % 4 != 0 {
            return Err(SimpleError::new(format!(
                "Stack size is not aligned to 4 bytes: {}",
                stack_size
            )));
        }
        if memory_size % 4 != 0 {
            return Err(SimpleError::new(format!(
                "Memory size is not aligned to 4 bytes: {}",
                memory_size
            )));
        }
        if memory_size + stack_size > usize::max_value() {
            return Err(SimpleError::new(format!(
                "Memory and stack sizes combined cannot exceed machine pointer range: {}",
                usize::max_value()
            )));
        }
        Ok(Self {
            assembly,
            state: State::new(stack_size, memory_size),
            stage: ExecutionStage::Uninitialized,
            callstack: vec![],
            data: None,
            globals: None,
            pointers: vec![],
        })
    }

    #[inline]
    pub fn from_bytes(bytes: Vec<u8>, stack_size: usize, memory_size: usize) -> SimpleResult<Self> {
        Self::new(VmAssembly::new(bytes)?, stack_size, memory_size)
    }

    #[inline]
    pub fn from_stream(
        stream: &mut Read,
        stack_size: usize,
        memory_size: usize,
    ) -> SimpleResult<Self> {
        let mut bytes = vec![];
        stream.read_to_end(&mut bytes)?;
        Self::from_bytes(bytes, stack_size, memory_size)
    }

    #[inline]
    pub fn from_source<V, R>(
        entry_path: &str,
        module_reader: R,
        ops_descriptor: &OpsDescriptor,
        stack_size: usize,
        memory_size: usize,
    ) -> SimpleResult<Self>
    where
        V: DeepValidator,
        R: ModuleReader,
    {
        let program = compile_program::<V, R>(entry_path, module_reader, ops_descriptor)?;
        let bytes = encode_assembly(&program, ops_descriptor)?;
        Self::from_bytes(bytes, stack_size, memory_size)
    }

    #[inline]
    pub fn fork(&self) -> SimpleResult<Self> {
        self.fork_advanced(self.state.stack_size(), self.state.memory_size())
    }

    #[inline]
    pub fn fork_advanced(&self, stack_size: usize, memory_size: usize) -> SimpleResult<Self> {
        if stack_size % 4 != 0 {
            return Err(SimpleError::new(format!(
                "Stack size is not aligned to 4 bytes: {}",
                stack_size
            )));
        }
        if memory_size % 4 != 0 {
            return Err(SimpleError::new(format!(
                "Memory size is not aligned to 4 bytes: {}",
                memory_size
            )));
        }
        if memory_size + stack_size > usize::max_value() {
            return Err(SimpleError::new(format!(
                "Memory and stack sizes combined cannot exceed machine pointer range: {}",
                usize::max_value()
            )));
        }
        Ok(Self {
            assembly: self.assembly.clone(),
            state: State::new(stack_size, memory_size),
            stage: ExecutionStage::Uninitialized,
            callstack: vec![],
            data: None,
            globals: None,
            pointers: vec![],
        })
    }

    #[inline]
    pub fn assembly(&self) -> &VmAssembly {
        &self.assembly
    }

    #[inline]
    pub fn state(&self) -> &State {
        &self.state
    }

    #[inline]
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    #[inline]
    pub fn stage(&self) -> ExecutionStage {
        self.stage
    }

    pub fn start(&mut self, entry: &str) -> SimpleResult<()> {
        if self.stage == ExecutionStage::Uninitialized {
            let f = {
                if let Some(f) = self.assembly.function_by_id(entry) {
                    if !f.params().is_empty() {
                        return Err(SimpleError::new(format!(
                            "Trying to call entry function `{}` that does have params",
                            entry
                        )));
                    }
                    if f.typeid().is_some() {
                        return Err(SimpleError::new(
                            "Trying to run function with return value".to_owned(),
                        ));
                    }
                    f.index()
                } else {
                    return Err(SimpleError::new(format!(
                        "Trying to start non-existing function: {}",
                        entry
                    )));
                }
            };
            let vd = self.alloc_data()?;
            let vg = self
                .state
                .alloc_memory_value(self.assembly.globals_size())?;
            self.data = vd;
            self.globals = Some(vg);
            self.stage = ExecutionStage::Running;
            self.call_function(f)?;
            Ok(())
        } else {
            Err(SimpleError::new(
                "Trying to start running or complete VM".to_owned(),
            ))
        }
    }

    #[inline]
    pub fn can_resume(&self) -> bool {
        self.stage == ExecutionStage::Running && !self.callstack.is_empty()
    }

    pub fn resume<P>(&mut self) -> SimpleResult<()>
    where
        P: Processor,
    {
        if self.stage == ExecutionStage::Running {
            while self.resume_op::<P>()? {}
            Ok(())
        } else {
            Err(SimpleError::new(
                "Trying to resume uninitialized or complete VM".to_owned(),
            ))
        }
    }

    #[inline]
    pub fn consume<P>(&mut self) -> SimpleResult<()>
    where
        P: Processor,
    {
        if self.stage == ExecutionStage::Running {
            while self.can_resume() {
                self.resume::<P>()?;
            }
            Ok(())
        } else {
            Err(SimpleError::new(
                "Trying to consume uninitialized or complete VM".to_owned(),
            ))
        }
    }

    #[inline]
    pub fn run<P>(&mut self, entry: &str) -> SimpleResult<()>
    where
        P: Processor,
    {
        self.start(entry)?;
        self.consume::<P>()
    }

    pub fn find_label(&self, id: &str) -> Option<usize> {
        if let Some((_, func, _)) = self.location() {
            if let Some(f) = self.assembly.function_body_by_index(func) {
                f.labels().get(id).cloned()
            } else {
                None
            }
        } else {
            None
        }
    }

    fn alloc_data(&mut self) -> SimpleResult<Option<Value>> {
        if self.assembly.data().is_empty() {
            return Ok(None);
        }
        let mut addr = ::std::usize::MAX;
        let mut size = 0;
        for d in self.assembly.data() {
            let v = match d {
                Data::I8(v) => {
                    let r = self.state.alloc_memory_value(1)?;
                    self.state.store_data(r.address, v)?;
                    r
                }
                Data::U8(v) => {
                    let r = self.state.alloc_memory_value(1)?;
                    self.state.store_data(r.address, v)?;
                    r
                }
                Data::I16(v) => {
                    let r = self.state.alloc_memory_value(2)?;
                    self.state.store_data(r.address, v)?;
                    r
                }
                Data::U16(v) => {
                    let r = self.state.alloc_memory_value(2)?;
                    self.state.store_data(r.address, v)?;
                    r
                }
                Data::I32(v) => {
                    let r = self.state.alloc_memory_value(4)?;
                    self.state.store_data(r.address, v)?;
                    r
                }
                Data::U32(v) => {
                    let r = self.state.alloc_memory_value(4)?;
                    self.state.store_data(r.address, v)?;
                    r
                }
                Data::I64(v) => {
                    let r = self.state.alloc_memory_value(8)?;
                    self.state.store_data(r.address, v)?;
                    r
                }
                Data::U64(v) => {
                    let r = self.state.alloc_memory_value(8)?;
                    self.state.store_data(r.address, v)?;
                    r
                }
                Data::F32(v) => {
                    let r = self.state.alloc_memory_value(4)?;
                    self.state.store_data(r.address, v)?;
                    r
                }
                Data::F64(v) => {
                    let r = self.state.alloc_memory_value(8)?;
                    self.state.store_data(r.address, v)?;
                    r
                }
                Data::Isize(v) => {
                    let r = self.state.alloc_memory_value(size_of::<isize>())?;
                    self.state.store_data(r.address, v)?;
                    r
                }
                Data::Usize(v) => {
                    let r = self.state.alloc_memory_value(size_of::<usize>())?;
                    self.state.store_data(r.address, v)?;
                    r
                }
                Data::String(s) => {
                    if let Ok(ref s) = CString::new(s.as_str()) {
                        let bytes = s.as_bytes_with_nul();
                        let sv = self.state.alloc_memory_value(bytes.len())?;
                        self.state.store_bytes(sv.address, bytes)?;
                        let v = self.state.alloc_memory_value(size_of::<usize>())?;
                        self.state.store_data(v.address, &sv.address)?;
                        Value::new(sv.address, sv.size + v.size)
                    } else {
                        return Err(SimpleError::new(format!(
                            "Could not store string that is not C-compatible: '{}'",
                            s
                        )));
                    }
                }
                Data::None => unreachable!(),
            };
            addr = addr.min(v.address);
            size += v.size;
        }
        Ok(Some(Value::new(addr, size)))
    }

    fn call_function(&mut self, function: usize) -> SimpleResult<()> {
        if self.stage == ExecutionStage::Running {
            if let Some(f) = self.assembly.function_by_index(function) {
                let params_stackpos = self
                    .state
                    .stack_pos()
                    .checked_sub(f.params().iter().map(|l| l.size()).sum())
                    .unwrap();
                let r = if let Some(t) = f.typeid() {
                    Some(self.state.alloc_stack_value(self.assembly.type_size(t))?)
                } else {
                    None
                };
                let size = f.locals().iter().map(|l| l.size()).sum();
                let l = if size != 0 {
                    Some(self.state.alloc_stack_value(size)?)
                } else {
                    None
                };
                self.callstack
                    .push(CallStackFrame::new(f.index(), 0, params_stackpos, r, l));
                Ok(())
            } else {
                Err(SimpleError::new(format!(
                    "Trying to call non-existing function with index: {}",
                    function
                )))
            }
        } else {
            Err(SimpleError::new(
                "Trying to call function on uninitialized or complete VM".to_owned(),
            ))
        }
    }

    fn return_function(&mut self) -> SimpleResult<()> {
        if self.stage == ExecutionStage::Running {
            if let Some(frame) = self.callstack.pop() {
                if let Some(v) = frame.result() {
                    let bytes = self.state.load_bytes(v.address, v.size)?;
                    self.state.stack_reset(frame.params_stackpos())?;
                    let v = self.state.stack_push_bytes(&bytes)?;
                    self.pointers.push(v.address);
                } else {
                    self.state.stack_reset(frame.params_stackpos())?;
                    self.pointers.push(0usize);
                }
                if !self.can_resume() {
                    self.stage = ExecutionStage::Complete;
                }
                Ok(())
            } else {
                Err(SimpleError::new(
                    "Trying to return from no running function".to_owned(),
                ))
            }
        } else {
            Err(SimpleError::new(
                "Trying to call function on uninitialized or complete VM".to_owned(),
            ))
        }
    }

    fn location(&self) -> Option<(usize, usize, usize)> {
        if let Some(f) = self.callstack.last() {
            Some((self.callstack.len() - 1, f.function(), f.address()))
        } else {
            None
        }
    }

    fn resume_op<P>(&mut self) -> SimpleResult<bool>
    where
        P: Processor,
    {
        if let Some((i, func, addr)) = self.location() {
            let b = self.assembly.function_body_by_index(func).unwrap();
            let bodysize = b.code().len();
            if addr >= bodysize {
                self.return_function()?;
                return Ok(false);
            }
            let mut stream = Cursor::new(b.code());
            stream.seek(SeekFrom::Start(addr as u64))?;
            loop {
                let op = OpIndex::from(stream.read_u8()?);
                match op {
                    OpIndex::NoOp => unreachable!(),
                    OpIndex::DataPointer => {
                        let offset = stream.read_u64::<BigEndian>()? as usize;
                        let address = self.data.unwrap().address + offset;
                        self.callstack[i].address = stream.position() as usize;
                        self.pointers.push(address);
                    }
                    OpIndex::ParamsPointer => {
                        let offset = stream.read_u64::<BigEndian>()? as usize;
                        let address = self.callstack[i].params_stackpos + offset;
                        self.callstack[i].address = stream.position() as usize;
                        self.pointers.push(address);
                    }
                    OpIndex::ResultPointer => {
                        let address = self.callstack[i].result.unwrap().address;
                        self.callstack[i].address = stream.position() as usize;
                        self.pointers.push(address);
                    }
                    OpIndex::LocalsPointer => {
                        let offset = stream.read_u64::<BigEndian>()? as usize;
                        let address = self.callstack[i].locals.unwrap().address + offset;
                        self.callstack[i].address = stream.position() as usize;
                        self.pointers.push(address);
                    }
                    OpIndex::GlobalsPointer => {
                        let offset = stream.read_u64::<BigEndian>()? as usize;
                        let address = self.globals.unwrap().address + offset;
                        self.callstack[i].address = stream.position() as usize;
                        self.pointers.push(address);
                    }
                    OpIndex::OffsetPointer => {
                        let offset = stream.read_u64::<BigEndian>()? as usize;
                        let address = self.pointers.pop().unwrap() + offset;
                        self.callstack[i].address = stream.position() as usize;
                        self.pointers.push(address);
                    }
                    OpIndex::ReferencePointer => {
                        let v = self.state.stack_push_data(&self.pointers.pop().unwrap())?;
                        self.callstack[i].address = stream.position() as usize;
                        self.pointers.push(v.address);
                    }
                    OpIndex::DereferencePointer => {
                        let address = self.pointers.pop().unwrap();
                        let address = self.state.load_data::<usize>(address)?;
                        self.callstack[i].address = stream.position() as usize;
                        self.pointers.push(address);
                    }
                    OpIndex::StoreTargetAddress => {
                        let address = self.pointers.pop().unwrap();
                        self.callstack[i].address = stream.position() as usize;
                        self.callstack[i].op_target_addresses.push(address);
                    }
                    OpIndex::StoreParamAddress => {
                        let address = self.pointers.pop().unwrap();
                        self.callstack[i].address = stream.position() as usize;
                        self.callstack[i].op_param_addresses.push(address);
                    }
                    OpIndex::ProduceTuple => {
                        let count = stream.read_u64::<BigEndian>()? as usize;
                        let mut addresses_sizes = vec![];
                        let mut size = 0;
                        for _ in 0..count {
                            let s = stream.read_u64::<BigEndian>()? as usize;
                            let a = self.pointers.pop().unwrap();
                            addresses_sizes.push((a, s));
                            size += s;
                        }
                        let v = self.state.alloc_stack_value(size)?;
                        let mut offset = 0;
                        for (a, s) in addresses_sizes {
                            self.state.memory_move(a, s, v.address + offset)?;
                            offset += s;
                        }
                        self.callstack[i].address = stream.position() as usize;
                        self.pointers.push(v.address);
                    }
                    OpIndex::ExecuteOpStart => {
                        self.callstack[i].op_stackpos = self.state.stack_pos();
                    }
                    OpIndex::ExecuteOpStop => {
                        let op = stream.read_u64::<BigEndian>()? as usize;
                        let op = self.assembly.ops_map().get(op).unwrap().to_owned();
                        let (params, targets) = self.callstack[i].collect_params_targets();
                        let addr = stream.position() as usize;
                        self.callstack[i].address = addr;
                        let action = { P::process_op(&op, &params, &targets, self)? };
                        self.state.stack_reset(self.callstack[i].op_stackpos)?;
                        self.callstack[i].op_stackpos = 0;
                        match action {
                            OpAction::None => {
                                if addr >= bodysize || !self.can_resume() {
                                    self.return_function()?;
                                }
                            }
                            OpAction::GoTo(a) => {
                                self.callstack[i].address = a;
                            }
                            OpAction::Return => self.return_function()?,
                        }
                        break;
                    }
                    OpIndex::ExecuteOpInlineStart => {
                        self.callstack[i].address = stream.position() as usize;
                        self.callstack.push(self.callstack[i].duplicate());
                        return Ok(true);
                    }
                    OpIndex::ExecuteOpInlineStop => {
                        let op = stream.read_u64::<BigEndian>()? as usize;
                        let size = stream.read_u64::<BigEndian>()? as usize;
                        let op = self.assembly.ops_map().get(op).unwrap().to_owned();
                        let (params, mut targets) = self.callstack[i].collect_params_targets();
                        let v = self.state.alloc_stack_value(size)?;
                        targets.push(v.address);
                        self.callstack[i - 1].address = stream.position() as usize;
                        {
                            P::process_op(&op, &params, &targets, self)?;
                        }
                        self.callstack.pop();
                        self.pointers.push(v.address);
                        break;
                    }
                    OpIndex::CallFunction => {
                        let f = stream.read_u64::<BigEndian>()? as usize;
                        if let Some(func) = self.assembly.function_by_index(f) {
                            let mut addresses_sizes = vec![];
                            let mut size = 0;
                            for p in func.params() {
                                let a = self.pointers.pop().unwrap();
                                addresses_sizes.push((a, p.size()));
                                size += p.size();
                            }
                            let v = self.state.alloc_stack_value(size)?;
                            let mut offset = 0;
                            for (a, s) in addresses_sizes {
                                self.state.memory_move(a, s, v.address + offset)?;
                                offset += s;
                            }
                            self.callstack[i].address = stream.position() as usize;
                            self.call_function(f)?;
                            return Ok(true);
                        } else {
                            unreachable!();
                        }
                    }
                }
            }
        } else {
            self.stage = ExecutionStage::Complete;
        }
        Ok(false)
    }
}
