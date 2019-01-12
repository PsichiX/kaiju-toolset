use crate::ast::*;
use crate::error::*;
use pest::error::{Error, InputLocation, LineColLocation};
use pest::iterators::Pair;
use pest::Parser;

#[derive(Parser)]
#[grammar = "grammars/kaiju.pest"]
pub struct KaijuParser;

fn translate_error(err: Error<Rule>) -> Error<Rule> {
    err.renamed_rules(|rule| match *rule {
        Rule::shebang => "shebang (`#!{ ... }`)".to_owned(),
        Rule::identifier => "identifier (`name` or `$extra.name`)".to_owned(),
        Rule::identifier_simple => "simple identifier (`name`)".to_owned(),
        Rule::identifier_extended => "extended identifier (`$extra.name`)".to_owned(),
        Rule::type_ann => "type annotation (`:type`, `:*type`, `:(typeA, typeB)`)".to_owned(),
        Rule::type_ => "type (`type`, `*type`, `(typeA, typeB)`)".to_owned(),
        Rule::tuple_type => "tuple type (`(typeA, typeB)`)".to_owned(),
        Rule::pointer_type => "pointer type (`*type`)".to_owned(),
        Rule::string => "string (`'hello'`)".to_owned(),
        Rule::integer => "integer (`42`, `42u8`, `0x2A`, `0x2Au8`)".to_owned(),
        Rule::integer_inner => "integer (`42`, `0x2A`)".to_owned(),
        Rule::hex => "hex (`0x2A`)".to_owned(),
        Rule::float => "float (`4.2`, `4.2f64`, `4.2e7`, `4.2e7f64`)".to_owned(),
        Rule::float_inner => "float (`4.2`, `4.2e7`)".to_owned(),
        Rule::number => "number (`42`, `4.2`, `4.2e7`, `0x2A`, `42u8`, `4.2e7f64`)".to_owned(),
        Rule::tuple_value => "tuple value (`(42, '42', &4.2)`)".to_owned(),
        Rule::variable => "variable (`a:i32`)".to_owned(),
        Rule::variable_access => "variable access (`a.foo`)".to_owned(),
        Rule::tuple_access => "tuple access (`a.0`)".to_owned(),
        Rule::ref_value => "reference access (`&<a>`, `&<a.v>`)".to_owned(),
        Rule::deref_value => "dereference value (`*<a>`, `*<a.v>`)".to_owned(),
        Rule::label => "label (`name:`)".to_owned(),
        Rule::block => "code block (`{ ... }`)".to_owned(),
        Rule::operation => "operation (`op param => target`)".to_owned(),
        Rule::operation_inline => "inline operation (`(op param)`)".to_owned(),
        Rule::operation_id => "operation identifier (`name`)".to_owned(),
        Rule::operation_params => "operation params (`param1 param2 ...`)".to_owned(),
        Rule::operation_targets => "operation targets (`=> target1 target2`)".to_owned(),
        Rule::function => "function (`fn foo(v:i32):i32 { ... }`)".to_owned(),
        Rule::function_header => "function header (`fn foo(v:i32):i32`)".to_owned(),
        Rule::function_params => "function params (`(a:i32, b:f64)`)".to_owned(),
        Rule::function_locals => "function locals (`<a:i32, b:f64>`)".to_owned(),
        Rule::function_call => "function call (`foo(42, 4.2)`)".to_owned(),
        Rule::function_call_args => "function call args (`foo(42, 4.2)`)".to_owned(),
        Rule::meta_global => "global meta (`#![attrib];`)".to_owned(),
        Rule::meta_local => "local meta (`#[attrib]`)".to_owned(),
        Rule::meta_fields => "meta fields (`#[attrib1, attrib2]`)".to_owned(),
        Rule::meta_field => "meta field (`#[attrib()]`)".to_owned(),
        Rule::meta_field_args => "meta field args (`#[attrib(arg)]`)".to_owned(),
        Rule::meta_value => {
            "meta field value (`#[attrib(42, 4.2, '42', named = 42, attrib2())]`)".to_owned()
        }
        Rule::meta_named_value => "meta field named value (`#[attrib(named = 42)]`)".to_owned(),
        Rule::extern_ => "external function (`extern fn log() from console:log;`)".to_owned(),
        Rule::extern_item => "external function header (`fn foo(v:i32):i32`)".to_owned(),
        Rule::extern_location => "external function location (`module:funname`)".to_owned(),
        Rule::import => "import symbols (`import { foo, bar } from 'std';`)".to_owned(),
        Rule::import_name => "import symbol identifier (`foo`)".to_owned(),
        Rule::import_names => "import symbol identifiers (`{ foo, bar }`)".to_owned(),
        Rule::import_module => "import module path (`'path/to/module`)".to_owned(),
        Rule::globals => "globals (`<a:i32, b:f64>`)".to_owned(),
        Rule::struct_ => "struct (`struct A { a:i32, b:f64 }`)".to_owned(),
        Rule::struct_fields => "struct fields (`{ a:i32, b:f64 }`)".to_owned(),
        Rule::op_rule => "rule (`add @value:a @value:b => r {}`)".to_owned(),
        Rule::op_rule_def => "rule definition (`{ field: { name: 'value' } }`)".to_owned(),
        Rule::op_rule_def_field => "rule definition field (`field: { name: 'value' }`)".to_owned(),
        Rule::op_rule_def_field_id => "field id (`foo`)".to_owned(),
        Rule::op_rule_def_field_desc => "field description (`{ name: 'value' }`)".to_owned(),
        Rule::op_rule_def_field_desc_field => {
            "field description parameter (`name: 'value'`)".to_owned()
        }
        _ => format!("{:?}", rule),
    })
}

