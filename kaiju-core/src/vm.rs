#![allow(clippy::unused_io_amount)]

use crate::assembly::DataType;
use crate::error::*;
use byteorder::{BigEndian, ReadBytesExt};
use std::collections::HashMap;
use std::fmt;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::mem::size_of;

fn read_string(stream: &mut Read) -> SimpleResult<String> {
    let size = stream.read_u64::<BigEndian>()? as usize;
    let mut bytes = vec![0; size];
    stream.read(&mut bytes)?;
    match String::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(err) => Err(SimpleError::new(format!("{}", err))),
    }
}

fn read_type(stream: &mut Read) -> SimpleResult<Type> {
    let mode = stream.read_u8()?;
    match mode {
        0 => {
            let index = stream.read_u64::<BigEndian>()? as usize;
            Ok(Type::Identifier(index))
        }
        1 => Ok(Type::Pointer(Box::new(read_type(stream)?))),
        2 => {
            let count = stream.read_u64::<BigEndian>()? as usize;
            let mut types = vec![];
            for _ in 0..count {
                types.push(read_type(stream)?);
            }
            Ok(Type::Tuple(types))
        }
        _ => unreachable!(),
    }
}

fn read_variable(stream: &mut Read) -> SimpleResult<Variable> {
    let index = stream.read_u64::<BigEndian>()? as usize;
    let typeid = read_type(stream)?;
    let size = stream.read_u64::<BigEndian>()? as usize;
    let offset = {
        if stream.read_u8()? > 0 {
            Some(stream.read_u64::<BigEndian>()? as usize)
        } else {
            None
        }
    };
    Ok(Variable {
        index,
        typeid,
        size,
        offset,
    })
}

#[derive(Debug, Clone)]
pub enum Data {
    None,
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    Isize(isize),
    Usize(usize),
    String(String),
}

#[derive(Debug, Clone)]
pub enum Type {
    Identifier(usize),
    Pointer(Box<Type>),
    Tuple(Vec<Type>),
}

#[derive(Debug, Clone)]
pub struct Struct {
    index: usize,
    fields: Vec<StructField>,
    size: usize,
    export: bool,
}

