#![allow(clippy::too_many_arguments)]

use crate::error::*;
use crate::program::*;
use std::collections::{HashMap, HashSet};

pub trait DeepValidator {
    fn filter_module(_module: &Module, _program: &Program, _validator: &Validator) -> bool {
        true
    }
    fn filter_struct(
        _struct_: &Struct,
        _module: &Module,
        _program: &Program,
        _validator: &Validator,
    ) -> bool {
        true
    }
    fn filter_function(
        _function: &Function,
        _module: &Module,
        _program: &Program,
        _validator: &Validator,
    ) -> bool {
        true
    }
    fn filter_op(
        _op: &Operation,
        _function: &Function,
        _module: &Module,
        _program: &Program,
        _validator: &Validator,
    ) -> bool {
        true
    }
    fn validate_program(_program: &Program, _validator: &Validator) -> SimpleResult<()> {
        Ok(())
    }
    fn validate_module(
        _module: &Module,
        _program: &Program,
        _validator: &Validator,
    ) -> SimpleResult<()> {
        Ok(())
    }
    fn validate_op(
        _op: &Operation,
        _function: &Function,
        _module: &Module,
        _program: &Program,
        _rule: &Rule,
        _validator: &Validator,
    ) -> SimpleResult<()> {
        Ok(())
    }
    fn transform_module(
        module: Module,
        _program: &Program,
        _validator: &Validator,
    ) -> SimpleResult<Module> {
        Ok(module)
    }
}

pub struct EmptyDeepValidator {}
impl DeepValidator for EmptyDeepValidator {
    fn transform_module(
        module: Module,
        _program: &Program,
        _validator: &Validator,
    ) -> SimpleResult<Module> {
        transform_module_auto_types(module)
    }
}

pub struct Rule {
    pub id: String,
    pub params: Vec<OpParam>,
    pub targets: Vec<Type>,
}

impl Rule {
    pub fn from_op_rule(rule: &OpRule) -> Self {
        Self {
            id: rule.id.clone(),
            params: rule.params.clone(),
            targets: rule.targets.clone(),
        }
    }
}

type ModuleTypeIDs = HashMap<String, Option<String>>;
type ModuleFunctionIDs = HashMap<String, Option<Type>>;
type FunctionVariablesIDs = HashMap<String, Type>;

pub fn resolve_module_types(module: &Module, program: &Program) -> SimpleResult<ModuleTypeIDs> {
    let mut types = HashMap::new();
    types.insert("i8".to_owned(), None);
    types.insert("i16".to_owned(), None);
    types.insert("i32".to_owned(), None);
    types.insert("i64".to_owned(), None);
    types.insert("u8".to_owned(), None);
    types.insert("i16".to_owned(), None);
    types.insert("u32".to_owned(), None);
    types.insert("u64".to_owned(), None);
    types.insert("f32".to_owned(), None);
    types.insert("f64".to_owned(), None);
    types.insert("isize".to_owned(), None);
    types.insert("usize".to_owned(), None);
    for s in &module.structs {
        if types.contains_key(&s.id) {
            return Err(SimpleError::new(format!(
                "Struct name already taken: {}",
                s.id
            )));
        }
        types.insert(s.id.clone(), Some(module.path.clone()));
    }
    for i in &module.imports {
        if let Some(m) = program.find_module(&i.module) {
            for n in &i.names {
                if let Some(s) = m.find_struct(n) {
                    if types.contains_key(&s.id) {
                        return Err(SimpleError::new(format!(
                            "Struct name already taken: {}",
                            s.id
                        )));
                    }
                    types.insert(s.id.clone(), Some(m.path.clone()));
                }
            }
        } else {
            return Err(SimpleError::new(format!(
                "There is no module: {}",
                i.module
            )));
        }
    }
    Ok(types)
}