pub fn parse_module(source: &str) -> CompilationResult<AstModule> {
    match KaijuParser::parse(Rule::module, source) {
        Ok(mut ast) => {
            let pair = ast.next().unwrap();
            match pair.as_rule() {
                Rule::module => Ok(parse_module_inner(pair)),
                _ => unreachable!(),
            }
        }
        Err(err) => {
            let location = match err.location {
                InputLocation::Pos(a) => (a, a),
                InputLocation::Span((a, b)) => (a, b),
            };
            let (line, column) = match err.line_col {
                LineColLocation::Pos((a, b)) => ((a, a), (b, b)),
                LineColLocation::Span((a, b), (c, d)) => ((a, c), (b, d)),
            };
            let err = translate_error(err);
            Err(CompilationError {
                message: "".to_owned(),
                location,
                line,
                column,
                pretty: format!("{}", err),
            })
        }
    }
}

pub fn parse_ops_descriptor(source: &str) -> CompilationResult<AstOpsDescriptor> {
    match KaijuParser::parse(Rule::ops_descriptor, source) {
        Ok(mut ast) => {
            let pair = ast.next().unwrap();
            match pair.as_rule() {
                Rule::ops_descriptor => Ok(parse_ops_descriptor_inner(pair)),
                _ => unreachable!(),
            }
        }
        Err(err) => {
            let location = match err.location {
                InputLocation::Pos(a) => (a, a),
                InputLocation::Span((a, b)) => (a, b),
            };
            let (line, column) = match err.line_col {
                LineColLocation::Pos((a, b)) => ((a, a), (b, b)),
                LineColLocation::Span((a, b), (c, d)) => ((a, c), (b, d)),
            };
            let err = translate_error(err);
            Err(CompilationError {
                message: "".to_owned(),
                location,
                line,
                column,
                pretty: format!("{}", err),
            })
        }
    }
}

fn parse_module_inner(pair: Pair<Rule>) -> AstModule {
    let mut shebang = None;
    let mut instructions = vec![];
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::shebang => shebang = Some(p.as_str().trim().to_owned()),
            Rule::instruction => {
                instructions.push(parse_instruction(p.into_inner().next().unwrap()))
            }
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }
    AstModule {
        shebang,
        instructions,
    }
}

fn parse_instruction(pair: Pair<Rule>) -> AstInstruction {
    match pair.as_rule() {
        Rule::meta_global => AstInstruction::Meta(parse_meta(pair)),
        Rule::import => AstInstruction::Import(parse_import(pair)),
        Rule::globals => AstInstruction::Globals(pair.into_inner().map(parse_variable).collect()),
        Rule::extern_ => AstInstruction::Extern(parse_extern(pair)),
        Rule::struct_ => AstInstruction::Struct(parse_struct(pair)),
        Rule::function => AstInstruction::Function(parse_function(pair)),
        _ => unreachable!(),
    }
}

fn parse_meta(pair: Pair<Rule>) -> AstMeta {
    AstMeta(
        pair.into_inner()
            .next()
            .unwrap()
            .into_inner()
            .map(parse_meta_field)
            .collect(),
    )
}