impl Struct {
    #[inline]
    pub fn index(&self) -> usize {
        self.index
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
}

#[derive(Debug, Clone)]
pub struct StructField {
    typeid: Type,
    offset: usize,
    size: usize,
}

impl StructField {
    #[inline]
    pub fn typeid(&self) -> &Type {
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
pub struct Function {
    index: usize,
    params: Vec<Variable>,
    typeid: Option<Type>,
    locals: Vec<Variable>,
    external: Option<(String, String)>,
    export: bool,
}

impl Function {
    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn params(&self) -> &[Variable] {
        &self.params
    }

    #[inline]
    pub fn typeid(&self) -> &Option<Type> {
        &self.typeid
    }

    #[inline]
    pub fn locals(&self) -> &[Variable] {
        &self.locals
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

#[derive(Clone)]
pub struct FunctionBody {
    labels: HashMap<String, usize>,
    code: Vec<u8>,
}

impl fmt::Debug for FunctionBody {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FunctionBody")
            .field("labels", &self.labels)
            .field("code", &format!("[...; {}]", self.code.len()))
            .finish()
    }
}

impl FunctionBody {
    #[inline]
    pub fn labels(&self) -> &HashMap<String, usize> {
        &self.labels
    }

    #[inline]
    pub fn code(&self) -> &[u8] {
        &self.code
    }
}

#[derive(Debug, Clone)]
pub struct Variable {
    index: usize,
    typeid: Type,
    size: usize,
    offset: Option<usize>,
}

impl Variable {
    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn typeid(&self) -> &Type {
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

#[derive(Clone)]
pub struct VmAssembly {
    export_structs: HashMap<String, usize>,
    export_functions: HashMap<String, usize>,
    structs: Vec<Struct>,
    functions: Vec<Function>,
    data: Vec<Data>,
    globals_size: usize,
    ops: Vec<String>,
    bodies: Vec<FunctionBody>,
}

impl fmt::Debug for VmAssembly {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("VmAssembly")
            .field("export_structs", &self.export_structs)
            .field("export_functions", &self.export_functions)
            .field("structs", &self.structs)
            .field("functions", &self.functions)
            .field("data", &format!("[...; {}]", self.data.len()))
            .field("globals_size", &self.globals_size)
            .field("ops", &self.ops)
            .field("bodies", &self.bodies)
            .finish()
    }
}

impl VmAssembly {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(bytes: Vec<u8>) -> SimpleResult<Self> {
        let mut stream = Cursor::new(bytes);
        let mut magic = [0; 4];
        stream.read(&mut magic)?;
        match magic {
            [0x4b, 0x4a, 0x50, 1] => Self::new_v1(stream),
            _ => Err(SimpleError::new(format!(
                "Trying to run assembly with unsupported version: {}",
                magic[3]
            ))),
        }
    }

    fn new_v1(mut stream: Cursor<Vec<u8>>) -> SimpleResult<Self> {
        let export_structs = {
            let _size = stream.read_u64::<BigEndian>()?;
            let count = stream.read_u64::<BigEndian>()?;
            let mut result = HashMap::new();
            for _ in 0..count {
                let index = stream.read_u64::<BigEndian>()? as usize;
                let id = read_string(&mut stream)?;
                result.insert(id, index);
            }
            result
        };
        let export_functions = {
            let _size = stream.read_u64::<BigEndian>()?;
            let count = stream.read_u64::<BigEndian>()?;
            let mut result = HashMap::new();
            for _ in 0..count {
                let index = stream.read_u64::<BigEndian>()? as usize;
                let id = read_string(&mut stream)?;
                result.insert(id, index);
            }
            result
        };
        let structs = {
            let size = stream.read_u64::<BigEndian>()? as i64;
            let count = stream.read_u64::<BigEndian>()? as usize;
            stream.seek(SeekFrom::Current(size))?;
            let mut result = vec![];
            for _ in 0..count {
                let index = stream.read_u64::<BigEndian>()? as usize;
                let fields = {
                    let mut result = vec![];
                    let count = stream.read_u64::<BigEndian>()? as usize;
                    for _ in 0..count {
                        let typeid = read_type(&mut stream)?;
                        let offset = stream.read_u64::<BigEndian>()? as usize;
                        let size = stream.read_u64::<BigEndian>()? as usize;
                        result.push(StructField {
                            typeid,
                            offset,
                            size,
                        });
                    }
                    result
                };
                let size = stream.read_u64::<BigEndian>()? as usize;
                let export = stream.read_u8()? > 0;
                result.push(Struct {
                    index,
                    fields,
                    size,
                    export,
                });
            }
            result
        };
        let functions = {
            let size = stream.read_u64::<BigEndian>()? as i64;
            let count = stream.read_u64::<BigEndian>()? as usize;
            stream.seek(SeekFrom::Current(size))?;
            let mut result = vec![];
            for _ in 0..count {
                let index = stream.read_u64::<BigEndian>()? as usize;
                let params = {
                    let mut result = vec![];
                    let count = stream.read_u64::<BigEndian>()? as usize;
                    for _ in 0..count {
                        result.push(read_variable(&mut stream)?);
                    }
                    result
                };
                let typeid = if stream.read_u8()? > 0 {
                    Some(read_type(&mut stream)?)
                } else {
                    None
                };
                let locals = {
                    let mut result = vec![];
                    let count = stream.read_u64::<BigEndian>()? as usize;
                    for _ in 0..count {
                        result.push(read_variable(&mut stream)?);
                    }
                    result
                };
                let external = if stream.read_u8()? > 0 {
                    let m = read_string(&mut stream)?;
                    let f = read_string(&mut stream)?;
                    Some((m, f))
                } else {
                    None
                };
                let export = stream.read_u8()? > 0;
                result.push(Function {
                    index,
                    params,
                    typeid,
                    locals,
                    external,
                    export,
                });
            }
            result
        };
        let data = {
            let _size = stream.read_u64::<BigEndian>()? as usize;
            let count = stream.read_u64::<BigEndian>()? as usize;
            let mut result = vec![];
            for _ in 0..count {
                let t = DataType::from(stream.read_u8()?);
                match t {
                    DataType::Unknown => (),
                    DataType::I8 => result.push(Data::I8(stream.read_i8()?)),
                    DataType::U8 => result.push(Data::U8(stream.read_u8()?)),
                    DataType::I16 => result.push(Data::I16(stream.read_i16::<BigEndian>()?)),
                    DataType::U16 => result.push(Data::U16(stream.read_u16::<BigEndian>()?)),
                    DataType::I32 => result.push(Data::I32(stream.read_i32::<BigEndian>()?)),
                    DataType::U32 => result.push(Data::U32(stream.read_u32::<BigEndian>()?)),
                    DataType::I64 => result.push(Data::I64(stream.read_i64::<BigEndian>()?)),
                    DataType::U64 => result.push(Data::U64(stream.read_u64::<BigEndian>()?)),
                    DataType::F32 => result.push(Data::F32(stream.read_f32::<BigEndian>()?)),
                    DataType::F64 => result.push(Data::F64(stream.read_f64::<BigEndian>()?)),
                    DataType::Isize => result.push(Data::Isize(
                        stream.read_int::<BigEndian>(size_of::<isize>())? as isize,
                    )),
                    DataType::Usize => result.push(Data::Usize(
                        stream.read_uint::<BigEndian>(size_of::<usize>())? as usize,
                    )),
                    DataType::StringU8 => result.push(Data::String(read_string(&mut stream)?)),
                }
            }
            result
        };
        let globals_size = stream.read_u64::<BigEndian>()? as usize;
        let ops = {
            let _size = stream.read_u64::<BigEndian>()? as usize;
            let count = stream.read_u64::<BigEndian>()? as usize;
            let mut result = vec![];
            for _ in 0..count {
                result.push(read_string(&mut stream)?);
            }
            result
        };
        let bodies = {
            let size = stream.read_u64::<BigEndian>()? as i64;
            let count = stream.read_u64::<BigEndian>()? as usize;
            stream.seek(SeekFrom::Current(size))?;
            let mut result = vec![];
            for _ in 0..count {
                let _size = stream.read_u64::<BigEndian>()? as usize;
                let labels = {
                    let mut result = HashMap::new();
                    let _size = stream.read_u64::<BigEndian>()? as usize;
                    let count = stream.read_u64::<BigEndian>()? as usize;
                    for _ in 0..count {
                        let id = read_string(&mut stream)?;
                        let address = stream.read_u64::<BigEndian>()? as usize;
                        result.insert(id, address);
                    }
                    result
                };
                let code = {
                    let size = stream.read_u64::<BigEndian>()? as usize;
                    let _count = stream.read_u64::<BigEndian>()? as usize;
                    let mut result = vec![0; size];
                    stream.read(&mut result)?;
                    result
                };
                result.push(FunctionBody { labels, code });
            }
            result
        };
        Ok(Self {
            export_structs,
            export_functions,
            structs,
            functions,
            data,
            globals_size,
            ops,
            bodies,
        })
    }

    #[inline]
    pub fn export_structs(&self) -> &HashMap<String, usize> {
        &self.export_structs
    }

    #[inline]
    pub fn export_functions(&self) -> &HashMap<String, usize> {
        &self.export_functions
    }

    #[inline]
    pub fn structs(&self) -> &[Struct] {
        &self.structs
    }

    #[inline]
    pub fn functions(&self) -> &[Function] {
        &self.functions
    }

    #[inline]
    pub fn data(&self) -> &[Data] {
        &self.data
    }

    #[inline]
    pub fn globals_size(&self) -> usize {
        self.globals_size
    }

    #[inline]
    pub fn ops_map(&self) -> &[String] {
        &self.ops
    }

    #[inline]
    pub fn functions_code(&self) -> &[FunctionBody] {
        &self.bodies
    }

    #[inline]
    pub fn struct_by_id(&self, id: &str) -> Option<&Struct> {
        if let Some(i) = self.export_structs.get(id) {
            Some(&self.structs[*i])
        } else {
            None
        }
    }

    #[inline]
    pub fn function_by_id(&self, id: &str) -> Option<&Function> {
        if let Some(i) = self.export_functions.get(id) {
            Some(&self.functions[*i])
        } else {
            None
        }
    }

    #[inline]
    pub fn function_body_by_id(&self, id: &str) -> Option<&FunctionBody> {
        if let Some(i) = self.export_functions.get(id) {
            Some(&self.bodies[*i])
        } else {
            None
        }
    }

    #[inline]
    pub fn struct_by_index(&self, index: usize) -> Option<&Struct> {
        if let Some(s) = self.structs.get(index) {
            Some(s)
        } else {
            None
        }
    }

    #[inline]
    pub fn function_by_index(&self, index: usize) -> Option<&Function> {
        if let Some(f) = self.functions.get(index) {
            Some(f)
        } else {
            None
        }
    }

    #[inline]
    pub fn function_body_by_index(&self, index: usize) -> Option<&FunctionBody> {
        if let Some(f) = self.bodies.get(index) {
            Some(f)
        } else {
            None
        }
    }

    #[inline]
    pub fn type_size(&self, typeid: &Type) -> usize {
        match typeid {
            Type::Identifier(i) => self.structs[*i].size(),
            Type::Pointer(_) => size_of::<usize>(),
            Type::Tuple(t) => t.iter().map(|t| self.type_size(t)).sum(),
        }
    }
}
