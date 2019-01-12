#![allow(clippy::unused_io_amount)]
#![allow(clippy::map_entry)]

use crate::error::*;
use crate::program::{
    Access as CoreAccess, BlockOp as CoreBlockOp, Extern as CoreExtern, Function as CoreFunction,
    Number as CoreNumber, OpsDescriptor, Program as CoreProgram, Struct as CoreStruct,
    Type as CoreType, Value as CoreValue, Variable as CoreVariable,
};
use crate::utils::*;
use byteorder::{BigEndian, WriteBytesExt};
use std::collections::HashMap;
use std::ffi::CString;
use std::io::{Cursor, Write};
use std::mem::size_of;

pub type OpsMap = HashMap<String, (u64, Option<CoreType>)>;

#[derive(Debug, Clone, Copy)]
pub enum OpIndex {
    NoOp = 0,
    DataPointer = 1,
    ParamsPointer = 2,
    ResultPointer = 3,
    LocalsPointer = 4,
    GlobalsPointer = 5,
    OffsetPointer = 6,
    ReferencePointer = 7,
    DereferencePointer = 8,
    StoreTargetAddress = 9,
    StoreParamAddress = 10,
    ExecuteOpStart = 11,
    ExecuteOpStop = 12,
    ExecuteOpInlineStart = 13,
    ExecuteOpInlineStop = 14,
    ProduceTuple = 15,
    CallFunction = 16,
}