fn parse_meta_field(pair: Pair<Rule>) -> AstMetaField {
    let mut inner = pair.into_inner();
    let id = parse_identifier(inner.next().unwrap());
    let args = if let Some(p) = inner.next() {
        p.into_inner()
            .map(|p| parse_meta_value(p.into_inner().next().unwrap()))
            .collect()
    } else {
        vec![]
    };
    AstMetaField { id, args }
}

fn parse_meta_value(pair: Pair<Rule>) -> AstMetaValue {
    match pair.as_rule() {
        Rule::meta_named_value => {
            let (id, val) = parse_meta_named_value(pair);
            AstMetaValue::Named(id, val)
        }
        Rule::meta_field => AstMetaValue::Field(parse_meta_field(pair)),
        Rule::string => AstMetaValue::String(parse_string(pair)),
        Rule::number => AstMetaValue::Number(parse_number(pair)),
        _ => unreachable!(),
    }
}

fn parse_meta_named_value(pair: Pair<Rule>) -> (AstIdentifier, Box<AstMetaValue>) {
    let mut inner = pair.into_inner();
    let pid = inner.next().unwrap();
    let pval = inner.next().unwrap();
    (
        parse_identifier(pid),
        Box::new(parse_meta_value(pval.into_inner().next().unwrap())),
    )
}

fn parse_identifier(pair: Pair<Rule>) -> AstIdentifier {
    let p = pair.into_inner().next().unwrap();
    match p.as_rule() {
        Rule::identifier_simple => AstIdentifier(p.as_str().to_owned()),
        Rule::identifier_extended => {
            AstIdentifier(p.into_inner().next().unwrap().as_str().to_owned())
        }
        _ => unreachable!(),
    }
}

fn parse_string(pair: Pair<Rule>) -> AstString {
    let mut inner = pair.into_inner();
    let value = inner.next().unwrap().as_str().to_owned();
    let type_ = if let Some(t) = inner.next() {
        parse_type(t)
    } else {
        AstType::Pointer(Box::new(AstType::Identifier(AstIdentifier(
            "{string}".to_string(),
        ))))
    };
    AstString(value, type_)
}

fn parse_number(pair: Pair<Rule>) -> AstNumber {
    let p = pair.into_inner().next().unwrap();
    match p.as_rule() {
        Rule::integer => AstNumber::Integer(parse_integer(p)),
        Rule::float => AstNumber::Float(parse_float(p)),
        _ => unreachable!(),
    }
}

fn parse_integer(pair: Pair<Rule>) -> AstInteger {
    let mut inner = pair.into_inner();
    let value = inner.next().unwrap().as_str();
    let type_ = if let Some(t) = inner.next() {
        parse_type(t)
    } else {
        AstType::Identifier(AstIdentifier("{integer}".to_string()))
    };
    if value.starts_with("0x") {
        AstInteger(
            i64::from_str_radix(value.trim_start_matches("0x"), 16).unwrap(),
            type_,
        )
    } else {
        AstInteger(value.parse().unwrap(), type_)
    }
}

fn parse_float(pair: Pair<Rule>) -> AstFloat {
    let mut inner = pair.into_inner();
    let value = inner.next().unwrap().as_str();
    let type_ = if let Some(t) = inner.next() {
        parse_type(t)
    } else {
        AstType::Identifier(AstIdentifier("{float}".to_string()))
    };
    AstFloat(value.parse().unwrap(), type_)
}

fn parse_import(pair: Pair<Rule>) -> AstImport {
    let mut meta = vec![];
    let mut names = vec![];
    let mut module = AstString::default();
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::meta_local => meta.push(parse_meta(p)),
            Rule::import_names => names.extend(
                p.into_inner()
                    .map(|p| parse_identifier(p.into_inner().next().unwrap()))
                    .collect::<Vec<AstIdentifier>>(),
            ),
            Rule::import_name => names.push(parse_identifier(p.into_inner().next().unwrap())),
            Rule::import_module => module = parse_string(p.into_inner().next().unwrap()),
            _ => unreachable!(),
        }
    }
    AstImport {
        meta,
        names,
        module,
    }
}

fn parse_variable(pair: Pair<Rule>) -> AstVariable {
    let mut inner = pair.into_inner();
    let id = parse_identifier(inner.next().unwrap());
    let typeid = parse_type(inner.next().unwrap().into_inner().next().unwrap());
    AstVariable { id, typeid }
}

fn parse_type(pair: Pair<Rule>) -> AstType {
    let p = pair.into_inner().next().unwrap();
    match p.as_rule() {
        Rule::tuple_type => AstType::Tuple(p.into_inner().map(parse_type).collect()),
        Rule::pointer_type => {
            AstType::Pointer(Box::new(parse_type(p.into_inner().next().unwrap())))
        }
        Rule::identifier => AstType::Identifier(parse_identifier(p)),
        _ => unreachable!(),
    }
}

