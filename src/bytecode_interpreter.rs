use crate::bytecode::*;
use crate::object::*;
use crate::universe::Universe;
use anyhow::{anyhow, Result};
use std::rc::Rc;
use std::cell::RefCell;

pub struct BytecodeInterpreter<'a> {
    pub universe: &'a Universe,
    pub image: Image,
    pub depth: std::cell::Cell<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub method_name: String,
    pub constants: Vec<Constant>,
    pub blocks: Vec<CompiledBlock>,
    pub locals: Vec<Value>,
    pub args: Vec<Value>,
    pub stack: Vec<Value>,
    pub ip: usize,
    pub bytecodes: Vec<Opcode>,
    pub context: Option<SomRef<Frame>>, // for block frames
    pub holder: Option<SomRef<SomClass>>,
    pub self_val: Value,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub enum FrameResult {
    Value(Value),
    NonLocalReturn(Value, SomRef<Frame>),
}

impl<'a> BytecodeInterpreter<'a> {
    pub fn new(universe: &'a Universe, image: Image) -> Self {
        Self { universe, image, depth: std::cell::Cell::new(0) }
    }

    pub fn run(&self, class_name: &str, args: Vec<String>) -> Result<()> {
        let main_class = self.universe.load_class(class_name)?;

        let instance = Rc::new(RefCell::new(SomObject {
            class: main_class.clone(),
            fields: vec![Value::Nil; main_class.borrow().instance_fields.len()],
        }));

        let som_args: Vec<Value> = args.iter()
            .map(|s| Value::new_string(s.clone()))
            .collect();
        let args_array = Value::Array(Rc::new(RefCell::new(som_args)));

        if main_class.borrow().methods.contains_key("run:") {
            self.dispatch(Value::Object(instance), "run:", vec![args_array])?;
        } else {
            self.dispatch(Value::Object(instance), "run", Vec::new())?;
        }

        Ok(())
    }

    fn dispatch(&self, receiver: Value, selector: &str, args: Vec<Value>) -> Result<Value> {
        let res = self.dispatch_internal(receiver, selector, args)?;
        match res {
            FrameResult::Value(v) => Ok(v),
            FrameResult::NonLocalReturn(_, _) => Err(anyhow!("NonLocalReturn escaped the top level")),
        }
    }

    fn dispatch_internal(&self, receiver: Value, selector: &str, args: Vec<Value>) -> Result<FrameResult> {
        let depth = self.depth.get() + 1;
        self.depth.set(depth);
        if depth > 2000 {
            return Err(anyhow!("Recursion limit exceeded"));
        }

        if let Value::CompiledBlock(cb) = &receiver {
            if selector.starts_with("value") {
                let block = cb.borrow().block.clone();
                let frame_args = args.clone();

                let frame = Rc::new(RefCell::new(Frame {
                    method_name: "block_invoke".to_string(),
                    constants: block.constants,
                    blocks: block.blocks,
                    locals: vec![Value::Nil; block.num_locals],
                    args: frame_args,
                    stack: Vec::new(),
                    ip: 0,
                    bytecodes: block.bytecodes,
                    context: cb.borrow().context.clone(),
                    holder: cb.borrow().context.as_ref().and_then(|c| c.borrow().holder.clone()),
                    self_val: cb.borrow().context.as_ref().map(|c| c.borrow().self_val.clone()).unwrap_or(Value::Nil),
                    is_active: true,
                }));

                let res = self.execute_frame(frame);
                self.depth.set(depth - 1);
                return res;
            }
        }

        let cls = self.get_class(&receiver)?;

        if selector == "asInteger" || selector == "round" {
            if let Value::Integer(_) = &receiver {
                self.depth.set(depth - 1);
                return Ok(FrameResult::Value(receiver.clone()));
            }
        }

        let method = self.lookup_compiled_method(cls, selector)?;
        let res = self.execute_method(method, receiver, args);

        self.depth.set(depth - 1);
        res
    }

    fn get_class(&self, val: &Value) -> Result<SomRef<SomClass>> {
        match val {
            Value::Integer(_) => self.universe.load_class("Integer"),
            Value::String(_) => self.universe.load_class("String"),
            Value::Boolean(true) => self.universe.load_class("True"),
            Value::Boolean(false) => self.universe.load_class("False"),
            Value::Nil => self.universe.load_class("Nil"),
            Value::Double(_) => self.universe.load_class("Double"),
            Value::Object(obj) => Ok(obj.borrow().class.clone()),
            Value::Array(_) => self.universe.load_class("Array"),
            Value::Class(cls) => Ok(cls.borrow().class.as_ref().unwrap().clone()),
            Value::Block(_) => self.universe.load_class("Block"),
            Value::CompiledBlock(_) => self.universe.load_class("Block"),
            Value::Symbol(_) => self.universe.load_class("Symbol"),
            Value::Method(_) => self.universe.load_class("Method"),
        }
    }

