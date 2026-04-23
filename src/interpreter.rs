use crate::object::*;
use crate::ast::*;
use crate::universe::Universe;
use anyhow::{Result, anyhow};
use std::rc::Rc;
use std::cell::RefCell;

pub struct Interpreter<'a> {
    pub universe: &'a Universe,
}

#[derive(Debug, Clone)]
pub enum ReturnValue {
    Value(Value),
    NonLocalReturn(Value, SomRef<Activation>),
}

impl<'a> Interpreter<'a> {
    pub fn new(universe: &'a Universe) -> Self {
        Self { universe }
    }

    pub fn run_method_internal(&self, method: SomRef<SomMethod>, self_val: Value, args: Vec<Value>) -> Result<ReturnValue> {
        match &method.borrow().body {
            crate::object::MethodBody::Primitive(f) => f(&self_val, args, self.universe, self),
            crate::object::MethodBody::Ast(block) => {
                let mut arg_map = std::collections::HashMap::new();
                for (i, name) in method.borrow().parameters.iter().enumerate() {
                    if let Some(val) = args.get(i) {
                        arg_map.insert(name.clone(), val.clone());
                    }
                }
                let mut locals = std::collections::HashMap::new();
                for local_name in &block.locals {
                    locals.insert(local_name.clone(), Value::Nil);
                }

                let activation = Rc::new(RefCell::new(Activation {
                    holder: Some(method.borrow().holder.clone()),
                    self_val: self_val.clone(),
                    args: arg_map,
                    locals,
                    parent: None,
                }));
                
                match self.evaluate_block(block, activation.clone()) {
                    Ok(ReturnValue::NonLocalReturn(v, target)) => {
                        if Rc::ptr_eq(&target, &activation) {
                            Ok(ReturnValue::Value(v))
                        } else {
                            Ok(ReturnValue::NonLocalReturn(v, target))
                        }
                    }
                    Ok(ReturnValue::Value(_)) => Ok(ReturnValue::Value(self_val)),
                    res => res,
                }
            }
        }
    }

    pub fn evaluate_block(&self, block: &Block, activation: SomRef<Activation>) -> Result<ReturnValue> {
        let mut last_val = Value::Nil;
        for expr in &block.body {
            match self.evaluate_expression(expr, activation.clone())? {
                ReturnValue::Value(v) => last_val = v,
                ReturnValue::NonLocalReturn(v, target) => return Ok(ReturnValue::NonLocalReturn(v, target)),
            }
        }
        Ok(ReturnValue::Value(last_val))
    }

    pub fn evaluate_expression(&self, expr: &Expression, activation: SomRef<Activation>) -> Result<ReturnValue> {
        thread_local! {
            static DEPTH: std::cell::Cell<usize> = std::cell::Cell::new(0);
        }
        let depth = DEPTH.with(|d| {
            let next = d.get() + 1;
            d.set(next);
            next
        });
        if depth > 2000 {
            return Err(anyhow!("Recursion limit exceeded"));
        }
        
        let res = self.evaluate_expression_internal(expr, activation);
        
        DEPTH.with(|d| d.set(d.get() - 1));
        res
    }