pub fn resolve_module_functions(
    module: &Module,
    program: &Program,
) -> SimpleResult<ModuleFunctionIDs> {
    let mut functions = HashMap::new();
    for f in &module.functions {
        if functions.contains_key(&f.header.id) {
            return Err(SimpleError::new(format!(
                "Function name already taken: {}",
                f.header.id
            )));
        }
        functions.insert(f.header.id.clone(), f.header.typeid.clone());
    }
    for i in &module.imports {
        if let Some(m) = program.find_module(&i.module) {
            for n in &i.names {
                if let Some(f) = m.find_function(n) {
                    if functions.contains_key(&f.header.id) {
                        return Err(SimpleError::new(format!(
                            "Function name already taken: {}",
                            f.header.id
                        )));
                    }
                    functions.insert(f.header.id.clone(), f.header.typeid.clone());
                }
            }
        } else {
            return Err(SimpleError::new(format!(
                "There is no module: {}",
                i.module
            )));
        }
    }
    Ok(functions)
}

pub fn resolve_function_variables(
    function: &Function,
    module: &Module,
) -> SimpleResult<FunctionVariablesIDs> {
    let mut variables = HashMap::new();
    for g in &module.globals {
        if variables.contains_key(&g.id) {
            return Err(SimpleError::new(format!(
                "Variable name already taken: {}",
                g.id
            )));
        }
        variables.insert(g.id.clone(), g.typeid.clone());
    }
    if let Some(ref t) = function.header.typeid {
        variables.insert("_".to_owned(), t.clone());
    }
    for p in &function.header.params {
        if variables.contains_key(&p.id) {
            return Err(SimpleError::new(format!(
                "Function `{}`: Variable name already taken: {}",
                function.header.id, p.id,
            )));
        }
        variables.insert(p.id.clone(), p.typeid.clone());
    }
    for l in &function.locals {
        if variables.contains_key(&l.id) {
            return Err(SimpleError::new(format!(
                "Function `{}`: Local variable name already taken: {}",
                function.header.id, l.id,
            )));
        }
        variables.insert(l.id.clone(), l.typeid.clone());
    }
    Ok(variables)
}

pub fn transform_module_auto_types(mut module: Module) -> SimpleResult<Module> {
    let integer_type = if let Some(m) = module.meta.iter().find(|m| m.id == "auto_integer_type") {
        if let Some(MetaValue::String(n)) = m.args.first() {
            n.to_owned()
        } else {
            "i32".to_owned()
        }
    } else {
        "i32".to_owned()
    };
    let float_type = if let Some(m) = module.meta.iter().find(|m| m.id == "auto_float_type") {
        if let Some(MetaValue::String(n)) = m.args.first() {
            n.to_owned()
        } else {
            "f32".to_owned()
        }
    } else {
        "f32".to_owned()
    };
    let string_type = if let Some(m) = module.meta.iter().find(|m| m.id == "auto_string_type") {
        if let Some(MetaValue::String(n)) = m.args.first() {
            n.to_owned()
        } else {
            "u8".to_owned()
        }
    } else {
        "u8".to_owned()
    };
    module.functions = module
        .functions
        .iter()
        .map(|f| transform_function_auto_types(f.clone(), &integer_type, &float_type, &string_type))
        .collect::<SimpleResult<Vec<_>>>()?;
    Ok(module)
}

pub fn transform_function_auto_types(
    mut function: Function,
    integer_type: &str,
    float_type: &str,
    string_type: &str,
) -> SimpleResult<Function> {
    function.body = function
        .body
        .iter()
        .map(|o| match o.clone() {
            BlockOp::Operation(mut op) => {
                op.params = op
                    .params
                    .iter()
                    .map(|v| {
                        transform_value_auto_types(v.clone(), integer_type, float_type, string_type)
                    })
                    .collect::<SimpleResult<Vec<_>>>()?;
                op.targets = op
                    .targets
                    .iter()
                    .map(|v| {
                        transform_value_auto_types(v.clone(), integer_type, float_type, string_type)
                    })
                    .collect::<SimpleResult<Vec<_>>>()?;
                Ok(BlockOp::Operation(op))
            }
            BlockOp::Label(n) => Ok(BlockOp::Label(n)),
        })
        .collect::<SimpleResult<Vec<_>>>()?;
    Ok(function)
}

