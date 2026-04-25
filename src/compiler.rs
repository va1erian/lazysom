use crate::ast::*;
use crate::bytecode::*;
use std::collections::HashMap;

pub struct Compiler {
}

#[derive(Clone)]
struct BlockContext {
    arguments: Vec<String>,
    locals: Vec<String>,
}

struct MethodCompiler {
    constants: Vec<Constant>,
    blocks: Vec<CompiledBlock>,
    bytecodes: Vec<Opcode>,
    contexts: Vec<BlockContext>,
    instance_fields: Vec<String>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {}
    }

    pub fn compile_class(&mut self, class_def: &ClassDef, universe: &crate::universe::Universe) -> CompiledClass {
        let mut instance_methods = HashMap::new();
        let mut class_methods = HashMap::new();

        let mut all_instance_fields = Vec::new();
        let mut all_class_fields = Vec::new();

        if let Ok(cls) = universe.load_class(&class_def.name) {
            all_instance_fields = cls.borrow().instance_fields.clone();
            if let Some(metaclass) = &cls.borrow().class {
                all_class_fields = metaclass.borrow().instance_fields.clone();
            }
        }

        for method_def in &class_def.instance_methods {
            let compiled = self.compile_method(method_def, &all_instance_fields);
            instance_methods.insert(compiled.signature.clone(), compiled);
        }

        for method_def in &class_def.class_methods {
            let compiled = self.compile_method(method_def, &all_class_fields);
            class_methods.insert(compiled.signature.clone(), compiled);
        }

        CompiledClass {
            name: class_def.name.clone(),
            super_class: class_def.super_class.clone(),
            instance_fields: all_instance_fields,
            class_fields: all_class_fields,
            instance_methods,
            class_methods,
        }
    }

    fn compile_method(&mut self, method_def: &MethodDef, fields: &[String]) -> CompiledMethod {
        let signature = method_def.signature.selector();
        let mut args = Vec::new();
        match &method_def.signature {
            Signature::Unary(_) => {},
            Signature::Binary(_, arg) => args.push(arg.clone()),
            Signature::Keyword(parts) => {
                for (_, arg) in parts {
                    args.push(arg.clone());
                }
            }
        }

        let is_primitive = method_def.body == MethodBody::Primitive;

        let mut mc = MethodCompiler {
            constants: Vec::new(),
            blocks: Vec::new(),
            bytecodes: Vec::new(),
            contexts: vec![BlockContext {
                arguments: args.clone(),
                locals: match &method_def.body {
                    MethodBody::Block(b) => b.locals.clone(),
                    _ => vec![],
                }
            }],
            instance_fields: fields.to_vec(),
        };

        if !is_primitive {
            if let MethodBody::Block(block) = &method_def.body {
                for (i, expr) in block.body.iter().enumerate() {
                    mc.compile_expression(expr);
                    if i < block.body.len() - 1 {
                        mc.bytecodes.push(Opcode::Pop);
                    }
                }
                if block.body.is_empty() {
                    mc.bytecodes.push(Opcode::PushSelf(0));
                }
                mc.bytecodes.push(Opcode::ReturnLocal);
            }
        } else {
            mc.bytecodes.push(Opcode::PushSelf(0));
            mc.bytecodes.push(Opcode::ReturnLocal);
        }

        CompiledMethod {
            name: signature.clone(),
            is_primitive,
            bytecodes: mc.bytecodes,
            constants: mc.constants,
            blocks: mc.blocks,
            num_locals: mc.contexts[0].locals.len(),
            num_args: mc.contexts[0].arguments.len(),
            signature,
        }
    }
}

impl MethodCompiler {
    fn add_constant(&mut self, constant: Constant) -> u16 {
        if let Some(idx) = self.constants.iter().position(|c| c == &constant) {
            return idx as u16;
        }
        let idx = self.constants.len() as u16;
        self.constants.push(constant);
        idx
    }

    fn find_variable(&mut self, name: &str) -> Opcode {
        if name == "self" || name == "super" {
            let level = self.contexts.len() as u8 - 1;
            return Opcode::PushSelf(level);
        }
        if name == "nil" { return Opcode::PushNil; }
        if name == "true" { return Opcode::PushTrue; }
        if name == "false" { return Opcode::PushFalse; }

        for (level_idx, ctx) in self.contexts.iter().rev().enumerate() {
            let level = level_idx as u8;
            if let Some(idx) = ctx.locals.iter().position(|l| l == name) {
                return Opcode::PushLocal(level, idx as u8);
            }
            if let Some(idx) = ctx.arguments.iter().position(|a| a == name) {
                return Opcode::PushArgument(level, idx as u8);
            }
        }

        if self.instance_fields.contains(&name.to_string()) {
            let sym_idx = self.add_constant(Constant::Symbol(name.to_string()));
            return Opcode::PushField(sym_idx);
        }

        let sym_idx = self.add_constant(Constant::Symbol(name.to_string()));
        Opcode::PushGlobal(sym_idx)
    }

    fn compile_assignment(&mut self, name: &str) -> Opcode {
        for (level_idx, ctx) in self.contexts.iter().rev().enumerate() {
            let level = level_idx as u8;
            if let Some(idx) = ctx.locals.iter().position(|l| l == name) {
                return Opcode::PopLocal(level, idx as u8);
            }
            if let Some(idx) = ctx.arguments.iter().position(|a| a == name) {
                return Opcode::PopArgument(level, idx as u8);
            }
        }

        if self.instance_fields.contains(&name.to_string()) {
            let sym_idx = self.add_constant(Constant::Symbol(name.to_string()));
            return Opcode::PopField(sym_idx);
        }

        panic!("Cannot assign to unknown variable or global: {}", name);
    }

