use crate::ast::*;
use crate::error::*;
use crate::parser::*;
use serde_json;
use std::collections::HashMap;

const VERSION: u8 = 1;
const MAGIC_PROGRAM: [u8; 4] = [0x4b, 0x4a, 0x50, VERSION];
const MAGIC_MODULE: [u8; 4] = [0x4b, 0x4a, 0x4d, VERSION];

#[inline]
pub fn compile_module(source: &str) -> CompilationResult<Module> {
    Module::from_ast(&parse_module(source)?)
}

#[inline]
pub fn compile_ops_descriptor(source: &str) -> CompilationResult<OpsDescriptor> {
    OpsDescriptor::from_ast(&parse_ops_descriptor(source)?)
}

fn convert_ast_meta(ast: &[AstMeta]) -> CompilationResult<Vec<Meta>> {
    let mut result = vec![];
    for m in ast {
        for field in &m.0 {
            result.push(Meta::from_ast(&field)?);
        }
    }
    Ok(result)
}

fn convert_ast_op_definition(ast: &[AstOpRuleDef]) -> CompilationResult<OpDefinition> {
    let mut result = HashMap::new();
    for def in ast {
        let mut r = HashMap::new();
        for desc in &def.description {
            r.insert(desc.id.0.clone(), desc.value.0.clone());
        }
        result.insert(def.id.0.clone(), r);
    }
    Ok(result)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Program {
    pub magic: [u8; 4],
    pub entry: Option<Entry>,
    pub modules: Vec<Module>,
}

impl Program {
    pub fn from_modules(entry: Option<Entry>, modules: Vec<Module>) -> SimpleResult<Self> {
        if let Some(ref entry) = entry {
            if entry.module >= modules.len() {
                return Err(SimpleError::new(format!(
                    "Entry module index: {} out of bounds: {}",
                    entry.module,
                    modules.len()
                )));
            }
            if entry.function >= modules[entry.module].functions.len() {
                return Err(SimpleError::new(format!(
                    "Entry function index: {} out of bounds: {}",
                    entry.function,
                    modules[entry.module].functions.len()
                )));
            }
        }
        Ok(Self {
            magic: MAGIC_PROGRAM,
            modules,
            entry,
        })
    }

    pub fn from_json(source: &str) -> serde_json::Result<Self> {
        serde_json::from_str(source)
    }

    pub fn to_json(&self, pretty: bool) -> serde_json::Result<String> {
        if pretty {
            serde_json::to_string_pretty(self)
        } else {
            serde_json::to_string(self)
        }
    }

    pub fn modules_map(&self) -> Vec<(usize, String)> {
        self.modules
            .iter()
            .enumerate()
            .map(|(i, m)| (i, m.path.clone()))
            .collect()
    }

    pub fn find_module(&self, path: &str) -> Option<&Module> {
        self.modules.iter().find(|m| m.path == path)
    }

    pub fn find_module_struct(&self, path: &str, id: &str) -> Option<&Struct> {
        self.find_module(path)?.find_struct(id)
    }

    pub fn find_struct(&self, id: &str) -> Option<&Struct> {
        for m in &self.modules {
            for s in &m.structs {
                if s.id == id {
                    return Some(s);
                }
            }
        }
        None
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entry {
    pub module: usize,
    pub function: usize,
}

impl Entry {
    #[inline]
    pub fn new(module: usize, function: usize) -> Self {
        Self { module, function }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Module {
    pub magic: [u8; 4],
    pub shebang: Option<String>,
    pub path: String,
    pub meta: Vec<Meta>,
    pub imports: Vec<Import>,
    pub globals: Vec<Variable>,
    pub externs: Vec<Extern>,
    pub structs: Vec<Struct>,
    pub functions: Vec<Function>,
}

impl Module {
    pub fn from_ast(ast: &AstModule) -> CompilationResult<Self> {
        let mut meta = vec![];
        let mut imports = vec![];
        let mut globals = vec![];
        let mut externs = vec![];
        let mut structs = vec![];
        let mut functions = vec![];
        for instruction in &ast.instructions {
            match instruction {
                AstInstruction::Meta(m) => meta.extend(
                    m.0.iter()
                        .map(|m| Meta::from_ast(m))
                        .collect::<CompilationResult<Vec<Meta>>>()?,
                ),
                AstInstruction::Import(i) => imports.push(Import::from_ast(i)?),
                AstInstruction::Globals(g) => globals.extend(
                    g.iter()
                        .map(|v| Variable::from_ast(v))
                        .collect::<CompilationResult<Vec<Variable>>>()?,
                ),
                AstInstruction::Extern(e) => externs.push(Extern::from_ast(e)?),
                AstInstruction::Struct(s) => structs.push(Struct::from_ast(s)?),
                AstInstruction::Function(f) => functions.push(Function::from_ast(f)?),
            }
        }
        Ok(Self {
            magic: MAGIC_MODULE,
            shebang: if let Some(ref shebang) = ast.shebang {
                Some(shebang.clone())
            } else {
                None
            },
            path: "".to_owned(),
            meta,
            imports,
            globals,
            externs,
            structs,
            functions,
        })
    }

    #[inline]
    pub fn from_json(source: &str) -> serde_json::Result<Self> {
        serde_json::from_str(source)
    }

    #[inline]
    pub fn to_json(&self, pretty: bool) -> serde_json::Result<String> {
        if pretty {
            serde_json::to_string_pretty(self)
        } else {
            serde_json::to_string(self)
        }
    }

    pub fn globals_map(&self) -> Vec<(usize, String)> {
        self.structs
            .iter()
            .enumerate()
            .map(|(i, g)| (i, g.id.clone()))
            .collect()
    }

    pub fn externs_map(&self) -> Vec<(usize, String)> {
        self.externs
            .iter()
            .enumerate()
            .map(|(i, e)| (i, e.item.id.clone()))
            .collect()
    }

    pub fn structs_map(&self) -> Vec<(usize, String)> {
        self.structs
            .iter()
            .enumerate()
            .map(|(i, s)| (i, s.id.clone()))
            .collect()
    }

    pub fn functions_map(&self) -> Vec<(usize, String)> {
        self.functions
            .iter()
            .enumerate()
            .map(|(i, f)| (i, f.header.id.clone()))
            .collect()
    }

    #[inline]
    pub fn find_struct(&self, id: &str) -> Option<&Struct> {
        self.structs.iter().find(|s| s.id == id)
    }

    #[inline]
    pub fn find_function(&self, id: &str) -> Option<&Function> {
        self.functions.iter().find(|f| f.header.id == id)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Meta {
    pub id: String,
    pub args: Vec<MetaValue>,
}

impl Meta {
    pub fn from_ast(ast: &AstMetaField) -> CompilationResult<Self> {
        Ok(Self {
            id: ast.id.0.clone(),
            args: ast
                .args
                .iter()
                .map(|v| MetaValue::from_ast(v))
                .collect::<CompilationResult<Vec<MetaValue>>>()?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MetaValue {
    Named(String, Box<MetaValue>),
    Field(Meta),
    String(String),
    Number(Number),
}

impl MetaValue {
    pub fn from_ast(ast: &AstMetaValue) -> CompilationResult<Self> {
        Ok(match ast {
            AstMetaValue::Named(n, v) => {
                MetaValue::Named(n.0.clone(), Box::new(MetaValue::from_ast(v)?))
            }
            AstMetaValue::Field(v) => MetaValue::Field(Meta::from_ast(v)?),
            AstMetaValue::String(v) => MetaValue::String(v.0.clone()),
            AstMetaValue::Number(v) => MetaValue::Number(Number::from_ast(v)?),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Import {
    pub meta: Vec<Meta>,
    pub names: Vec<String>,
    pub module: String,
}

impl Import {
    pub fn from_ast(ast: &AstImport) -> CompilationResult<Self> {
        Ok(Self {
            meta: convert_ast_meta(&ast.meta)?,
            names: ast.names.iter().map(|n| n.0.clone()).collect(),
            module: ast.module.0.clone(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Extern {
    pub meta: Vec<Meta>,
    pub item: FunctionHeader,
    pub location_module: String,
    pub location_function: String,
}

impl Extern {
    pub fn from_ast(ast: &AstExtern) -> CompilationResult<Self> {
        Ok(Self {
            meta: convert_ast_meta(&ast.meta)?,
            item: FunctionHeader::from_ast(&ast.item)?,
            location_module: ast.location_module.0.clone(),
            location_function: ast.location_function.0.clone(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Variable {
    pub id: String,
    pub typeid: Type,
}

impl Variable {
    pub fn from_ast(ast: &AstVariable) -> CompilationResult<Self> {
        Ok(Self {
            id: ast.id.0.clone(),
            typeid: Type::from_ast(&ast.typeid)?,
        })
    }
}

impl ToString for Variable {
    fn to_string(&self) -> String {
        format!("{}:{}", self.id, self.typeid.to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Type {
    Identifier(String),
    Pointer(Box<Type>),
    Tuple(Vec<Type>),
}

impl Type {
    pub fn from_ast(ast: &AstType) -> CompilationResult<Self> {
        Ok(match ast {
            AstType::Identifier(v) => Type::Identifier(v.0.clone()),
            AstType::Pointer(v) => Type::Pointer(Box::new(Type::from_ast(v)?)),
            AstType::Tuple(v) => Type::Tuple(
                v.iter()
                    .map(|v| Type::from_ast(v))
                    .collect::<CompilationResult<Vec<Type>>>()?,
            ),
        })
    }

    pub fn as_identifier(&self) -> Option<&str> {
        match self {
            Type::Identifier(ref i) => Some(i),
            _ => None,
        }
    }

    pub fn as_pointer(&self) -> Option<&Type> {
        match self {
            Type::Pointer(ref t) => Some(t.as_ref()),
            _ => None,
        }
    }

    pub fn as_tuple(&self) -> Option<&[Type]> {
        match self {
            Type::Tuple(ref t) => Some(t),
            _ => None,
        }
    }
}

impl ToString for Type {
    fn to_string(&self) -> String {
        match self {
            Type::Identifier(t) => t.clone(),
            Type::Pointer(t) => format!("*{}", t.to_string()),
            Type::Tuple(t) => format!(
                "({})",
                t.iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

impl Default for Type {
    fn default() -> Self {
        Type::Identifier("".to_owned())
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Type::Identifier(ts) => {
                ts == "?"
                    || match other {
                        Type::Identifier(to) => to == "?" || ts == to,
                        _ => false,
                    }
            }
            Type::Pointer(ts) => match other {
                Type::Identifier(to) => to == "?",
                Type::Pointer(to) => ts == to,
                _ => false,
            },
            Type::Tuple(ts) => match other {
                Type::Identifier(to) => to == "?",
                Type::Tuple(to) => ts == to,
                _ => false,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Struct {
    pub meta: Vec<Meta>,
    pub export: bool,
    pub id: String,
    pub fields: Vec<Variable>,
}

impl Struct {
    pub fn from_ast(ast: &AstStruct) -> CompilationResult<Self> {
        Ok(Self {
            meta: convert_ast_meta(&ast.meta)?,
            export: ast.export,
            id: ast.id.0.clone(),
            fields: ast
                .fields
                .iter()
                .map(|v| Variable::from_ast(v))
                .collect::<CompilationResult<Vec<Variable>>>()?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Function {
    pub meta: Vec<Meta>,
    pub export: bool,
    pub header: FunctionHeader,
    pub locals: Vec<Variable>,
    pub body: Vec<BlockOp>,
}

impl Function {
    pub fn from_ast(ast: &AstFunction) -> CompilationResult<Self> {
        Ok(Self {
            meta: convert_ast_meta(&ast.meta)?,
            export: ast.export,
            header: FunctionHeader::from_ast(&ast.header)?,
            locals: ast
                .locals
                .iter()
                .map(|v| Variable::from_ast(v))
                .collect::<CompilationResult<Vec<Variable>>>()?,
            body: ast
                .ops
                .iter()
                .map(|o| BlockOp::from_ast(o))
                .collect::<CompilationResult<Vec<BlockOp>>>()?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionHeader {
    pub id: String,
    pub params: Vec<Variable>,
    pub typeid: Option<Type>,
}

impl FunctionHeader {
    pub fn from_ast(ast: &AstFunctionHeader) -> CompilationResult<Self> {
        Ok(Self {
            id: ast.id.0.clone(),
            params: ast
                .params
                .iter()
                .map(|v| Variable::from_ast(v))
                .collect::<CompilationResult<Vec<Variable>>>()?,
            typeid: if let Some(ref t) = ast.typeid {
                Some(Type::from_ast(&t)?)
            } else {
                None
            },
        })
    }
}

impl ToString for FunctionHeader {
    fn to_string(&self) -> String {
        let id = self.id.clone();
        let params = self
            .params
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<String>>()
            .join(", ");
        let typeid = if let Some(ref t) = self.typeid {
            t.to_string()
        } else {
            "".to_owned()
        };
        format!("fn {}({}){}", id, params, typeid)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BlockOp {
    Label(String),
    Operation(Operation),
}

impl BlockOp {
    pub fn from_ast(ast: &AstBlockOp) -> CompilationResult<Self> {
        Ok(match ast {
            AstBlockOp::Label(l) => BlockOp::Label(l.0.clone()),
            AstBlockOp::Operation(o) => BlockOp::Operation(Operation::from_ast(o)?),
        })
    }

    pub fn is_operation(&self) -> bool {
        match self {
            BlockOp::Operation(_) => true,
            _ => false,
        }
    }

    pub fn as_label(&self) -> Option<&String> {
        match self {
            BlockOp::Label(ref v) => Some(v),
            _ => None,
        }
    }

    pub fn as_operation(&self) -> Option<&Operation> {
        match self {
            BlockOp::Operation(ref v) => Some(v),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Operation {
    pub meta: Vec<Meta>,
    pub id: String,
    pub params: Vec<Value>,
    pub targets: Vec<Value>,
}

impl Operation {
    pub fn from_ast(ast: &AstOperation) -> CompilationResult<Self> {
        Ok(Self {
            meta: convert_ast_meta(&ast.meta)?,
            id: ast.id.0.clone(),
            params: ast
                .params
                .iter()
                .map(|v| Value::from_ast(v))
                .collect::<CompilationResult<Vec<Value>>>()?,
            targets: ast
                .targets
                .iter()
                .map(|t| Value::from_ast(t))
                .collect::<CompilationResult<Vec<Value>>>()?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Value {
    Ref(Box<Value>, Option<Box<Access>>),
    Deref(Box<Value>, Option<Box<Access>>),
    FunctionCall(String, Vec<Value>, Option<Box<Access>>),
    Tuple(Vec<Value>, Option<Box<Access>>),
    String(String, Type),
    Number(Number),
    OperationInline(String, Vec<Value>, Option<Box<Access>>),
    Variable(String, Option<Box<Access>>),
}

impl Value {
    pub fn from_ast(ast: &AstValue) -> CompilationResult<Self> {
        Ok(match ast {
            AstValue::Ref(v, a) => Value::Ref(
                Box::new(Value::from_ast(v)?),
                if let Some(a) = a {
                    Some(Box::new(Access::from_ast(a)?))
                } else {
                    None
                },
            ),
            AstValue::Deref(v, a) => Value::Deref(
                Box::new(Value::from_ast(v)?),
                if let Some(a) = a {
                    Some(Box::new(Access::from_ast(a)?))
                } else {
                    None
                },
            ),
            AstValue::FunctionCall(i, v, a) => Value::FunctionCall(
                i.0.clone(),
                v.iter()
                    .map(|v| Value::from_ast(v))
                    .collect::<CompilationResult<Vec<Value>>>()?,
                if let Some(a) = a {
                    Some(Box::new(Access::from_ast(a)?))
                } else {
                    None
                },
            ),
            AstValue::Tuple(v, a) => Value::Tuple(
                v.iter()
                    .map(|v| Value::from_ast(v))
                    .collect::<CompilationResult<Vec<Value>>>()?,
                if let Some(a) = a {
                    Some(Box::new(Access::from_ast(a)?))
                } else {
                    None
                },
            ),
            AstValue::String(v) => Value::String(v.0.clone(), Type::from_ast(&v.1)?),
            AstValue::Number(v) => Value::Number(Number::from_ast(v)?),
            AstValue::OperationInline(i, v, a) => Value::OperationInline(
                i.0.clone(),
                v.iter()
                    .map(|v| Value::from_ast(v))
                    .collect::<CompilationResult<Vec<Value>>>()?,
                if let Some(a) = a {
                    Some(Box::new(Access::from_ast(a)?))
                } else {
                    None
                },
            ),
            AstValue::Variable(v, a) => Value::Variable(
                v.0.clone(),
                if let Some(a) = a {
                    Some(Box::new(Access::from_ast(a)?))
                } else {
                    None
                },
            ),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Access {
    Tuple(i64, Option<Box<Access>>),
    Variable(String, Option<Box<Access>>),
}

impl Access {
    pub fn from_ast(ast: &AstAccess) -> CompilationResult<Self> {
        Ok(match ast {
            AstAccess::Variable(v, a) => Access::Variable(
                v.0.clone(),
                if let Some(a) = a {
                    Some(Box::new(Access::from_ast(a)?))
                } else {
                    None
                },
            ),
            AstAccess::Tuple(v, a) => Access::Tuple(
                v.0,
                if let Some(a) = a {
                    Some(Box::new(Access::from_ast(a)?))
                } else {
                    None
                },
            ),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Number {
    Integer(i64, Type),
    Float(f64, Type),
}

impl Number {
    pub fn from_ast(ast: &AstNumber) -> CompilationResult<Self> {
        Ok(match ast {
            AstNumber::Integer(v) => Number::Integer(v.0, Type::from_ast(&v.1)?),
            AstNumber::Float(v) => Number::Float(v.0, Type::from_ast(&v.1)?),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct OpsDescriptor {
    pub meta: Vec<Meta>,
    pub rules: Vec<OpRule>,
}

impl OpsDescriptor {
    pub fn from_ast(ast: &AstOpsDescriptor) -> CompilationResult<Self> {
        Ok(OpsDescriptor {
            meta: convert_ast_meta(&ast.meta)?,
            rules: ast
                .rules
                .iter()
                .map(|r| OpRule::from_ast(r))
                .collect::<CompilationResult<Vec<OpRule>>>()?,
        })
    }

    pub fn merge(descriptors: &[OpsDescriptor]) -> Self {
        OpsDescriptor {
            meta: descriptors.iter().flat_map(|d| d.meta.clone()).collect(),
            rules: descriptors.iter().flat_map(|d| d.rules.clone()).collect(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpRule {
    pub meta: Vec<Meta>,
    pub id: String,
    pub params: Vec<OpParam>,
    pub targets: Vec<Type>,
    pub definition: OpDefinition,
}

impl OpRule {
    pub fn from_ast(ast: &AstOpRule) -> CompilationResult<Self> {
        Ok(OpRule {
            meta: convert_ast_meta(&ast.meta)?,
            id: ast.id.0.clone(),
            params: ast
                .params
                .iter()
                .map(|p| OpParam::from_ast(p))
                .collect::<CompilationResult<Vec<OpParam>>>()?,
            targets: ast
                .targets
                .iter()
                .map(|t| Type::from_ast(t))
                .collect::<Result<Vec<Type>, CompilationError>>()?,
            definition: convert_ast_op_definition(&ast.definition)?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpParam {
    pub id: String,
    pub typeid: Type,
}

impl OpParam {
    pub fn from_ast(ast: &AstOpParam) -> CompilationResult<Self> {
        Ok(OpParam {
            id: ast.id.0.clone(),
            typeid: Type::from_ast(&ast.typeid)?,
        })
    }
}

pub type OpDefinition = HashMap<String, HashMap<String, String>>;