pub fn transform_value_auto_types(
    value: Value,
    integer_type: &str,
    float_type: &str,
    string_type: &str,
) -> SimpleResult<Value> {
    match value {
        Value::Ref(v, a) => Ok(Value::Ref(
            Box::new(transform_value_auto_types(
                *v,
                integer_type,
                float_type,
                string_type,
            )?),
            a,
        )),
        Value::Deref(v, a) => Ok(Value::Deref(
            Box::new(transform_value_auto_types(
                *v,
                integer_type,
                float_type,
                string_type,
            )?),
            a,
        )),
        Value::FunctionCall(i, v, a) => Ok(Value::FunctionCall(
            i,
            v.into_iter()
                .map(|v| transform_value_auto_types(v, integer_type, float_type, string_type))
                .collect::<SimpleResult<Vec<_>>>()?,
            a,
        )),
        Value::Tuple(v, a) => Ok(Value::Tuple(
            v.into_iter()
                .map(|v| transform_value_auto_types(v, integer_type, float_type, string_type))
                .collect::<SimpleResult<Vec<_>>>()?,
            a,
        )),
        Value::String(v, t) => {
            if let Type::Pointer(t) = t {
                if let Type::Identifier(i) = *t {
                    Ok(Value::String(
                        v,
                        Type::Pointer(Box::new(Type::Identifier(if i == "{string}" {
                            string_type.to_owned()
                        } else {
                            i
                        }))),
                    ))
                } else {
                    Err(SimpleError::new(format!(
                        "Type is not identifier: {}",
                        t.to_string()
                    )))
                }
            } else {
                Err(SimpleError::new(format!(
                    "Type is not pointer: {}",
                    t.to_string()
                )))
            }
        }
        Value::Number(v) => match v {
            Number::Integer(v, t) => {
                if let Type::Identifier(i) = t {
                    Ok(Value::Number(Number::Integer(
                        v,
                        Type::Identifier(if i == "{integer}" {
                            integer_type.to_owned()
                        } else {
                            i
                        }),
                    )))
                } else {
                    Err(SimpleError::new(format!(
                        "Type is not identifier: {}",
                        t.to_string()
                    )))
                }
            }
            Number::Float(v, t) => {
                if let Type::Identifier(i) = t {
                    Ok(Value::Number(Number::Float(
                        v,
                        Type::Identifier(if i == "{float}" {
                            float_type.to_owned()
                        } else {
                            i
                        }),
                    )))
                } else {
                    Err(SimpleError::new(format!(
                        "Type is not identifier: {}",
                        t.to_string()
                    )))
                }
            }
        },
        Value::OperationInline(i, v, a) => Ok(Value::OperationInline(
            i,
            v.into_iter()
                .map(|v| transform_value_auto_types(v, integer_type, float_type, string_type))
                .collect::<SimpleResult<Vec<_>>>()?,
            a,
        )),
        Value::Variable(i, a) => Ok(Value::Variable(i, a)),
    }
}

pub struct Validator {
    meta: Vec<Meta>,
    rules: Vec<Rule>,
}

impl Validator {
    #[inline]
    pub fn new(ops_descriptor: &OpsDescriptor) -> Self {
        Self::with_filter(ops_descriptor, |_, _| true)
    }

    pub fn with_filter<F>(ops_descriptor: &OpsDescriptor, filter: F) -> Self
    where
        F: Fn(&OpRule, &OpsDescriptor) -> bool,
    {
        Self {
            meta: ops_descriptor.meta.clone(),
            rules: ops_descriptor
                .rules
                .iter()
                .filter_map(|r| {
                    if filter(r, ops_descriptor) {
                        Some(Rule::from_op_rule(r))
                    } else {
                        None
                    }
                })
                .collect(),
        }
    }