    fn lookup_compiled_method(&self, cls: SomRef<SomClass>, selector: &str) -> Result<CompiledMethod> {
        let mut curr_cls_name = cls.borrow().name.clone();

        loop {
            if let Some(compiled_cls) = self.image.classes.get(&curr_cls_name) {
                if let Some(m) = compiled_cls.instance_methods.get(selector) {
                    return Ok(m.clone());
                }
                if let Some(m) = compiled_cls.class_methods.get(selector) {
                    return Ok(m.clone());
                }
            }

            let next = {
                let cls_borrow = self.universe.load_class(&curr_cls_name)?;
                cls_borrow.borrow().super_class.clone()
            };

            if let Some(sup) = next {
                curr_cls_name = sup.borrow().name.clone();
            } else {
                return Err(anyhow!("Method {} not found in image for {}", selector, cls.borrow().name));
            }
        }
    }

    fn execute_method(&self, method: CompiledMethod, receiver: Value, args: Vec<Value>) -> Result<FrameResult> {
        if method.is_primitive {
            let cls = self.get_class(&receiver)?;
            let key = format!("{}>>{}", cls.borrow().name, method.name);
            if let Some(f) = self.universe.primitives.get(&key) {
                let ast_interp = crate::interpreter::Interpreter::new(self.universe);
                ast_interp.depth.set(self.depth.get());
                match f(&receiver, args, self.universe, &ast_interp) {
                    Ok(crate::interpreter::ReturnValue::Value(v)) => return Ok(FrameResult::Value(v)),
                    Ok(crate::interpreter::ReturnValue::NonLocalReturn(_, _)) => return Err(anyhow!("NLR from primitive not supported")),
                    Ok(crate::interpreter::ReturnValue::Restart) => return Err(anyhow!("Restart from primitive not supported")),
                    Err(e) => return Err(e),
                }
            } else {
                return Err(anyhow!("Primitive {} not found", key));
            }
        }

        let frame = Rc::new(RefCell::new(Frame {
            method_name: method.name,
            constants: method.constants,
            blocks: method.blocks,
            locals: vec![Value::Nil; method.num_locals],
            args,
            stack: Vec::new(),
            ip: 0,
            bytecodes: method.bytecodes,
            context: None,
            holder: Some(self.get_class(&receiver)?),
            self_val: receiver,
            is_active: true,
        }));

        self.execute_frame(frame)
    }