fn parse_extern(pair: Pair<Rule>) -> AstExtern {
    let mut meta = vec![];
    let mut item = AstFunctionHeader {
        id: AstIdentifier::default(),
        params: vec![],
        typeid: None,
    };
    let mut location_module = AstIdentifier::default();
    let mut location_function = AstIdentifier::default();
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::meta_local => meta.push(parse_meta(p)),
            Rule::extern_item => item = parse_function_header(p),
            Rule::extern_location => {
                let mut inner = p.into_inner();
                location_module = parse_identifier(inner.next().unwrap());
                location_function = parse_identifier(inner.next().unwrap());
            }
            _ => unreachable!(),
        }
    }
    AstExtern {
        meta,
        item,
        location_module,
        location_function,
    }
}

fn parse_struct(pair: Pair<Rule>) -> AstStruct {
    let mut meta = vec![];
    let mut export = false;
    let mut id = AstIdentifier::default();
    let mut fields = vec![];
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::meta_local => meta.push(parse_meta(p)),
            Rule::export => export = true,
            Rule::identifier => id = parse_identifier(p),
            Rule::struct_fields => fields = p.into_inner().map(parse_variable).collect(),
            _ => unreachable!(),
        }
    }
    AstStruct {
        meta,
        export,
        id,
        fields,
    }
}

fn parse_function(pair: Pair<Rule>) -> AstFunction {
    let mut meta = vec![];
    let mut export = false;
    let mut header = None;
    let mut locals = vec![];
    let mut ops = vec![];
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::meta_local => meta.push(parse_meta(p)),
            Rule::export => export = true,
            Rule::function_header => header = Some(parse_function_header(p)),
            Rule::function_locals => locals = p.into_inner().map(parse_variable).collect(),
            Rule::block => ops = parse_block(p),
            _ => unreachable!(),
        }
    }
    AstFunction {
        meta,
        export,
        header: header.unwrap(),
        locals,
        ops,
    }
}

fn parse_function_header(pair: Pair<Rule>) -> AstFunctionHeader {
    let mut id = AstIdentifier::default();
    let mut params = vec![];
    let mut typeid = None;
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::identifier => id = parse_identifier(p),
            Rule::function_params => params = p.into_inner().map(parse_variable).collect(),
            Rule::type_ann => typeid = Some(parse_type(p.into_inner().next().unwrap())),
            _ => unreachable!(),
        }
    }
    AstFunctionHeader { id, params, typeid }
}

fn parse_block(pair: Pair<Rule>) -> Vec<AstBlockOp> {
    pair.into_inner()
        .map(|p| match p.as_rule() {
            Rule::label => AstBlockOp::Label(parse_identifier(p.into_inner().next().unwrap())),
            Rule::operation => AstBlockOp::Operation(parse_operation(p)),
            _ => unreachable!(),
        })
        .collect()
}

fn parse_operation(pair: Pair<Rule>) -> AstOperation {
    let mut meta = vec![];
    let mut id = AstIdentifier::default();
    let mut params = vec![];
    let mut targets = vec![];
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::meta_local => meta.push(parse_meta(p)),
            Rule::operation_id => id = parse_identifier(p),
            Rule::operation_params => params = p.into_inner().map(parse_value).collect(),
            Rule::operation_targets => targets = p.into_inner().map(parse_value).collect(),
            _ => unreachable!(),
        }
    }
    AstOperation {
        meta,
        id,
        params,
        targets,
    }
}

fn parse_value(pair: Pair<Rule>) -> AstValue {
    let mut inner = pair.into_inner();
    let p = inner.next().unwrap().into_inner().next().unwrap();
    let a = inner.next().map(|p| Box::new(parse_access(p)));
    match p.as_rule() {
        Rule::ref_value => parse_ref(p, a),
        Rule::deref_value => parse_deref(p, a),
        Rule::function_call => parse_function_call(p, a),
        Rule::tuple_value => AstValue::Tuple(p.into_inner().map(parse_value).collect(), a),
        Rule::string => AstValue::String(parse_string(p)),
        Rule::number => AstValue::Number(parse_number(p)),
        Rule::operation_inline => parse_operation_inline(p, a),
        Rule::variable_value => {
            AstValue::Variable(parse_identifier(p.into_inner().next().unwrap()), a)
        }
        _ => unreachable!(),
    }
}