    fn compile_literal(&mut self, lit: &Literal) {
        match lit {
            Literal::Integer(i) => {
                let idx = self.add_constant(Constant::Integer(i.to_string()));
                self.bytecodes.push(Opcode::PushConstant(idx));
            }
            Literal::Double(d) => {
                let idx = self.add_constant(Constant::Double(*d));
                self.bytecodes.push(Opcode::PushConstant(idx));
            }
            Literal::String(s) => {
                let idx = self.add_constant(Constant::String(s.clone()));
                self.bytecodes.push(Opcode::PushConstant(idx));
            }
            Literal::Symbol(s) => {
                let idx = self.add_constant(Constant::Symbol(s.clone()));
                self.bytecodes.push(Opcode::PushConstant(idx));
            }
            Literal::Array(arr) => {
                let mut const_arr = Vec::new();
                for a in arr {
                    let c = match a {
                        Literal::Integer(i) => Constant::Integer(i.to_string()),
                        Literal::Double(d) => Constant::Double(*d),
                        Literal::String(s) => Constant::String(s.clone()),
                        Literal::Symbol(s) => Constant::Symbol(s.clone()),
                        Literal::Array(_) => panic!("Nested array literals not supported in bytecode yet"),
                    };
                    const_arr.push(c);
                }
                let idx = self.add_constant(Constant::Array(const_arr));
                self.bytecodes.push(Opcode::PushConstant(idx));
            }
        }
    }

    fn compile_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Literal(lit) => self.compile_literal(lit),
            Expression::Variable(name) => {
                let op = self.find_variable(name);
                self.bytecodes.push(op);
            }
            Expression::Assignment(name, val_expr) => {
                self.compile_expression(val_expr);
                self.bytecodes.push(Opcode::Dup);
                let op = self.compile_assignment(name);
                self.bytecodes.push(op);
            }
            Expression::Message(receiver, msg) => {
                let is_super = if let Expression::Variable(name) = &**receiver {
                    name == "super"
                } else {
                    false
                };

                if is_super {
                    let level = self.contexts.len() as u8 - 1;
                    self.bytecodes.push(Opcode::PushSelf(level));
                } else {
                    self.compile_expression(receiver);
                }

                match msg {
                    Message::Unary(_) => {},
                    Message::Binary(_, arg) => {
                        self.compile_expression(arg);
                    },
                    Message::Keyword(parts) => {
                        for (_, arg) in parts {
                            self.compile_expression(arg);
                        }
                    }
                }

                let selector = msg.selector();
                let sym_idx = self.add_constant(Constant::Symbol(selector));

                if is_super {
                    self.bytecodes.push(Opcode::SuperSend(sym_idx));
                } else {
                    self.bytecodes.push(Opcode::Send(sym_idx));
                }
            }
            Expression::Block(block) => {
                let compiled_block = self.compile_block(block);
                let block_idx = self.blocks.len() as u16;
                self.blocks.push(compiled_block);
                self.bytecodes.push(Opcode::PushBlock(block_idx));
            }
            Expression::Return(expr) => {
                self.compile_expression(expr);
                self.bytecodes.push(Opcode::ReturnNonLocal);
            }
        }
    }

    fn compile_block(&mut self, block: &Block) -> CompiledBlock {
        let mut child_mc = MethodCompiler {
            constants: Vec::new(),
            blocks: Vec::new(),
            bytecodes: Vec::new(),
            contexts: self.contexts.clone(),
            instance_fields: self.instance_fields.clone(),
        };

        child_mc.contexts.push(BlockContext {
            arguments: block.parameters.clone(),
            locals: block.locals.clone(),
        });

        for (i, expr) in block.body.iter().enumerate() {
            child_mc.compile_expression(expr);
            if i < block.body.len() - 1 {
                child_mc.bytecodes.push(Opcode::Pop);
            }
        }

        if block.body.is_empty() {
            child_mc.bytecodes.push(Opcode::PushNil);
        }
        child_mc.bytecodes.push(Opcode::ReturnLocal);

        CompiledBlock {
            bytecodes: child_mc.bytecodes,
            constants: child_mc.constants,
            blocks: child_mc.blocks,
            num_locals: block.locals.len(),
            num_args: block.parameters.len(),
        }
    }
}

pub fn compile_image(universe: &crate::universe::Universe, initial_classes: &[&str]) -> anyhow::Result<Image> {
    let mut compiler = Compiler::new();
    let mut image = Image {
        classes: HashMap::new(),
    };

    for cls in initial_classes {
        universe.load_class(cls)?;
    }

    for path in &universe.classpath {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if file_path.extension().and_then(|s| s.to_str()) == Some("som") {
                    let name = file_path.file_stem().unwrap().to_str().unwrap();
                    let _ = universe.load_class(name);
                }
            }
        }
    }

    let globals = universe.globals.borrow();
    let mut class_names = Vec::new();
    for (name, val) in globals.iter() {
        if let crate::object::Value::Class(_) = val {
            class_names.push(name.clone());
        }
    }
    drop(globals);

    for name in class_names {
        for path in &universe.classpath {
            let file_path = path.join(format!("{}.som", name));
            if file_path.exists() {
                let content = std::fs::read_to_string(file_path)?;
                let mut parser = crate::parser::Parser::new(&content);
                if let Ok(class_def) = parser.parse_class() {
                    let compiled_class = compiler.compile_class(&class_def, universe);
                    image.classes.insert(name.clone(), compiled_class);
                }
                break;
            }
        }
    }

    Ok(image)
}