    fn evaluate_expression_internal(&self, expr: &Expression, activation: SomRef<Activation>) -> Result<ReturnValue> {
        match expr {
            Expression::Literal(lit) => Ok(ReturnValue::Value(self.evaluate_literal(lit))),
            Expression::Variable(name) => {
                if name == "super" {
                    return Err(anyhow!("super can only be used as a message receiver"));
                }
                Ok(ReturnValue::Value(self.lookup(name, activation)?))
            },
            Expression::Assignment(name, val_expr) => {
                match self.evaluate_expression(val_expr, activation.clone())? {
                    ReturnValue::Value(val) => Ok(ReturnValue::Value(self.assign(name, val, activation)?)),
                    ret => Ok(ret),
                }
            }
            Expression::Message(receiver_expr, msg) => {
                let is_super = if let Expression::Variable(name) = &**receiver_expr {
                    name == "super"
                } else {
                    false
                };

                let receiver = if is_super {
                    self.lookup("self", activation.clone())?
                } else {
                    match self.evaluate_expression(receiver_expr, activation.clone())? {
                        ReturnValue::Value(v) => v,
                        ret => return Ok(ret),
                    }
                };

                let mut args = Vec::new();
                match msg {
                    Message::Unary(_) => {},
                    Message::Binary(_, arg) => {
                        match self.evaluate_expression(arg, activation.clone())? {
                            ReturnValue::Value(v) => args.push(v),
                            ret => return Ok(ret),
                        }
                    },
                    Message::Keyword(parts) => {
                        for (_, arg) in parts {
                            match self.evaluate_expression(arg, activation.clone())? {
                                ReturnValue::Value(v) => args.push(v),
                                ret => return Ok(ret),
                            }
                        }
                    }
                }

                if is_super {
                    let method_act = self.find_method_activation(activation)?;
                    let holder = method_act.borrow().holder.as_ref().unwrap().clone();
                    let super_class = holder.borrow().super_class.as_ref().ok_or_else(|| anyhow!("Object has no superclass"))?.clone();
                    let method = self.lookup_method(super_class, &msg.selector())?;
                    self.run_method_internal(method, receiver, args)
                } else {
                    self.dispatch_internal(receiver, &msg.selector(), args)
                }
            }
            Expression::Block(b) => {
                Ok(ReturnValue::Value(Value::Block(Rc::new(RefCell::new(SomBlock {
                    body: b.clone(),
                    context: Some(activation),
                })))))
            }
            Expression::Return(expr) => {
                let val = match self.evaluate_expression(expr, activation.clone())? {
                    ReturnValue::Value(v) => v,
                    ret => return Ok(ret),
                };
                
                let method_act = self.find_method_activation(activation)?;
                Ok(ReturnValue::NonLocalReturn(val, method_act))
            }
        }
    }

    fn find_method_activation(&self, activation: SomRef<Activation>) -> Result<SomRef<Activation>> {
        let mut curr = activation;
        loop {
            if curr.borrow().holder.is_some() { return Ok(curr); }
            let next = curr.borrow().parent.clone();
            if let Some(p) = next { curr = p; }
            else { return Err(anyhow!("Context escape: no method activation found")); }
        }
    }

    pub fn run_block(&self, block: SomRef<SomBlock>, args: Vec<Value>) -> Result<ReturnValue> {
        let block_ref = block.borrow();
        let mut arg_map = std::collections::HashMap::new();
        for (i, name) in block_ref.body.parameters.iter().enumerate() {
            if let Some(arg) = args.get(i) {
                arg_map.insert(name.clone(), arg.clone());
            }
        }
        let mut locals = std::collections::HashMap::new();
        for local_name in &block_ref.body.locals {
            locals.insert(local_name.clone(), Value::Nil);
        }

        let activation = Rc::new(RefCell::new(Activation {
            holder: None,
            self_val: block_ref.context.as_ref().map(|c| c.borrow().self_val.clone()).unwrap_or(Value::Nil),
            args: arg_map,
            locals,
            parent: block_ref.context.clone(),
        }));

        drop(block_ref);
        self.evaluate_block(&block.borrow().body, activation)
    }

    fn evaluate_literal(&self, lit: &Literal) -> Value {
        match lit {
            Literal::Integer(i) => Value::Integer(i.clone()),
            Literal::Double(d) => Value::Double(*d),
            Literal::String(s) => Value::String(s.clone()),
            Literal::Symbol(s) => Value::Symbol(s.clone()),
            Literal::Array(arr) => {
                let mut vals = Vec::new();
                for lit in arr {
                    vals.push(self.evaluate_literal(lit));
                }
                Value::Array(Rc::new(RefCell::new(vals)))
            }
        }
    }

    fn lookup(&self, name: &str, activation: SomRef<Activation>) -> Result<Value> {
        let mut current_act = Some(activation);
        while let Some(act) = current_act {
            let act_ref = act.borrow();
            if name == "self" { return Ok(act_ref.self_val.clone()); }
            if name == "nil" { return Ok(Value::Nil); }
            if name == "true" { return Ok(Value::Boolean(true)); }
            if name == "false" { return Ok(Value::Boolean(false)); }

            if let Some(val) = act_ref.locals.get(name) {
                return Ok(val.clone());
            }
            if let Some(val) = act_ref.args.get(name) {
                return Ok(val.clone());
            }

            if act_ref.holder.is_some() {
                if let Value::Object(obj) = &act_ref.self_val {
                    let cls = obj.borrow().class.clone();
                    let cls_borrow = cls.borrow();
                    if let Some(idx) = cls_borrow.instance_fields.iter().position(|f| f == name) {
                        return Ok(obj.borrow().fields[idx].clone());
                    }
                }
            }

            current_act = act_ref.parent.clone();
        }

        if let Some(val) = self.universe.get_global(name) {
            return Ok(val);
        }

        Err(anyhow!("Unknown variable: {}", name))
    }