fn parse_ref(pair: Pair<Rule>, access: Option<Box<AstAccess>>) -> AstValue {
    AstValue::Ref(
        Box::new(parse_value(pair.into_inner().next().unwrap())),
        access,
    )
}

fn parse_deref(pair: Pair<Rule>, access: Option<Box<AstAccess>>) -> AstValue {
    AstValue::Deref(
        Box::new(parse_value(pair.into_inner().next().unwrap())),
        access,
    )
}

fn parse_function_call(pair: Pair<Rule>, access: Option<Box<AstAccess>>) -> AstValue {
    let mut inner = pair.into_inner();
    let id = parse_identifier(inner.next().unwrap());
    let params = inner
        .next()
        .unwrap()
        .into_inner()
        .map(parse_value)
        .collect();
    AstValue::FunctionCall(id, params, access)
}

fn parse_access(pair: Pair<Rule>) -> AstAccess {
    let p = pair.into_inner().next().unwrap();
    match p.as_rule() {
        Rule::variable_access => parse_variable_access(p),
        Rule::tuple_access => parse_tuple_access(p),
        _ => unreachable!(),
    }
}

fn parse_variable_access(pair: Pair<Rule>) -> AstAccess {
    let mut inner = pair.into_inner();
    let id = parse_identifier(inner.next().unwrap());
    let next = if let Some(p) = inner.next() {
        Some(Box::new(parse_access(p)))
    } else {
        None
    };
    AstAccess::Variable(id, next)
}

fn parse_tuple_access(pair: Pair<Rule>) -> AstAccess {
    let mut inner = pair.into_inner();
    let id = parse_integer(inner.next().unwrap());
    let next = if let Some(p) = inner.next() {
        Some(Box::new(parse_access(p)))
    } else {
        None
    };
    AstAccess::Tuple(id, next)
}

fn parse_operation_inline(pair: Pair<Rule>, access: Option<Box<AstAccess>>) -> AstValue {
    let mut inner = pair.into_inner();
    let id = parse_identifier(inner.next().unwrap());
    let params = inner
        .next()
        .unwrap()
        .into_inner()
        .map(parse_value)
        .collect();
    AstValue::OperationInline(id, params, access)
}

fn parse_ops_descriptor_inner(pair: Pair<Rule>) -> AstOpsDescriptor {
    let mut meta = vec![];
    let mut rules = vec![];
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::meta_global => meta.push(parse_meta(p)),
            Rule::op_rule => rules.push(parse_op_rule(p)),
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }
    AstOpsDescriptor { meta, rules }
}

fn parse_op_rule(pair: Pair<Rule>) -> AstOpRule {
    let mut meta = vec![];
    let mut id = AstIdentifier::default();
    let mut params = vec![];
    let mut targets = vec![];
    let mut definition = vec![];
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::meta_local => meta.push(parse_meta(p)),
            Rule::identifier_simple => id = AstIdentifier(p.as_str().to_owned()),
            Rule::op_param => params.push(parse_op_param(p)),
            Rule::op_targets => targets = p.into_inner().map(parse_type).collect(),
            Rule::op_rule_def => definition = p.into_inner().map(parse_op_rule_def).collect(),
            _ => unreachable!(),
        }
    }
    AstOpRule {
        meta,
        id,
        params,
        targets,
        definition,
    }
}

fn parse_op_param(pair: Pair<Rule>) -> AstOpParam {
    let mut inner = pair.into_inner();
    AstOpParam {
        id: AstIdentifier(inner.next().unwrap().as_str().to_owned()),
        typeid: parse_op_value(inner.next().unwrap()),
    }
}

fn parse_op_value(pair: Pair<Rule>) -> AstType {
    match pair.as_rule() {
        Rule::op_value => parse_type(pair.into_inner().next().unwrap()),
        _ => unreachable!(),
    }
}

fn parse_op_rule_def(pair: Pair<Rule>) -> AstOpRuleDef {
    let mut inner = pair.into_inner();
    AstOpRuleDef {
        id: AstIdentifier(inner.next().unwrap().as_str().to_owned()),
        description: inner
            .next()
            .unwrap()
            .into_inner()
            .map(parse_op_rule_def_desc)
            .collect(),
    }
}

fn parse_op_rule_def_desc(pair: Pair<Rule>) -> AstOpRuleDefDesc {
    let mut inner = pair.into_inner();
    AstOpRuleDefDesc {
        id: AstIdentifier(inner.next().unwrap().as_str().to_owned()),
        value: parse_string(inner.next().unwrap()),
    }
}