impl From<u8> for OpIndex {
    fn from(v: u8) -> Self {
        match v {
            0 => OpIndex::NoOp,
            1 => OpIndex::DataPointer,
            2 => OpIndex::ParamsPointer,
            3 => OpIndex::ResultPointer,
            4 => OpIndex::LocalsPointer,
            5 => OpIndex::GlobalsPointer,
            6 => OpIndex::OffsetPointer,
            7 => OpIndex::ReferencePointer,
            8 => OpIndex::DereferencePointer,
            9 => OpIndex::StoreTargetAddress,
            10 => OpIndex::StoreParamAddress,
            11 => OpIndex::ExecuteOpStart,
            12 => OpIndex::ExecuteOpStop,
            13 => OpIndex::ExecuteOpInlineStart,
            14 => OpIndex::ExecuteOpInlineStop,
            15 => OpIndex::ProduceTuple,
            16 => OpIndex::CallFunction,
            _ => panic!("Unsupported op index: {}", v),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DataType {
    Unknown = 0,
    I8 = 1,
    U8 = 2,
    I16 = 3,
    U16 = 4,
    I32 = 5,
    U32 = 6,
    I64 = 7,
    U64 = 8,
    F32 = 9,
    F64 = 10,
    Isize = 11,
    Usize = 12,
    StringU8 = 13,
}

impl From<u8> for DataType {
    fn from(v: u8) -> Self {
        match v {
            0 => DataType::Unknown,
            1 => DataType::I8,
            2 => DataType::U8,
            3 => DataType::I16,
            4 => DataType::U16,
            5 => DataType::I32,
            6 => DataType::U32,
            7 => DataType::I64,
            8 => DataType::U64,
            9 => DataType::F32,
            10 => DataType::F64,
            11 => DataType::Isize,
            12 => DataType::Usize,
            13 => DataType::StringU8,
            _ => panic!("Unsupported data type: {}", v),
        }
    }
}

pub fn encode_assembly(program: &CoreProgram, ops: &OpsDescriptor) -> SimpleResult<Vec<u8>> {
    let assembly = Assembly::from_core(program)?;
    assembly.to_bytes(ops)
}

fn write_core_type(typeid: &CoreType, stream: &mut Write, assembly: &Assembly) -> SimpleResult<()> {
    match typeid {
        CoreType::Identifier(id) => {
            stream.write_u8(0)?;
            let index = assembly
                .structs()
                .iter()
                .position(|s| s.id() == id)
                .unwrap();
            stream.write_u64::<BigEndian>(index as u64)?;
        }
        CoreType::Pointer(typeid) => {
            stream.write_u8(1)?;
            write_core_type(&typeid, stream, assembly)?;
        }
        CoreType::Tuple(types) => {
            stream.write_u8(2)?;
            stream.write_u64::<BigEndian>(types.len() as u64)?;
            for t in types {
                write_core_type(t, stream, assembly)?;
            }
        }
    }
    Ok(())
}

fn write_string(value: &str, stream: &mut Write) -> SimpleResult<()> {
    stream.write_u64::<BigEndian>(value.as_bytes().len() as u64)?;
    stream.write(value.as_bytes())?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Assembly {
    magic: [u8; 4],
    structs: Vec<Struct>,
    globals: Vec<Variable>,
    functions: Vec<Function>,
    modules: Vec<Module>,
    export_structs: Vec<usize>,
    export_functions: Vec<usize>,
}

impl Assembly {
    pub fn from_core(program: &CoreProgram) -> SimpleResult<Self> {
        let mut structs = vec![
            Struct::new_atom(0, "i8", 1),
            Struct::new_atom(1, "u8", 1),
            Struct::new_atom(2, "i16", 2),
            Struct::new_atom(3, "u16", 2),
            Struct::new_atom(4, "i32", 4),
            Struct::new_atom(5, "u32", 4),
            Struct::new_atom(6, "i64", 8),
            Struct::new_atom(7, "u64", 8),
            Struct::new_atom(8, "f32", 4),
            Struct::new_atom(9, "f64", 8),
            Struct::new_atom(10, "isize", size_of::<isize>()),
            Struct::new_atom(11, "usize", size_of::<usize>()),
        ];
        let mut globals = vec![];
        let mut functions = vec![];
        let mut modules = vec![];
        for module in &program.modules {
            let index = modules.len();
            let structs = module
                .structs
                .iter()
                .map(|struct_| {
                    let index = structs.len();
                    let s = Struct::from_core(index, struct_, program)?;
                    structs.push(s);
                    Ok(index)
                })
                .collect::<SimpleResult<_>>()?;
            let globals = module
                .globals
                .iter()
                .map(|global| {
                    let index = globals.len();
                    let g = Variable::from_core(index, global, program, None)?;
                    globals.push(g);
                    Ok(index)
                })
                .collect::<SimpleResult<_>>()?;
            let functions = module
                .functions
                .iter()
                .map(|function| {
                    let index = functions.len();
                    let f = Function::from_core(index, function, program)?;
                    functions.push(f);
                    Ok(index)
                })
                .collect::<SimpleResult<_>>()?;
            modules.push(Module {
                index,
                structs,
                globals,
                functions,
            });
        }
        for (index, module) in program.modules.iter().enumerate() {
            let data: Vec<(Vec<usize>, Vec<usize>)> = module
                .imports
                .iter()
                .map(|import| {
                    let mi = program
                        .modules
                        .iter()
                        .position(|m| m.path == import.module)
                        .unwrap();
                    let pm = &program.modules[mi];
                    let m = &modules[mi];
                    let ss = import
                        .names
                        .iter()
                        .filter_map(|name| {
                            if let Some(i) = pm.structs.iter().position(|s| &s.id == name) {
                                Some(m.structs[i])
                            } else {
                                None
                            }
                        })
                        .collect();
                    let ff = import
                        .names
                        .iter()
                        .filter_map(|name| {
                            if let Some(i) = pm
                                .functions
                                .iter()
                                .position(|f| &f.header.id.clone() == name)
                            {
                                Some(m.functions[i])
                            } else {
                                None
                            }
                        })
                        .collect();
                    (ss, ff)
                })
                .collect();
            for (ss, ff) in data {
                modules[index].structs.extend(ss);
                modules[index].functions.extend(ff);
            }
        }
        let mut extern_functions = HashMap::<String, Function>::new();
        for module in &program.modules {
            for extern_ in &module.externs {
                let id = extern_.item.to_string();
                if !extern_functions.contains_key(&id) {
                    let index = functions.len() + extern_functions.len();
                    let f = Function::from_core_extern(index, extern_, program)?;
                    extern_functions.insert(id, f);
                }
            }
        }
        for (i, pm) in program.modules.iter().enumerate() {
            let m = &mut modules[i];
            for extern_ in &pm.externs {
                let id = extern_.item.id.clone();
                let index = extern_functions
                    .iter()
                    .find(|(i, _)| i == &&id)
                    .unwrap()
                    .1
                    .index;
                m.functions.push(index);
            }
        }
        for (_, f) in extern_functions {
            functions.push(f);
        }
        let export_structs = structs
            .iter()
            .filter_map(|s| if s.export { Some(s.index) } else { None })
            .collect();
        let export_functions = functions
            .iter()
            .filter_map(|f| if f.export { Some(f.index) } else { None })
            .collect();
        Ok(Self {
            magic: program.magic,
            structs,
            globals,
            functions,
            modules,
            export_structs,
            export_functions,
        })
    }

    pub fn to_bytes(&self, ops: &OpsDescriptor) -> SimpleResult<Vec<u8>> {
        let mut stream = Cursor::new(vec![]);
        let export_structs = {
            let mut stream = Cursor::new(vec![]);
            for i in &self.export_structs {
                let i = &self.structs[*i];
                stream.write_u64::<BigEndian>(i.index() as u64)?;
                write_string(i.id(), &mut stream)?;
            }
            stream.into_inner()
        };
        let export_functions = {
            let mut stream = Cursor::new(vec![]);
            for i in &self.export_functions {
                let i = &self.functions[*i];
                stream.write_u64::<BigEndian>(i.index() as u64)?;
                write_string(i.id(), &mut stream)?;
            }
            stream.into_inner()
        };
        let structs = self
            .structs
            .iter()
            .map(|i| Ok((i, i.to_bytes(self)?)))
            .collect::<SimpleResult<Vec<(&Struct, Vec<u8>)>>>()?;
        let structs_offsets = {
            let mut stream = Cursor::new(vec![]);
            let mut offset = 0;
            for (i, b) in &structs {
                stream.write_u64::<BigEndian>(i.index() as u64)?;
                stream.write_u64::<BigEndian>(offset)?;
                offset += b.len() as u64;
            }
            stream.into_inner()
        };
        let functions = self
            .functions
            .iter()
            .map(|i| Ok((i, i.to_header_bytes(self)?)))
            .collect::<SimpleResult<Vec<(&Function, Vec<u8>)>>>()?;
        let functions_offsets = {
            let mut stream = Cursor::new(vec![]);
            let mut offset = 0;
            for (i, b) in &functions {
                stream.write_u64::<BigEndian>(i.index() as u64)?;
                stream.write_u64::<BigEndian>(offset)?;
                offset += b.len() as u64;
            }
            stream.into_inner()
        };
        let (data_offsets, data) = self.collect_data()?;
        let (globals, globals_size) = {
            let mut result = HashMap::new();
            let mut offset = 0;
            for i in &self.globals {
                result.insert(i.id().to_owned(), offset);
                offset += i.size() as u64;
            }
            (result, offset)
        };
        let (ops_map, ops) = self.collect_ops(ops)?;
        let bodies = self
            .functions
            .iter()
            .map(|i| Ok((i, i.to_body_bytes(&ops_map, &data_offsets, &globals, self)?)))
            .collect::<SimpleResult<Vec<(&Function, Vec<u8>)>>>()?;
        let bodies_offsets = {
            let mut stream = Cursor::new(vec![]);
            let mut offset = 0;
            for (i, b) in &bodies {
                stream.write_u64::<BigEndian>(i.index() as u64)?;
                stream.write_u64::<BigEndian>(offset)?;
                offset += b.len() as u64;
            }
            stream.into_inner()
        };

        stream.write(&self.magic)?;

        stream.write_u64::<BigEndian>(export_structs.len() as u64)?;
        stream.write_u64::<BigEndian>(self.export_structs.len() as u64)?;
        stream.write(&export_structs)?;

        stream.write_u64::<BigEndian>(export_functions.len() as u64)?;
        stream.write_u64::<BigEndian>(self.export_functions.len() as u64)?;
        stream.write(&export_functions)?;

        stream.write_u64::<BigEndian>(structs_offsets.len() as u64)?;
        stream.write_u64::<BigEndian>(structs.len() as u64)?;
        stream.write(&structs_offsets)?;
        for (_, b) in &structs {
            stream.write(&b)?;
        }

        stream.write_u64::<BigEndian>(functions_offsets.len() as u64)?;
        stream.write_u64::<BigEndian>(functions.len() as u64)?;
        stream.write(&functions_offsets)?;
        for (_, b) in &functions {
            stream.write(&b)?;
        }

        stream.write_u64::<BigEndian>(data.len() as u64)?;
        stream.write_u64::<BigEndian>(data_offsets.len() as u64)?;
        stream.write(&data)?;

        stream.write_u64::<BigEndian>(globals_size)?;

        stream.write_u64::<BigEndian>(ops.len() as u64)?;
        stream.write_u64::<BigEndian>(ops_map.len() as u64)?;
        stream.write(&ops)?;

        stream.write_u64::<BigEndian>(bodies_offsets.len() as u64)?;
        stream.write_u64::<BigEndian>(bodies.len() as u64)?;
        stream.write(&bodies_offsets)?;
        for (_, b) in &bodies {
            stream.write_u64::<BigEndian>(b.len() as u64)?;
            stream.write(&b)?;
        }

        Ok(stream.into_inner())
    }

    #[inline]
    pub fn magic(&self) -> &[u8; 4] {
        &self.magic
    }

    #[inline]
    pub fn structs(&self) -> &[Struct] {
        &self.structs
    }

    #[inline]
    pub fn globals(&self) -> &[Variable] {
        &self.globals
    }

    #[inline]
    pub fn functions(&self) -> &[Function] {
        &self.functions
    }

    #[inline]
    pub fn modules(&self) -> &[Module] {
        &self.modules
    }

    #[inline]
    pub fn export_structs(&self) -> &[usize] {
        &self.export_structs
    }

    #[inline]
    pub fn export_functions(&self) -> &[usize] {
        &self.export_functions
    }

    #[inline]
    pub fn find_struct(&self, id: &str) -> Option<&Struct> {
        self.structs.iter().find(|s| s.id() == id)
    }

    #[inline]
    pub fn find_function(&self, id: &str) -> Option<&Function> {
        self.functions.iter().find(|f| f.id() == id)
    }

    #[inline]
    pub fn find_module_function(&self, index: usize, id: &str) -> Option<&Function> {
        if index < self.modules.len() {
            for f in self.modules[index].functions() {
                let f = &self.functions[*f];
                if f.id() == id {
                    return Some(f);
                }
            }
        }
        None
    }

    fn collect_data(&self) -> SimpleResult<(HashMap<String, u64>, Vec<u8>)> {
        let mut stream = Cursor::new(vec![]);
        let mut offsets = HashMap::new();
        let mut offset = 0;
        for f in &self.functions {
            for o in &f.body {
                if let CoreBlockOp::Operation(o) = o {
                    for p in &o.params {
                        offset = self.collect_op_data(p, &mut stream, &mut offsets, offset)?;
                    }
                }
            }
        }
        Ok((offsets, stream.into_inner()))
    }

    fn collect_op_data(
        &self,
        value: &CoreValue,
        stream: &mut Cursor<Vec<u8>>,
        offsets: &mut HashMap<String, u64>,
        mut offset: usize,
    ) -> SimpleResult<usize> {
        match value {
            CoreValue::Ref(ref v, _) => self.collect_op_data(v, stream, offsets, offset),
            CoreValue::Deref(ref v, _) => self.collect_op_data(v, stream, offsets, offset),
            CoreValue::FunctionCall(_, ref v, _) => {
                for v in v {
                    offset = self.collect_op_data(v, stream, offsets, offset)?;
                }
                Ok(offset)
            }
            CoreValue::Tuple(ref v, _) => {
                for v in v {
                    offset = self.collect_op_data(v, stream, offsets, offset)?;
                }
                Ok(offset)
            }
            CoreValue::String(ref s, ref t) => {
                let id = format!("___CONST_STRING_{}", hash(s));
                if !offsets.contains_key(&id) {
                    if let Ok(ref cs) = CString::new(s.as_str()) {
                        match t.to_string().as_str() {
                            "*u8" => {
                                stream.write_u8(DataType::StringU8 as u8)?;
                                write_string(s, stream)?;
                                offset += cs.as_bytes_with_nul().len();
                                offsets.insert(id, offset as u64);
                                Ok(offset + size_of::<usize>())
                            }
                            _ => Err(SimpleError::new(format!(
                                "Trying to store constant as non-string bytes type `{}`",
                                t.to_string()
                            ))),
                        }
                    } else {
                        Err(SimpleError::new(format!(
                            "Could not store string that is not C-compatible: '{}'",
                            s
                        )))
                    }
                } else {
                    Ok(offset)
                }
            }
            CoreValue::Number(ref n) => match n {
                CoreNumber::Integer(i, ref t) => {
                    let id = format!("___CONST_INTEGER_{}", i);
                    if !offsets.contains_key(&id) {
                        match t.to_string().as_str() {
                            "i8" => {
                                stream.write_u8(DataType::I8 as u8)?;
                                stream.write_i8(*i as i8)?;
                                offsets.insert(id, offset as u64);
                                Ok(offset + 1)
                            }
                            "u8" => {
                                stream.write_u8(DataType::U8 as u8)?;
                                stream.write_u8(*i as u8)?;
                                offsets.insert(id, offset as u64);
                                Ok(offset + 1)
                            }
                            "i16" => {
                                stream.write_u8(DataType::I16 as u8)?;
                                stream.write_i16::<BigEndian>(*i as i16)?;
                                offsets.insert(id, offset as u64);
                                Ok(offset + 2)
                            }
                            "u16" => {
                                stream.write_u8(DataType::U16 as u8)?;
                                stream.write_u16::<BigEndian>(*i as u16)?;
                                offsets.insert(id, offset as u64);
                                Ok(offset + 2)
                            }
                            "i32" => {
                                stream.write_u8(DataType::I32 as u8)?;
                                stream.write_i32::<BigEndian>(*i as i32)?;
                                offsets.insert(id, offset as u64);
                                Ok(offset + 4)
                            }
                            "u32" => {
                                stream.write_u8(DataType::U32 as u8)?;
                                stream.write_u32::<BigEndian>(*i as u32)?;
                                offsets.insert(id, offset as u64);
                                Ok(offset + 4)
                            }
                            "i64" => {
                                stream.write_u8(DataType::I64 as u8)?;
                                stream.write_i64::<BigEndian>(*i as i64)?;
                                offsets.insert(id, offset as u64);
                                Ok(offset + 8)
                            }
                            "u64" => {
                                stream.write_u8(DataType::U64 as u8)?;
                                stream.write_u64::<BigEndian>(*i as u64)?;
                                offsets.insert(id, offset as u64);
                                Ok(offset + 8)
                            }
                            "isize" => {
                                stream.write_u8(DataType::Isize as u8)?;
                                stream.write_int::<BigEndian>(*i as i64, size_of::<isize>())?;
                                offsets.insert(id, offset as u64);
                                Ok(offset + size_of::<isize>())
                            }
                            "usize" => {
                                stream.write_u8(DataType::Usize as u8)?;
                                stream.write_uint::<BigEndian>(*i as u64, size_of::<usize>())?;
                                offsets.insert(id, offset as u64);
                                Ok(offset + size_of::<usize>())
                            }
                            _ => Err(SimpleError::new(format!(
                                "Trying to store constant as non-integer type `{}`",
                                t.to_string()
                            ))),
                        }
                    } else {
                        Ok(offset)
                    }
                }
                CoreNumber::Float(f, ref t) => {
                    let id = format!("___CONST_FLOAT_{}", f);
                    if !offsets.contains_key(&id) {
                        match t.to_string().as_str() {
                            "f32" => {
                                stream.write_u8(DataType::F32 as u8)?;
                                stream.write_f32::<BigEndian>(*f as f32)?;
                                offsets.insert(id, offset as u64);
                                Ok(offset + 4)
                            }
                            "f64" => {
                                stream.write_u8(DataType::F64 as u8)?;
                                stream.write_f64::<BigEndian>(*f as f64)?;
                                offsets.insert(id, offset as u64);
                                Ok(offset + 8)
                            }
                            _ => Err(SimpleError::new(format!(
                                "Trying to store constant as non-float type `{}`",
                                t.to_string()
                            ))),
                        }
                    } else {
                        Ok(offset)
                    }
                }
            },
            CoreValue::OperationInline(_, ref v, _) => {
                for v in v {
                    offset = self.collect_op_data(v, stream, offsets, offset)?;
                }
                Ok(offset)
            }
            CoreValue::Variable(_, _) => Ok(offset),
        }
    }

    fn collect_ops(&self, opsdesc: &OpsDescriptor) -> SimpleResult<(OpsMap, Vec<u8>)> {
        let mut stream = Cursor::new(vec![]);
        let mut index = 0;
        let mut ops = HashMap::new();
        for f in &self.functions {
            for o in &f.body {
                if let CoreBlockOp::Operation(o) = o {
                    if !ops.contains_key(&o.id) {
                        write_string(&o.id, &mut stream)?;
                        ops.insert(o.id.to_owned(), (index, self.find_op_type(&o.id, opsdesc)));
                        index += 1;
                    }
                    for v in &o.params {
                        self.collect_value_ops(v, opsdesc, &mut stream, &mut ops, &mut index)?;
                    }
                    for v in &o.targets {
                        self.collect_value_ops(v, opsdesc, &mut stream, &mut ops, &mut index)?;
                    }
                }
            }
        }
        Ok((ops, stream.into_inner()))
    }

    fn collect_value_ops(
        &self,
        value: &CoreValue,
        opsdesc: &OpsDescriptor,
        stream: &mut Write,
        ops: &mut OpsMap,
        index: &mut u64,
    ) -> SimpleResult<()> {
        match value {
            CoreValue::Ref(v, _) => self.collect_value_ops(v, opsdesc, stream, ops, index),
            CoreValue::Deref(v, _) => self.collect_value_ops(v, opsdesc, stream, ops, index),
            CoreValue::FunctionCall(_, v, _) => {
                for v in v {
                    self.collect_value_ops(v, opsdesc, stream, ops, index)?;
                }
                Ok(())
            }
            CoreValue::Tuple(v, _) => {
                for v in v {
                    self.collect_value_ops(v, opsdesc, stream, ops, index)?;
                }
                Ok(())
            }
            CoreValue::OperationInline(id, v, _) => {
                if !ops.contains_key(id) {
                    write_string(id, stream)?;
                    ops.insert(id.to_owned(), (*index, self.find_op_type(id, opsdesc)));
                    *index += 1;
                }
                for v in v {
                    self.collect_value_ops(v, opsdesc, stream, ops, index)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn find_op_type(&self, id: &str, opsdesc: &OpsDescriptor) -> Option<CoreType> {
        opsdesc
            .rules
            .iter()
            .find(|r| r.id == id && r.targets.len() == 1)
            .map(|r| r.targets[0].clone())
    }

    fn write_core_value(
        &self,
        value: &CoreValue,
        stream: &mut Cursor<Vec<u8>>,
        function: &Function,
        data: &HashMap<String, u64>,
        globals: &HashMap<String, u64>,
        ops: &HashMap<String, (u64, Option<CoreType>)>,
    ) -> SimpleResult<()> {
        match value {
            CoreValue::Ref(ref v, ref a) => {
                self.write_core_value(v, stream, function, data, globals, ops)?;
                stream.write_u8(OpIndex::ReferencePointer as u8)?;
                if a.is_some() {
                    Err(SimpleError::new(
                        "References cannot be accessed. They can be passed or dereferenced"
                            .to_owned(),
                    ))
                } else {
                    Ok(())
                }
            }
            CoreValue::Deref(ref v, ref a) => {
                self.write_core_value(v, stream, function, data, globals, ops)?;
                stream.write_u8(OpIndex::DereferencePointer as u8)?;
                if let Some(ref a) = a {
                    stream.write_u8(OpIndex::OffsetPointer as u8)?;
                    let t = self.find_value_type(v, function, data, ops)?;
                    self.write_core_value_access(stream, &t, a)?;
                }
                Ok(())
            }
            CoreValue::FunctionCall(ref id, ref v, ref a) => {
                let f = self.find_function(id).unwrap();
                for v in v.iter().rev() {
                    self.write_core_value(v, stream, function, data, globals, ops)?;
                }
                stream.write_u8(OpIndex::CallFunction as u8)?;
                stream.write_u64::<BigEndian>(f.index() as u64)?;
                if let Some(ref t) = f.typeid() {
                    if let Some(ref a) = a {
                        stream.write_u8(OpIndex::OffsetPointer as u8)?;
                        self.write_core_value_access(stream, t, a)?;
                    }
                    Ok(())
                } else if a.is_some() {
                    Err(SimpleError::new(format!(
                        "Trying to access return value of non-returning function: {}",
                        id
                    )))
                } else {
                    Ok(())
                }
            }
            CoreValue::Tuple(ref v, ref a) => {
                for v in v.iter().rev() {
                    self.write_core_value(v, stream, function, data, globals, ops)?;
                }
                stream.write_u8(OpIndex::ProduceTuple as u8)?;
                stream.write_u64::<BigEndian>(v.len() as u64)?;
                for v in v {
                    let t = self.find_value_type(v, function, data, ops)?;
                    stream.write_u64::<BigEndian>(self.type_size(&t) as u64)?;
                }
                if let Some(ref a) = a {
                    stream.write_u8(OpIndex::OffsetPointer as u8)?;
                    let mut types = vec![];
                    for v in v {
                        types.push(self.find_value_type(v, function, data, ops)?);
                    }
                    self.write_core_value_access_tuple(stream, &types, a)?;
                }
                Ok(())
            }
            CoreValue::String(ref s, _) => {
                stream.write_u8(OpIndex::DataPointer as u8)?;
                stream.write_u64::<BigEndian>(data[&format!("___CONST_STRING_{}", hash(s))])?;
                Ok(())
            }
            CoreValue::Number(ref n) => {
                stream.write_u8(OpIndex::DataPointer as u8)?;
                match n {
                    CoreNumber::Integer(ref v, _) => {
                        stream.write_u64::<BigEndian>(data[&format!("___CONST_INTEGER_{}", v)])?;
                        Ok(())
                    }
                    CoreNumber::Float(ref v, _) => {
                        stream.write_u64::<BigEndian>(data[&format!("___CONST_FLOAT_{}", v)])?;
                        Ok(())
                    }
                }
            }
            CoreValue::OperationInline(ref id, ref v, ref a) => {
                stream.write_u8(OpIndex::ExecuteOpInlineStart as u8)?;
                for v in v {
                    self.write_core_value(v, stream, function, data, globals, ops)?;
                    stream.write_u8(OpIndex::StoreParamAddress as u8)?;
                }
                stream.write_u8(OpIndex::ExecuteOpInlineStop as u8)?;
                stream.write_u64::<BigEndian>(ops[id].0)?;
                let t = ops[id].1.clone().unwrap();
                stream.write_u64::<BigEndian>(self.type_size(&t) as u64)?;
                if let Some(ref a) = a {
                    stream.write_u8(OpIndex::OffsetPointer as u8)?;
                    self.write_core_value_access(stream, &t, a)?;
                }
                Ok(())
            }
            CoreValue::Variable(ref id, ref a) => {
                let t = if let Some(v) = function.params().iter().find(|v| v.id() == id) {
                    stream.write_u8(OpIndex::ParamsPointer as u8)?;
                    stream.write_u64::<BigEndian>(v.offset().unwrap() as u64)?;
                    v.typeid()
                } else if let Some(v) = function.locals().iter().find(|v| v.id() == id) {
                    stream.write_u8(OpIndex::LocalsPointer as u8)?;
                    stream.write_u64::<BigEndian>(v.offset().unwrap() as u64)?;
                    v.typeid()
                } else if let Some(o) = globals.get(id) {
                    stream.write_u8(OpIndex::GlobalsPointer as u8)?;
                    stream.write_u64::<BigEndian>(*o)?;
                    self.globals.iter().find(|v| v.id() == id).unwrap().typeid()
                } else if id == "_" {
                    if let Some(t) = function.typeid() {
                        stream.write_u8(OpIndex::ResultPointer as u8)?;
                        t
                    } else if a.is_some() {
                        return Err(SimpleError::new(format!(
                            "Trying to access return value of non-returning function: {}",
                            function.index(),
                        )));
                    } else {
                        unreachable!()
                    }
                } else {
                    unreachable!()
                };
                if let Some(ref a) = a {
                    stream.write_u8(OpIndex::OffsetPointer as u8)?;
                    self.write_core_value_access(stream, t, a)?;
                }
                Ok(())
            }
        }
    }

    fn write_core_value_access(
        &self,
        stream: &mut Cursor<Vec<u8>>,
        type_: &CoreType,
        access: &CoreAccess,
    ) -> SimpleResult<()> {
        match type_ {
            CoreType::Identifier(ref id) => {
                let s = self.find_struct(id).unwrap();
                self.write_core_value_access_struct(stream, s, access)
            }
            CoreType::Tuple(ref v) => self.write_core_value_access_tuple(stream, v, access),
            CoreType::Pointer(ref v) => self.write_core_value_access(stream, v, access),
        }
    }

    fn write_core_value_access_struct(
        &self,
        stream: &mut Cursor<Vec<u8>>,
        struct_: &Struct,
        access: &CoreAccess,
    ) -> SimpleResult<()> {
        match access {
            CoreAccess::Variable(ref i, ref a) => {
                let field = struct_.find_field(i).unwrap();
                stream.write_u64::<BigEndian>(field.offset() as u64)?;
                if let Some(ref a) = a {
                    stream.write_u8(OpIndex::OffsetPointer as u8)?;
                    self.write_core_value_access(stream, field.typeid(), a)?;
                }
                Ok(())
            }
            _ => Err(SimpleError::new(format!(
                "Trying to get struct access of not straight struct id: {}",
                struct_.id()
            ))),
        }
    }

    fn write_core_value_access_tuple(
        &self,
        stream: &mut Cursor<Vec<u8>>,
        types: &[CoreType],
        access: &CoreAccess,
    ) -> SimpleResult<()> {
        match access {
            CoreAccess::Tuple(i, ref a) => {
                let (typeid, offset) = self.find_tuple_field(types, *i as usize)?;
                stream.write_u64::<BigEndian>(offset as u64)?;
                if let Some(ref a) = a {
                    stream.write_u8(OpIndex::OffsetPointer as u8)?;
                    self.write_core_value_access(stream, typeid, a)?;
                }
                Ok(())
            }
            _ => Err(SimpleError::new(format!(
                "Trying to get tuple access of not straight tuple id: {:?}",
                types
            ))),
        }
    }

    fn find_tuple_field<'a>(
        &self,
        types: &'a [CoreType],
        index: usize,
    ) -> SimpleResult<(&'a CoreType, usize)> {
        let mut offset = 0;
        for (i, t) in types.iter().enumerate() {
            let size = self.type_size(t);
            if i == index {
                return Ok((t, offset));
            } else {
                offset += size;
            }
        }
        Err(SimpleError::new(format!(
            "Tuple does not have field #{}",
            index
        )))
    }

    #[inline]
    pub fn type_size(&self, typeid: &CoreType) -> usize {
        match typeid {
            CoreType::Identifier(id) => self.structs.iter().find(|s| s.id() == id).unwrap().size(),
            CoreType::Pointer(_) => size_of::<usize>(),
            CoreType::Tuple(t) => t.iter().map(|t| self.type_size(t)).sum(),
        }
    }

    pub fn find_value_type(
        &self,
        value: &CoreValue,
        function: &Function,
        data: &HashMap<String, u64>,
        ops: &HashMap<String, (u64, Option<CoreType>)>,
    ) -> SimpleResult<CoreType> {
        match value {
            CoreValue::Ref(ref v, ref a) => {
                let t = CoreType::Pointer(Box::new(self.find_value_type(v, function, data, ops)?));
                if a.is_some() {
                    Err(SimpleError::new(
                        "References cannot be accessed. They can be passed or dereferenced"
                            .to_owned(),
                    ))
                } else {
                    Ok(t)
                }
            }
            CoreValue::Deref(ref v, ref a) => {
                if let CoreType::Pointer(t) = self.find_value_type(v, function, data, ops)? {
                    if let Some(ref a) = a {
                        self.find_access_type(&t, a, function, data)
                    } else {
                        Ok(*t)
                    }
                } else {
                    Err(SimpleError::new(format!(
                        "Trying to get type of dereferenced non-pointer value: {:?}",
                        v
                    )))
                }
            }
            CoreValue::FunctionCall(ref id, _, ref a) => {
                if let Some(f) = self.find_function(id) {
                    if let Some(ref t) = f.typeid() {
                        if let Some(ref a) = a {
                            self.find_access_type(t, a, function, data)
                        } else {
                            Ok(t.clone())
                        }
                    } else {
                        Err(SimpleError::new(format!("Trying to get type of function call where function does not return any value: {:?}", id)))
                    }
                } else {
                    Err(SimpleError::new(format!(
                        "Trying to get type of non-existing function call: {:?}",
                        id
                    )))
                }
            }
            CoreValue::Tuple(ref v, ref a) => {
                let mut t = vec![];
                for v in v {
                    t.push(self.find_value_type(v, function, data, ops)?);
                }
                let t = CoreType::Tuple(t);
                if let Some(ref a) = a {
                    self.find_access_type(&t, a, function, data)
                } else {
                    Ok(t)
                }
            }
            CoreValue::String(_, ref t) => Ok(t.clone()),
            CoreValue::Number(ref n) => Ok(match n {
                CoreNumber::Integer(_, ref t) => t.clone(),
                CoreNumber::Float(_, ref t) => t.clone(),
            }),
            CoreValue::OperationInline(ref id, _, ref a) => {
                let t = if let Some((_, Some(t))) = ops.get(id) {
                    Ok(t.clone())
                } else {
                    Err(SimpleError::new(format!(
                        "There is no op `{}` that can be inlined",
                        id
                    )))
                }?;
                if let Some(ref a) = a {
                    self.find_access_type(&t, a, function, data)
                } else {
                    Ok(t)
                }
            }
            CoreValue::Variable(ref id, ref a) => {
                let t = if let Some(v) = function.params().iter().find(|v| v.id() == id) {
                    v.typeid()
                } else if let Some(v) = function.locals().iter().find(|v| v.id() == id) {
                    v.typeid()
                } else if let Some(v) = self.globals.iter().find(|v| v.id() == id) {
                    v.typeid()
                } else {
                    return Err(SimpleError::new(format!(
                        "Trying to get type of non-existing symbol: {}",
                        id
                    )));
                };
                if let Some(ref a) = a {
                    self.find_access_type(t, a, function, data)
                } else {
                    Ok(t.clone())
                }
            }
        }
    }

    pub fn find_access_type(
        &self,
        type_: &CoreType,
        access: &CoreAccess,
        function: &Function,
        data: &HashMap<String, u64>,
    ) -> SimpleResult<CoreType> {
        match access {
            CoreAccess::Tuple(i, ref a) => match type_ {
                CoreType::Tuple(ref v) => {
                    let i = *i as usize;
                    if i < v.len() {
                        if let Some(ref a) = a {
                            self.find_access_type(&v[i], a, function, data)
                        } else {
                            Ok(v[i].clone())
                        }
                    } else {
                        Err(SimpleError::new(format!(
                            "Trying to get access of tuple with index out of bounds: {}",
                            i
                        )))
                    }
                }
                _ => Err(SimpleError::new(format!(
                    "Trying to get access of tuple on non-tuple type: {:?}",
                    type_
                ))),
            },
            CoreAccess::Variable(ref i, ref a) => match type_ {
                CoreType::Identifier(ref id) => {
                    if let Some(s) = self.find_struct(id) {
                        if let Some(f) = s.find_field(i) {
                            if let Some(ref a) = a {
                                self.find_access_type(f.typeid(), a, function, data)
                            } else {
                                Ok(f.typeid().clone())
                            }
                        } else {
                            Err(SimpleError::new(format!(
                                "Trying to get access of non-existing `{}` field of struct: {}",
                                i, id
                            )))
                        }
                    } else {
                        Err(SimpleError::new(format!(
                            "Trying to get `{}` field access of non-existing struct: {}",
                            i, id
                        )))
                    }
                }
                _ => Err(SimpleError::new(format!(
                    "Trying to get access of field which is not straight type name: {}",
                    i
                ))),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Struct {
    index: usize,
    id: String,
    fields: Vec<StructField>,
    size: usize,
    export: bool,
}

impl Struct {
    pub fn new_atom(index: usize, id: &str, size: usize) -> Self {
        Struct {
            index,
            id: id.to_owned(),
            fields: vec![],
            size,
            export: false,
        }
    }

    pub fn from_core(
        index: usize,
        struct_: &CoreStruct,
        program: &CoreProgram,
    ) -> SimpleResult<Self> {
        let mut fields = vec![];
        let mut offset = 0;
        for f in &struct_.fields {
            let field = StructField::from_core(f, offset, program)?;
            offset += field.size;
            fields.push(field);
        }
        Ok(Self {
            index,
            id: struct_.id.clone(),
            fields,
            size: offset,
            export: struct_.export,
        })
    }

    pub fn to_bytes(&self, assembly: &Assembly) -> SimpleResult<Vec<u8>> {
        let mut stream = Cursor::new(vec![]);
        stream.write_u64::<BigEndian>(self.index as u64)?;
        stream.write_u64::<BigEndian>(self.fields.len() as u64)?;
        for field in &self.fields {
            field.write(&mut stream, assembly)?;
        }
        stream.write_u64::<BigEndian>(self.size as u64)?;
        stream.write_u8(if self.export { 1 } else { 0 })?;
        Ok(stream.into_inner())
    }

    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[inline]
    pub fn fields(&self) -> &[StructField] {
        &self.fields
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }

    #[inline]
    pub fn export(&self) -> bool {
        self.export
    }

    #[inline]
    pub fn find_field(&self, id: &str) -> Option<&StructField> {
        self.fields.iter().find(|f| f.id() == id)
    }
}

#[derive(Debug, Clone)]
pub struct StructField {
    id: String,
    typeid: CoreType,
    offset: usize,
    size: usize,
}

impl StructField {
    pub fn from_core(
        field: &CoreVariable,
        offset: usize,
        program: &CoreProgram,
    ) -> SimpleResult<Self> {
        Ok(Self {
            id: field.id.clone(),
            typeid: field.typeid.clone(),
            offset,
            size: calculate_type_size(&field.typeid, program),
        })
    }

    pub fn write(&self, stream: &mut Write, assembly: &Assembly) -> SimpleResult<()> {
        write_core_type(&self.typeid, stream, assembly)?;
        stream.write_u64::<BigEndian>(self.offset as u64)?;
        stream.write_u64::<BigEndian>(self.size as u64)?;
        Ok(())
    }

    #[inline]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[inline]
    pub fn typeid(&self) -> &CoreType {
        &self.typeid
    }

    #[inline]
    pub fn offset(&self) -> usize {
        self.offset
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }
}

#[derive(Debug, Clone)]
pub struct Variable {
    index: usize,
    id: String,
    typeid: CoreType,
    size: usize,
    offset: Option<usize>,
}

impl Variable {
    pub fn from_core(
        index: usize,
        variable: &CoreVariable,
        program: &CoreProgram,
        offset: Option<usize>,
    ) -> SimpleResult<Self> {
        Ok(Self {
            index,
            id: variable.id.clone(),
            typeid: variable.typeid.clone(),
            size: calculate_type_size(&variable.typeid, program),
            offset,
        })
    }

    pub fn to_bytes(&self, assembly: &Assembly) -> SimpleResult<Vec<u8>> {
        let mut stream = Cursor::new(vec![]);
        stream.write_u64::<BigEndian>(self.index as u64)?;
        write_core_type(&self.typeid, &mut stream, assembly)?;
        stream.write_u64::<BigEndian>(self.size as u64)?;
        if let Some(o) = self.offset {
            stream.write_u8(1)?;
            stream.write_u64::<BigEndian>(o as u64)?;
        } else {
            stream.write_u8(0)?;
        }
        Ok(stream.into_inner())
    }

    pub fn write(&self, stream: &mut Write, assembly: &Assembly) -> SimpleResult<()> {
        stream.write(&self.to_bytes(assembly)?)?;
        Ok(())
    }

    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[inline]
    pub fn typeid(&self) -> &CoreType {
        &self.typeid
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }

    #[inline]
    pub fn offset(&self) -> &Option<usize> {
        &self.offset
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    index: usize,
    id: String,
    params: Vec<Variable>,
    typeid: Option<CoreType>,
    locals: Vec<Variable>,
    body: Vec<CoreBlockOp>,
    external: Option<(String, String)>,
    export: bool,
}

impl Function {
    pub fn from_core(
        index: usize,
        function: &CoreFunction,
        program: &CoreProgram,
    ) -> SimpleResult<Function> {
        let mut po = 0;
        let mut lo = 0;
        Ok(Self {
            index,
            id: function.header.id.clone(),
            params: function
                .header
                .params
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let v = Variable::from_core(i, p, program, Some(po))?;
                    po += v.size();
                    Ok(v)
                })
                .collect::<SimpleResult<_>>()?,
            typeid: function.header.typeid.clone(),
            locals: function
                .locals
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let v = Variable::from_core(i, p, program, Some(lo))?;
                    lo += v.size();
                    Ok(v)
                })
                .collect::<SimpleResult<_>>()?,
            body: function.body.clone(),
            external: None,
            export: function.export,
        })
    }

    pub fn from_core_extern(
        index: usize,
        extern_: &CoreExtern,
        program: &CoreProgram,
    ) -> SimpleResult<Function> {
        let mut po = 0;
        Ok(Self {
            index,
            id: extern_.item.id.clone(),
            params: extern_
                .item
                .params
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let v = Variable::from_core(i, p, program, Some(po))?;
                    po += v.size();
                    Ok(v)
                })
                .collect::<SimpleResult<_>>()?,
            typeid: extern_.item.typeid.clone(),
            locals: vec![],
            body: vec![],
            external: Some((
                extern_.location_module.clone(),
                extern_.location_function.clone(),
            )),
            export: false,
        })
    }

    pub fn to_header_bytes(&self, assembly: &Assembly) -> SimpleResult<Vec<u8>> {
        let mut stream = Cursor::new(vec![]);
        stream.write_u64::<BigEndian>(self.index as u64)?;
        stream.write_u64::<BigEndian>(self.params.len() as u64)?;
        for p in &self.params {
            p.write(&mut stream, assembly)?;
        }
        if let Some(t) = &self.typeid {
            stream.write_u8(1)?;
            write_core_type(t, &mut stream, assembly)?;
        } else {
            stream.write_u8(0)?;
        }
        stream.write_u64::<BigEndian>(self.locals.len() as u64)?;
        for l in &self.locals {
            l.write(&mut stream, assembly)?;
        }
        if let Some((m, f)) = &self.external {
            stream.write_u8(1)?;
            write_string(&m, &mut stream)?;
            write_string(&f, &mut stream)?;
        } else {
            stream.write_u8(0)?;
        }
        stream.write_u8(if self.export { 1 } else { 0 })?;
        Ok(stream.into_inner())
    }

    pub fn to_body_bytes(
        &self,
        ops: &HashMap<String, (u64, Option<CoreType>)>,
        data: &HashMap<String, u64>,
        globals: &HashMap<String, u64>,
        assembly: &Assembly,
    ) -> SimpleResult<Vec<u8>> {
        let mut stream_labels = Cursor::new(vec![]);
        let mut stream_ops = Cursor::new(vec![]);
        let mut labels_count = 0;
        let mut ops_count = 0;
        for op in &self.body {
            match op {
                CoreBlockOp::Operation(op) => {
                    stream_ops.write_u8(OpIndex::ExecuteOpStart as u8)?;
                    for v in op.targets.iter() {
                        assembly.write_core_value(v, &mut stream_ops, self, data, globals, ops)?;
                        stream_ops.write_u8(OpIndex::StoreTargetAddress as u8)?;
                    }
                    for v in op.params.iter() {
                        assembly.write_core_value(v, &mut stream_ops, self, data, globals, ops)?;
                        stream_ops.write_u8(OpIndex::StoreParamAddress as u8)?;
                    }
                    stream_ops.write_u8(OpIndex::ExecuteOpStop as u8)?;
                    stream_ops.write_u64::<BigEndian>(ops[&op.id].0)?;
                    ops_count += 1;
                }
                CoreBlockOp::Label(name) => {
                    write_string(name, &mut stream_labels)?;
                    stream_labels.write_u64::<BigEndian>(stream_ops.position())?;
                    labels_count += 1;
                }
            }
        }
        let mut stream = Cursor::new(vec![]);
        stream.write_u64::<BigEndian>(stream_labels.position())?;
        stream.write_u64::<BigEndian>(labels_count)?;
        stream.write(&stream_labels.into_inner())?;
        stream.write_u64::<BigEndian>(stream_ops.position())?;
        stream.write_u64::<BigEndian>(ops_count)?;
        stream.write(&stream_ops.into_inner())?;
        Ok(stream.into_inner())
    }

    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[inline]
    pub fn params(&self) -> &[Variable] {
        &self.params
    }

    #[inline]
    pub fn typeid(&self) -> &Option<CoreType> {
        &self.typeid
    }

    #[inline]
    pub fn locals(&self) -> &[Variable] {
        &self.locals
    }

    #[inline]
    pub fn body(&self) -> &[CoreBlockOp] {
        &self.body
    }

    #[inline]
    pub fn external(&self) -> &Option<(String, String)> {
        &self.external
    }

    #[inline]
    pub fn export(&self) -> bool {
        self.export
    }
}

#[derive(Debug, Clone)]
pub struct Module {
    index: usize,
    structs: Vec<usize>,
    globals: Vec<usize>,
    functions: Vec<usize>,
}

impl Module {
    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn structs(&self) -> &[usize] {
        &self.structs
    }

    #[inline]
    pub fn globals(&self) -> &[usize] {
        &self.globals
    }

    #[inline]
    pub fn functions(&self) -> &[usize] {
        &self.functions
    }

    #[inline]
    pub fn find_function<'a>(&self, id: &str, assembly: &'a Assembly) -> Option<&'a Function> {
        for f in &self.functions {
            let f = &assembly.functions()[*f];
            if f.id() == id {
                return Some(f);
            }
        }
        None
    }
}

pub fn calculate_type_size(typeid: &CoreType, program: &CoreProgram) -> usize {
    match typeid {
        CoreType::Identifier(ref id) => match id.as_str() {
            "i8" | "u8" => 1,
            "i16" | "u16" => 2,
            "i32" | "u32" | "f32" => 4,
            "i64" | "u64" | "f64" => 8,
            "isize" | "usize" => size_of::<usize>(),
            _ => {
                if let Some(s) = program.find_struct(id) {
                    s.fields
                        .iter()
                        .map(|f| calculate_type_size(&f.typeid, program))
                        .sum()
                } else {
                    0
                }
            }
        },
        CoreType::Pointer(_) => size_of::<usize>(),
        CoreType::Tuple(ref v) => v.iter().map(|t| calculate_type_size(t, program)).sum(),
    }
}