    fn execute_frame(&self, frame: SomRef<Frame>) -> Result<FrameResult> {
        loop {
            let opcode = {
                let f = frame.borrow();
                if f.ip >= f.bytecodes.len() {
                    break;
                }
                f.bytecodes[f.ip].clone()
            };

            frame.borrow_mut().ip += 1;

            match opcode {
                Opcode::Halt => break,
                Opcode::Dup => {
                    let mut f = frame.borrow_mut();
                    let val = f.stack.last().unwrap().clone();
                    f.stack.push(val);
                }
                Opcode::PushLocal(level, idx) => {
                    let val = self.get_local(&frame, level, idx)?;
                    frame.borrow_mut().stack.push(val);
                }
                Opcode::PushArgument(level, idx) => {
                    let val = self.get_arg(&frame, level, idx)?;
                    frame.borrow_mut().stack.push(val);
                }
                Opcode::PushField(sym_idx) => {
                    let field_name = self.get_symbol(&frame, sym_idx)?;
                    let self_val = self.get_self(&frame, 0)?;
                    let val = self.get_field(self_val, &field_name)?;
                    frame.borrow_mut().stack.push(val);
                }
                Opcode::PushConstant(idx) => {
                    let constant = { frame.borrow().constants[idx as usize].clone() };
                    let val = self.eval_constant(constant)?;
                    frame.borrow_mut().stack.push(val);
                }
                Opcode::PushGlobal(sym_idx) => {
                    let global_name = self.get_symbol(&frame, sym_idx)?;
                    if let Some(val) = self.universe.get_global(&global_name) {
                        frame.borrow_mut().stack.push(val);
                    } else {
                        let self_val = self.get_self(&frame, 0)?;
                        let sym = Value::Symbol(global_name);
                        match self.dispatch_internal(self_val, "unknownGlobal:", vec![sym])? {
                            FrameResult::Value(v) => frame.borrow_mut().stack.push(v),
                            FrameResult::NonLocalReturn(v, target) => return Ok(FrameResult::NonLocalReturn(v, target)),
                        }
                    }
                }
                Opcode::PushSelf(level) => {
                    let self_val = self.get_self(&frame, level)?;
                    frame.borrow_mut().stack.push(self_val);
                }
                Opcode::PushNil => frame.borrow_mut().stack.push(Value::Nil),
                Opcode::PushTrue => frame.borrow_mut().stack.push(Value::Boolean(true)),
                Opcode::PushFalse => frame.borrow_mut().stack.push(Value::Boolean(false)),
                Opcode::Pop => {
                    frame.borrow_mut().stack.pop();
                }
                Opcode::PopLocal(level, idx) => {
                    let val = frame.borrow_mut().stack.pop().unwrap();
                    self.set_local(&frame, level, idx, val)?;
                }
                Opcode::PopArgument(level, idx) => {
                    let val = frame.borrow_mut().stack.pop().unwrap();
                    self.set_arg(&frame, level, idx, val)?;
                }
                Opcode::PopField(sym_idx) => {
                    let field_name = self.get_symbol(&frame, sym_idx)?;
                    let val = frame.borrow_mut().stack.pop().unwrap();
                    let self_val = self.get_self(&frame, 0)?;
                    self.set_field(self_val, &field_name, val)?;
                }
                Opcode::Send(sym_idx) => {
                    let selector = self.get_symbol(&frame, sym_idx)?;
                    let num_args = selector.chars().filter(|c| *c == ':').count();
                    let mut args = Vec::new();
                    for _ in 0..num_args {
                        args.push(frame.borrow_mut().stack.pop().unwrap());
                    }
                    args.reverse();
                    let receiver = frame.borrow_mut().stack.pop().unwrap();
                    match self.dispatch_internal(receiver, &selector, args)? {
                        FrameResult::Value(v) => frame.borrow_mut().stack.push(v),
                        FrameResult::NonLocalReturn(v, target) => return Ok(FrameResult::NonLocalReturn(v, target)),
                    }
                }
                Opcode::SuperSend(sym_idx) => {
                    let selector = self.get_symbol(&frame, sym_idx)?;
                    let num_args = selector.chars().filter(|c| *c == ':').count();
                    let mut args = Vec::new();
                    for _ in 0..num_args {
                        args.push(frame.borrow_mut().stack.pop().unwrap());
                    }
                    args.reverse();
                    let receiver = frame.borrow_mut().stack.pop().unwrap();

                    let holder = frame.borrow().holder.as_ref().unwrap().clone();
                    let super_class = holder.borrow().super_class.as_ref().unwrap().clone();

                    let method = self.lookup_compiled_method(super_class, &selector)?;
                    match self.execute_method(method, receiver, args)? {
                        FrameResult::Value(v) => frame.borrow_mut().stack.push(v),
                        FrameResult::NonLocalReturn(v, target) => return Ok(FrameResult::NonLocalReturn(v, target)),
                    }
                }
                Opcode::ReturnLocal => {
                    let val = frame.borrow_mut().stack.pop().unwrap_or(Value::Nil);
                    frame.borrow_mut().is_active = false;
                    return Ok(FrameResult::Value(val));
                }
                Opcode::ReturnNonLocal => {
                    let val = frame.borrow_mut().stack.pop().unwrap_or(Value::Nil);
                    frame.borrow_mut().is_active = false;

                    let mut curr = frame.clone();
                    loop {
                        let is_method = curr.borrow().holder.is_some();
                        if is_method {
                            return Ok(FrameResult::NonLocalReturn(val, curr));
                        }
                        let next = curr.borrow().context.clone();
                        if let Some(ctx) = next {
                            curr = ctx;
                        } else {
                            return Err(anyhow!("Cannot return non-locally outside a method context"));
                        }
                    }
                }
                Opcode::PushBlock(idx) => {
                    let block = frame.borrow().blocks[idx as usize].clone();
                    let block_inst = CompiledBlockInstance { block, context: Some(frame.clone()) };
                    let val = Value::CompiledBlock(Rc::new(RefCell::new(block_inst)));
                    frame.borrow_mut().stack.push(val);
                }
            }
        }

        let val = frame.borrow_mut().stack.pop().unwrap_or(Value::Nil);
        frame.borrow_mut().is_active = false;
        Ok(FrameResult::Value(val))
    }