    fn assign(&self, name: &str, val: Value, activation: SomRef<Activation>) -> Result<Value> {
        let mut current_act = Some(activation);
        while let Some(act) = current_act {
            let found_in_locals = act.borrow().locals.contains_key(name);
            if found_in_locals {
                act.borrow_mut().locals.insert(name.to_string(), val.clone());
                return Ok(val);
            }
            let found_in_args = act.borrow().args.contains_key(name);
            if found_in_args {
                act.borrow_mut().args.insert(name.to_string(), val.clone());
                return Ok(val);
            }

            let act_ref = act.borrow();
            if act_ref.holder.is_some() {
                if let Value::Object(obj) = &act_ref.self_val {
                    let cls = obj.borrow().class.clone();
                    let cls_borrow = cls.borrow();
                    if let Some(idx) = cls_borrow.instance_fields.iter().position(|f| f == name) {
                        obj.borrow_mut().fields[idx] = val.clone();
                        return Ok(val);
                    }
                }
            }
            current_act = act_ref.parent.clone();
        }

        Err(anyhow!("Cannot assign to: {}", name))
    }

    pub fn dispatch(&self, receiver: Value, selector: &str, args: Vec<Value>) -> Result<Value> {
        match self.dispatch_internal(receiver, selector, args)? {
            ReturnValue::Value(v) => Ok(v),
            ReturnValue::NonLocalReturn(_, _) => Err(anyhow!("Non-local return escaped method scope")),
        }
    }

    pub fn dispatch_internal(&self, receiver: Value, selector: &str, args: Vec<Value>) -> Result<ReturnValue> {
        thread_local! {
            static DEPTH: std::cell::Cell<usize> = std::cell::Cell::new(0);
        }
        let depth = DEPTH.with(|d| {
            let next = d.get() + 1;
            d.set(next);
            next
        });
        if depth > 1000 {
            return Err(anyhow!("Recursion limit exceeded in dispatch"));
        }

        let res = self.dispatch_internal_actual(receiver, selector, args);
        
        DEPTH.with(|d| d.set(d.get() - 1));
        res
    }

    fn dispatch_internal_actual(&self, receiver: Value, selector: &str, args: Vec<Value>) -> Result<ReturnValue> {
        if let Value::Block(block) = &receiver {
            if selector == "value" || selector == "value:" || selector == "value:with:" || selector == "value:with:with:" {
                return self.run_block(block.clone(), args);
            }
        }
        let cls = self.get_class(&receiver)?;
        
        // Special case: asInteger on an Integer returns itself.
        // This is to avoid modifying Integer.som while supporting polymorphic calls.
        if selector == "asInteger" {
            if let Value::Integer(_) = &receiver {
                return Ok(ReturnValue::Value(receiver.clone()));
            }
        }

        let method = self.lookup_method(cls, selector)?;
        self.run_method_internal(method, receiver, args)
    }

    fn get_class(&self, val: &Value) -> Result<SomRef<SomClass>> {
        // eprintln!("DEBUG: get_class({:?})", val);
        match val {
            Value::Integer(_) => self.universe.load_class("Integer"),
            Value::String(_) => self.universe.load_class("String"),
            Value::Boolean(true) => self.universe.load_class("True"),
            Value::Boolean(false) => self.universe.load_class("False"),
            Value::Nil => self.universe.load_class("Nil"),
            Value::Double(_) => self.universe.load_class("Double"),
            Value::Object(obj) => Ok(obj.borrow().class.clone()),
            Value::Array(_) => self.universe.load_class("Array"),
            Value::Class(cls) => Ok(cls.borrow().class.as_ref().ok_or_else(|| anyhow!("Class has no metaclass"))?.clone()),
            Value::Block(_) => self.universe.load_class("Block"),
            Value::Symbol(_) => self.universe.load_class("Symbol"),
            Value::Method(_) => self.universe.load_class("Method"),
        }
    }

    fn lookup_method(&self, mut cls: SomRef<SomClass>, selector: &str) -> Result<SomRef<SomMethod>> {
        let start_cls = cls.borrow().name.clone();
        loop {
            if let Some(m) = cls.borrow().methods.get(selector) {
                return Ok(m.clone());
            }
            let next = cls.borrow().super_class.clone();
            if let Some(sup) = next {
                cls = sup;
            } else {
                break;
            }
        }
        Err(anyhow!("Method {} not found in {} (started at {})", selector, cls.borrow().name, start_cls))
    }
}
