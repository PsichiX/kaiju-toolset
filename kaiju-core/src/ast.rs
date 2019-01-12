use serde_json;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstModule {
    pub shebang: Option<String>,
    pub instructions: Vec<AstInstruction>,
}

impl AstModule {
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstIdentifier(pub String);

impl Default for AstIdentifier {
    fn default() -> Self {
        AstIdentifier("".to_owned())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstString(pub String, pub AstType);

impl Default for AstString {
    fn default() -> Self {
        AstString("".to_owned(), AstType::default())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstFloat(pub f64, pub AstType);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstInteger(pub i64, pub AstType);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AstNumber {
    Float(AstFloat),
    Integer(AstInteger),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AstValue {
    Ref(Box<AstValue>, Option<Box<AstAccess>>),
    Deref(Box<AstValue>, Option<Box<AstAccess>>),
    FunctionCall(AstIdentifier, Vec<AstValue>, Option<Box<AstAccess>>),
    Tuple(Vec<AstValue>, Option<Box<AstAccess>>),
    String(AstString),
    Number(AstNumber),
    OperationInline(AstIdentifier, Vec<AstValue>, Option<Box<AstAccess>>),
    Variable(AstIdentifier, Option<Box<AstAccess>>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AstAccess {
    Tuple(AstInteger, Option<Box<AstAccess>>),
    Variable(AstIdentifier, Option<Box<AstAccess>>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstVariable {
    pub id: AstIdentifier,
    pub typeid: AstType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AstType {
    Tuple(Vec<AstType>),
    Pointer(Box<AstType>),
    Identifier(AstIdentifier),
}

impl Default for AstType {
    fn default() -> Self {
        AstType::Identifier(AstIdentifier::default())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AstInstruction {
    Meta(AstMeta),
    Import(AstImport),
    Globals(Vec<AstVariable>),
    Extern(AstExtern),
    Struct(AstStruct),
    Function(AstFunction),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstMeta(pub Vec<AstMetaField>);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstMetaField {
    pub id: AstIdentifier,
    pub args: Vec<AstMetaValue>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AstMetaValue {
    Named(AstIdentifier, Box<AstMetaValue>),
    Field(AstMetaField),
    String(AstString),
    Number(AstNumber),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstImport {
    pub meta: Vec<AstMeta>,
    pub names: Vec<AstIdentifier>,
    pub module: AstString,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstExtern {
    pub meta: Vec<AstMeta>,
    pub item: AstFunctionHeader,
    pub location_module: AstIdentifier,
    pub location_function: AstIdentifier,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstStruct {
    pub meta: Vec<AstMeta>,
    pub export: bool,
    pub id: AstIdentifier,
    pub fields: Vec<AstVariable>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstFunction {
    pub meta: Vec<AstMeta>,
    pub export: bool,
    pub header: AstFunctionHeader,
    pub locals: Vec<AstVariable>,
    pub ops: Vec<AstBlockOp>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstFunctionHeader {
    pub id: AstIdentifier,
    pub params: Vec<AstVariable>,
    pub typeid: Option<AstType>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AstBlockOp {
    Label(AstIdentifier),
    Operation(AstOperation),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstOperation {
    pub meta: Vec<AstMeta>,
    pub id: AstIdentifier,
    pub params: Vec<AstValue>,
    pub targets: Vec<AstValue>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstOpsDescriptor {
    pub meta: Vec<AstMeta>,
    pub rules: Vec<AstOpRule>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstOpRule {
    pub meta: Vec<AstMeta>,
    pub id: AstIdentifier,
    pub params: Vec<AstOpParam>,
    pub targets: Vec<AstType>,
    pub definition: Vec<AstOpRuleDef>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstOpParam {
    pub id: AstIdentifier,
    pub typeid: AstType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstOpRuleDef {
    pub id: AstIdentifier,
    pub description: Vec<AstOpRuleDefDesc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstOpRuleDefDesc {
    pub id: AstIdentifier,
    pub value: AstString,
}