    pub fn meta(&self) -> &[Meta] {
        &self.meta
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn filter_program<V>(&self, program: &mut Program)
    where
        V: DeepValidator,
    {
        let mut modules = program
            .modules
            .iter()
            .filter(|m| V::filter_module(m, program, self))
            .cloned()
            .collect();
        for m in &mut modules {
            self.filter_module::<V>(m, program);
        }
        program.modules = modules;
    }

    fn filter_module<V>(&self, module: &mut Module, program: &Program)
    where
        V: DeepValidator,
    {
        let structs = module
            .structs
            .iter()
            .filter(|s| V::filter_struct(s, module, program, self))
            .cloned()
            .collect();
        let mut functions = module
            .functions
            .iter()
            .filter(|f| V::filter_function(f, module, program, self))
            .cloned()
            .collect();
        for f in &mut functions {
            self.filter_function::<V>(f, module, program);
        }
        module.structs = structs;
        module.functions = functions;
    }

    fn filter_function<V>(&self, function: &mut Function, module: &Module, program: &Program)
    where
        V: DeepValidator,
    {
        let body = function
            .body
            .iter()
            .filter(|o| {
                if let BlockOp::Operation(o) = o {
                    V::filter_op(&o, function, module, program, self)
                } else {
                    true
                }
            })
            .cloned()
            .collect();
        function.body = body;
    }

    pub fn validate_program<V>(&self, program: &Program) -> SimpleResult<()>
    where
        V: DeepValidator,
    {
        for module in &program.modules {
            self.ensure_no_duplicates_module(module)?;
            self.validate_module::<V>(module, program)?;
        }
        V::validate_program(program, self)
    }

    pub fn transform_program<V>(&self, program: &mut Program) -> SimpleResult<()>
    where
        V: DeepValidator,
    {
        program.modules = program
            .modules
            .iter()
            .map(|m| V::transform_module(m.clone(), program, self))
            .collect::<SimpleResult<Vec<Module>>>()?;
        Ok(())
    }

    fn ensure_no_duplicates_module(&self, module: &Module) -> SimpleResult<()> {
        let mut ids = HashSet::new();
        for i in &module.imports {
            for name in &i.names {
                if ids.contains(name) {
                    return Err(SimpleError::new(format!(
                        "Import name `{}` is already taken",
                        name
                    )));
                }
                ids.insert(name.clone());
            }
        }
        for g in &module.globals {
            if ids.contains(&g.id) {
                return Err(SimpleError::new(format!(
                    "Global name `{}` is already taken",
                    g.id
                )));
            }
            ids.insert(g.id.clone());
        }
        for e in &module.externs {
            if ids.contains(&e.item.id) {
                return Err(SimpleError::new(format!(
                    "Export name `{}` is already taken",
                    e.item.id
                )));
            }
            ids.insert(e.item.id.clone());
        }
        for s in &module.structs {
            if ids.contains(&s.id) {
                return Err(SimpleError::new(format!(
                    "Struct name `{}` is already taken",
                    s.id
                )));
            }
            ids.insert(s.id.clone());
        }
        for f in &module.functions {
            if ids.contains(&f.header.id) {
                return Err(SimpleError::new(format!(
                    "Function name `{}` is already taken",
                    f.header.id
                )));
            }
            ids.insert(f.header.id.clone());
        }
        Ok(())
    }

    fn validate_module<V>(&self, module: &Module, program: &Program) -> SimpleResult<()>
    where
        V: DeepValidator,
    {
        let types = resolve_module_types(module, program)?;
        let functions = resolve_module_functions(module, program)?;
        for s in &module.structs {
            Self::validate_struct(s, program)?;
        }
        for g in &module.globals {
            Self::validate_type(&g.typeid, &types)?;
        }
        for e in &module.externs {
            Self::validate_function_header(&e.item, &types)?;
        }
        for f in &module.functions {
            let variables = resolve_function_variables(f, module)?;
            self.validate_function::<V>(f, module, program, &types, &functions, &variables)?;
        }
        V::validate_module(module, program, self)
    }

    fn validate_struct(struct_: &Struct, program: &Program) -> SimpleResult<()> {
        let mut stack = vec![];
        Self::validate_struct_inner(struct_, program, &mut stack)
    }

    fn validate_struct_inner(
        struct_: &Struct,
        program: &Program,
        stack: &mut Vec<String>,
    ) -> SimpleResult<()> {
        stack.push(struct_.id.clone());
        for f in &struct_.fields {
            Self::validate_struct_type(&f.typeid, program, stack)?;
        }
        stack.pop();
        Ok(())
    }

    fn validate_struct_type(
        type_: &Type,
        program: &Program,
        stack: &mut Vec<String>,
    ) -> SimpleResult<()> {
        match type_ {
            Type::Identifier(ref t) => {
                if let Some(s) = program.find_struct(t) {
                    if stack.iter().any(|t| t == &s.id) {
                        return Err(SimpleError::new(format!(
                            "Type `{}` is found to be in infinite loop of fields types with chain: {}\nConsider using pointer to that type instead",
                            s.id,
                            stack.join(" => "),
                        )));
                    }
                    Self::validate_struct_inner(s, program, stack)?;
                }
                Ok(())
            }
            Type::Tuple(ref v) => {
                for t in v {
                    Self::validate_struct_type(t, program, stack)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn validate_function<V>(
        &self,
        function: &Function,
        module: &Module,
        program: &Program,
        types: &ModuleTypeIDs,
        functions: &ModuleFunctionIDs,
        variables: &FunctionVariablesIDs,
    ) -> SimpleResult<()>
    where
        V: DeepValidator,
    {
        Self::validate_function_header(&function.header, types)?;
        for v in &function.locals {
            Self::validate_type(&v.typeid, types)?;
        }
        for o in &function.body {
            if let BlockOp::Operation(ref o) = o {
                self.validate_op::<V>(o, function, module, program, types, functions, variables)?;
            }
        }
        Ok(())
    }

    fn validate_function_header(
        header: &FunctionHeader,
        types: &ModuleTypeIDs,
    ) -> SimpleResult<()> {
        for p in &header.params {
            Self::validate_type(&p.typeid, types)?;
        }
        if let Some(ref t) = header.typeid {
            Self::validate_type(t, types)?;
        }
        Ok(())
    }

    fn validate_type(typeid: &Type, types: &ModuleTypeIDs) -> SimpleResult<()> {
        match typeid {
            Type::Identifier(ref t) => {
                if !t.is_empty() && !types.contains_key(t) {
                    Err(SimpleError::new(format!("Found unknown type: `{}`", t)))
                } else {
                    Ok(())
                }
            }
            Type::Pointer(ref t) => Self::validate_type(t, types),
            Type::Tuple(ref tt) => {
                for t in tt {
                    Self::validate_type(t, types)?;
                }
                Ok(())
            }
        }
    }

    fn validate_op<V>(
        &self,
        op: &Operation,
        function: &Function,
        module: &Module,
        program: &Program,
        types: &ModuleTypeIDs,
        functions: &ModuleFunctionIDs,
        variables: &FunctionVariablesIDs,
    ) -> SimpleResult<()>
    where
        V: DeepValidator,
    {
        if let Some(rule) = self.rules.iter().find(|r| r.id == op.id) {
            if rule.params.len() != op.params.len() {
                return Err(SimpleError::new(format!(
                    "Operation `{}` expects {} parameter(s)",
                    op.id,
                    rule.params.len()
                )));
            }
            if rule.targets.len() != op.targets.len() {
                return Err(SimpleError::new(format!(
                    "Operation `{}` expects {} target(s)",
                    op.id,
                    rule.targets.len()
                )));
            }
            for i in 0..rule.params.len() {
                let pr = &rule.params[i];
                let po = &op.params[i];
                match self.find_value_type(po, types, functions, variables, program) {
                    Ok(t) => {
                        if let Err(err) = Self::validate_type(&t, types) {
                            Err(SimpleError::new(format!(
                                "Operation `{}`: {}",
                                op.id, err.message
                            )))
                        } else if t == pr.typeid {
                            Ok(())
                        } else {
                            Err(SimpleError::new(format!(
                                "Operation `{}` parameter `{}` with type `{}` is not type of `{}`",
                                op.id,
                                pr.id,
                                t.to_string(),
                                pr.typeid.to_string(),
                            )))
                        }
                    }
                    Err(err) => Err(SimpleError::new(format!(
                        "Operation `{}`: {}",
                        op.id, err.message
                    ))),
                }?;
            }
            for i in 0..rule.targets.len() {
                let ta = &rule.targets[i];
                let tb = &op.targets[i];
                match self.find_value_type(tb, types, functions, variables, program) {
                    Ok(ref t) => {
                        if let Err(err) = Self::validate_type(&t, types) {
                            Err(SimpleError::new(format!(
                                "Operation `{}`: {}",
                                op.id, err.message
                            )))
                        } else if t == ta {
                            Ok(())
                        } else {
                            Err(SimpleError::new(format!(
                                "Operation `{}` target #`{}` with type `{}` is not type of `{}`",
                                op.id,
                                i,
                                t.to_string(),
                                ta.to_string(),
                            )))
                        }
                    }
                    Err(err) => Err(SimpleError::new(format!(
                        "Operation `{}`: {}",
                        op.id, err.message
                    ))),
                }?;
            }
            if let Err(err) = V::validate_op(op, function, module, program, rule, self) {
                Err(SimpleError::new(format!(
                    "Operation `{}`: {}",
                    op.id, err.message
                )))
            } else {
                Ok(())
            }
        } else {
            Err(SimpleError::new(format!(
                "Operation is not supported: {}",
                op.id
            )))
        }
    }

    fn find_value_type(
        &self,
        value: &Value,
        types: &ModuleTypeIDs,
        functions: &ModuleFunctionIDs,
        variables: &FunctionVariablesIDs,
        program: &Program,
    ) -> SimpleResult<Type> {
        match value {
            Value::Ref(ref v, ref a) => {
                let t = Type::Pointer(Box::new(
                    self.find_value_type(v, types, functions, variables, program)?,
                ));
                if let Some(ref a) = a {
                    self.find_access_value_type(&t, a, types, program)
                } else {
                    Ok(t)
                }
            }
            Value::Deref(ref v, ref a) => {
                let t = self.find_value_type(v, types, functions, variables, program)?;
                if let Type::Pointer(t) = t {
                    if let Some(ref a) = a {
                        self.find_access_value_type(&t, a, types, program)
                    } else {
                        Ok(*t)
                    }
                } else {
                    Err(SimpleError::new(format!(
                        "Trying to dereference non-pointer type: {}",
                        t.to_string()
                    )))
                }
            }
            Value::FunctionCall(ref fc, _, ref a) => {
                self.find_function_call_value_type(fc, a, functions, types, program)
            }
            Value::Tuple(ref t, ref a) => {
                self.find_tuple_value_type(t, a, types, functions, variables, program)
            }
            Value::String(_, ref t) => Ok(t.clone()),
            Value::Number(ref n) => Ok(match n {
                Number::Integer(_, ref t) => t.clone(),
                Number::Float(_, ref t) => t.clone(),
            }),
            Value::OperationInline(ref n, _, ref a) => {
                self.find_operation_inline_value_type(n, a, types, program)
            }
            Value::Variable(ref id, ref a) => {
                if let Some(t) = variables.iter().find(|v| v.0 == id) {
                    if let Some(ref a) = a {
                        self.find_access_value_type(&t.1, a, types, program)
                    } else {
                        Ok(t.1.clone())
                    }
                } else {
                    Err(SimpleError::new(format!(
                        "Could not find variable `{}` in scope",
                        id
                    )))
                }
            }
        }
    }

    fn find_function_call_value_type(
        &self,
        id: &str,
        access: &Option<Box<Access>>,
        functions: &ModuleFunctionIDs,
        types: &ModuleTypeIDs,
        program: &Program,
    ) -> SimpleResult<Type> {
        if let Some(t) = functions.get(id) {
            if let Some(t) = t {
                if let Some(a) = access {
                    self.find_access_value_type(t, a, types, program)
                } else {
                    Ok(t.clone())
                }
            } else if access.is_none() {
                Ok(Type::default())
            } else {
                Err(SimpleError::new("Trying to access empty type".to_owned()))
            }
        } else {
            Err(SimpleError::new(format!(
                "Trying to call unknown function: {:?}",
                id
            )))
        }
    }

    fn find_tuple_value_type(
        &self,
        values: &[Value],
        access: &Option<Box<Access>>,
        types: &ModuleTypeIDs,
        functions: &ModuleFunctionIDs,
        variables: &FunctionVariablesIDs,
        program: &Program,
    ) -> SimpleResult<Type> {
        let result = Type::Tuple(
            values
                .iter()
                .map(|t| self.find_value_type(t, types, functions, variables, program))
                .collect::<Result<Vec<Type>, SimpleError>>()?,
        );
        if let Some(ref a) = access {
            self.find_access_value_type(&result, a, types, program)
        } else {
            Ok(result)
        }
    }

    fn find_access_value_type(
        &self,
        typeid: &Type,
        access: &Access,
        types: &ModuleTypeIDs,
        program: &Program,
    ) -> SimpleResult<Type> {
        match access {
            Access::Tuple(i, a) => match typeid {
                Type::Tuple(ref t) => {
                    let i = *i as usize;
                    if i < t.len() {
                        if let Some(ref a) = a {
                            self.find_access_value_type(&t[i], a, types, program)
                        } else {
                            Ok(t[i].clone())
                        }
                    } else {
                        Err(SimpleError::new(format!(
                            "Tuple does not have field #{}",
                            i
                        )))
                    }
                }
                _ => Err(SimpleError::new(
                    "Only tuples can be accessed by index".to_owned(),
                )),
            },
            Access::Variable(id, a) => match typeid {
                Type::Identifier(ref i) => {
                    if let Some(m) = types.get(i).unwrap() {
                        if let Some(s) = program.find_module_struct(m, i) {
                            if let Some(v) = s.fields.iter().find(|v| &v.id == id) {
                                if let Some(a) = a {
                                    self.find_access_value_type(&v.typeid, a, types, program)
                                } else {
                                    Ok(v.typeid.clone())
                                }
                            } else {
                                Err(SimpleError::new(format!(
                                    "Could not find field `{}` in type: {:?}",
                                    id, s.id
                                )))
                            }
                        } else {
                            Err(SimpleError::new(format!(
                                "Atomic struct {} does not have any fields",
                                i
                            )))
                        }
                    } else {
                        Err(SimpleError::new(format!(
                            "Atomic struct {} does not have any fields",
                            i
                        )))
                    }
                }
                _ => Err(SimpleError::new(
                    "Only structs can be accessed by variable".to_owned(),
                )),
            },
        }
    }

    fn find_operation_inline_value_type(
        &self,
        id: &str,
        access: &Option<Box<Access>>,
        types: &ModuleTypeIDs,
        program: &Program,
    ) -> SimpleResult<Type> {
        if let Some(rule) = self.rules.iter().find(|r| r.id == id) {
            if rule.targets.len() == 1 {
                if let Some(a) = access {
                    self.find_access_value_type(&rule.targets[0], a, types, program)
                } else {
                    Ok(rule.targets[0].clone())
                }
            } else {
                Err(SimpleError::new(format!(
                    "Trying to inline operation of not one target: {}",
                    id
                )))
            }
        } else {
            Err(SimpleError::new(format!(
                "Trying to inline unknown operation: {}",
                id
            )))
        }
    }
}