    fn eval_constant(&self, constant: Constant) -> Result<Value> {
        match constant {
            Constant::Integer(s) => Ok(Value::Integer(s.parse()?)),
            Constant::Double(d) => Ok(Value::Double(d)),
            Constant::String(s) => Ok(Value::new_string(s)),
            Constant::Symbol(s) => Ok(Value::Symbol(s)),
            Constant::Array(arr) => {
                let mut vals = Vec::new();
                for c in arr {
                    vals.push(self.eval_constant(c)?);
                }
                Ok(Value::Array(Rc::new(RefCell::new(vals))))
            }
        }
    }

    fn get_symbol(&self, frame: &SomRef<Frame>, idx: u16) -> Result<String> {
        if let Constant::Symbol(sym) = &frame.borrow().constants[idx as usize] {
            Ok(sym.clone())
        } else {
            Err(anyhow!("Expected symbol constant"))
        }
    }

    fn get_self(&self, frame: &SomRef<Frame>, level: u8) -> Result<Value> {
        let mut curr = frame.clone();
        for _ in 0..level {
            let next = curr.borrow().context.clone();
            curr = next.ok_or_else(|| anyhow!("Context not found"))?;
        }
        Ok(curr.borrow().self_val.clone())
    }

    fn get_local(&self, frame: &SomRef<Frame>, level: u8, idx: u8) -> Result<Value> {
        let mut curr = frame.clone();
        for _ in 0..level {
            let next = curr.borrow().context.clone();
            curr = next.ok_or_else(|| anyhow!("Context not found"))?;
        }
        Ok(curr.borrow().locals[idx as usize].clone())
    }

    fn set_local(&self, frame: &SomRef<Frame>, level: u8, idx: u8, val: Value) -> Result<()> {
        let mut curr = frame.clone();
        for _ in 0..level {
            let next = curr.borrow().context.clone();
            curr = next.ok_or_else(|| anyhow!("Context not found"))?;
        }
        curr.borrow_mut().locals[idx as usize] = val;
        Ok(())
    }

    fn get_arg(&self, frame: &SomRef<Frame>, level: u8, idx: u8) -> Result<Value> {
        let mut curr = frame.clone();
        for _ in 0..level {
            let next = curr.borrow().context.clone();
            curr = next.ok_or_else(|| anyhow!("Context not found"))?;
        }
        Ok(curr.borrow().args[idx as usize].clone())
    }

    fn set_arg(&self, frame: &SomRef<Frame>, level: u8, idx: u8, val: Value) -> Result<()> {
        let mut curr = frame.clone();
        for _ in 0..level {
            let next = curr.borrow().context.clone();
            curr = next.ok_or_else(|| anyhow!("Context not found"))?;
        }
        curr.borrow_mut().args[idx as usize] = val;
        Ok(())
    }

    fn get_field(&self, self_val: Value, name: &str) -> Result<Value> {
        match self_val {
            Value::Object(ref obj) => {
                let cls = obj.borrow().class.clone();
                let cls_borrow = cls.borrow();
                if let Some(idx) = cls_borrow.instance_fields.iter().position(|f| f == name) {
                    return Ok(obj.borrow().fields[idx].clone());
                }
            }
            Value::Class(ref cls) => {
                let mc = cls.borrow().class.as_ref().unwrap().clone();
                let mc_borrow = mc.borrow();
                if let Some(idx) = mc_borrow.instance_fields.iter().position(|f| f == name) {
                    return Ok(cls.borrow().fields[idx].clone());
                }
            }
            _ => {}
        }
        Err(anyhow!("Field {} not found on {:?}", name, self_val))
    }

    fn set_field(&self, self_val: Value, name: &str, val: Value) -> Result<()> {
        match self_val {
            Value::Object(ref obj) => {
                let cls = obj.borrow().class.clone();
                let cls_borrow = cls.borrow();
                if let Some(idx) = cls_borrow.instance_fields.iter().position(|f| f == name) {
                    obj.borrow_mut().fields[idx] = val;
                    return Ok(());
                }
            }
            Value::Class(ref cls) => {
                let mc = cls.borrow().class.as_ref().unwrap().clone();
                let mc_borrow = mc.borrow();
                if let Some(idx) = mc_borrow.instance_fields.iter().position(|f| f == name) {
                    cls.borrow_mut().fields[idx] = val;
                    return Ok(());
                }
            }
            _ => {}
        }
        Err(anyhow!("Field {} not found on {:?}", name, self_val))
    }
}
