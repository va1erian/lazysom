use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::path::PathBuf;
use crate::object::*;
use crate::parser::Parser;
use crate::ast::{ClassDef, MethodDef, MethodBody, Signature};
use anyhow::{Result, anyhow};

pub struct Universe {
    pub globals: RefCell<HashMap<String, Value>>,
    pub classpath: Vec<PathBuf>,
    pub primitives: HashMap<String, fn(&Value, Vec<Value>, &Universe, &crate::interpreter::Interpreter) -> Result<crate::interpreter::ReturnValue>>,
}

impl Universe {
    pub fn new(classpath: Vec<PathBuf>) -> Self {
        let mut globals = HashMap::new();
        
        // Initial stub for bootstrap: Metaclass
        let metaclass = Rc::new(RefCell::new(SomClass {
            name: "Metaclass".to_string(),
            class: None,
            super_class: None,
            instance_fields: Vec::new(),
            fields: Vec::new(),
            methods: HashMap::new(),
            method_order: Vec::new(),
        }));
        metaclass.borrow_mut().class = Some(metaclass.clone());
        globals.insert("Metaclass".to_string(), Value::Class(metaclass));

        Self {
            globals: RefCell::new(globals),
            classpath,
            primitives: crate::primitives::get_primitives(),
        }
    }

    pub fn load_class(&self, name: &str) -> Result<SomRef<SomClass>> {
        if let Some(Value::Class(cls)) = self.globals.borrow().get(name) {
             if name == "Metaclass" && cls.borrow().class.is_some() && cls.borrow().super_class.is_none() && cls.borrow().methods.is_empty() {
                 // Metaclass is currently the initial dummy stub from Universe::new, continue to load it properly
             } else {
                 return Ok(cls.clone());
             }
        }

        // Create stub to break recursion
        let stub_opt = self.globals.borrow().get(name).cloned();
        let stub = match stub_opt {
            Some(Value::Class(cls)) => cls,
            _ => {
                let s = Rc::new(RefCell::new(SomClass {
                    name: name.to_string(),
                    class: None,
                    super_class: None,
                    instance_fields: Vec::new(),
                    fields: Vec::new(),
                    methods: HashMap::new(),
                    method_order: Vec::new(),
                }));
                self.globals.borrow_mut().insert(name.to_string(), Value::Class(s.clone()));
                s
            }
        };

        // Try to find .som file
        for path in &self.classpath {
            let file_path = path.join(format!("{}.som", name));
            if file_path.exists() {
                let content = std::fs::read_to_string(file_path)?;
                let mut parser = Parser::new(&content);
                match parser.parse_class() {
                    Ok(class_def) => {
                        let is_metaclass = name == "Metaclass";
                        if is_metaclass {
                            // Break recursion by making the stub look "loaded" (not empty)
                            stub.borrow_mut().methods.insert("__loading__".to_string(), Rc::new(RefCell::new(SomMethod {
                                name: "".to_string(),
                                signature: "".to_string(),
                                holder: stub.clone(),
                                parameters: vec![],
                                body: crate::object::MethodBody::Primitive(|_, _, _, _| Ok(crate::interpreter::ReturnValue::Value(Value::Nil))),
                            })));
                        }
                        let res = self.assemble_class_into(class_def, stub.clone());
                        if is_metaclass {
                            stub.borrow_mut().methods.remove("__loading__");
                        }
                        res?;
                        return Ok(stub);
                    }
                    Err(e) => {
                        println!("Error loading class {}: {}", name, e);
                        return Err(e);
                    }
                }
            }
        }

        Err(anyhow!("Class {} not found in classpath", name))
    }

    fn assemble_class_into(&self, def: ClassDef, cls: SomRef<SomClass>) -> Result<()> {
        let super_class = if let Some(super_name) = def.super_class {
            if super_name == "nil" {
                None
            } else {
                Some(self.load_class(&super_name)?)
            }
        } else {
            if def.name == "Object" {
                None
            } else {
                Some(self.load_class("Object")?)
            }
        };

        // Compute all instance fields (including inherited)
        let mut all_instance_fields = Vec::new();
        if let Some(sc) = &super_class {
            all_instance_fields.extend(sc.borrow().instance_fields.clone());
        }
        all_instance_fields.extend(def.instance_fields);

        // 1. Create Metaclass for this class
        let mc_name = format!("{} class", def.name);
        let mc_super = if def.name == "Object" {
            Some(self.load_class("Class")?)
        } else if let Some(sc) = &super_class {
            match &sc.borrow().class {
                Some(c) => Some(c.clone()),
                None => Some(self.load_class("Class")?),
            }
        } else {
            Some(self.load_class("Class")?)
        };

        // Compute all class fields
        let mut all_class_fields = Vec::new();
        if let Some(mcs) = &mc_super {
            all_class_fields.extend(mcs.borrow().instance_fields.clone());
        }
        all_class_fields.extend(def.class_fields);

        let metaclass = Rc::new(RefCell::new(SomClass {
            name: mc_name,
            class: Some(self.load_class("Metaclass")?),
            super_class: mc_super,
            instance_fields: all_class_fields.clone(),
            fields: vec![Value::Nil; all_class_fields.len()],
            methods: std::collections::HashMap::new(),
            method_order: Vec::new(),
        }));

        // 2. Update the Class stub
        {
            let mut cls_mut = cls.borrow_mut();
            cls_mut.class = Some(metaclass.clone());
            cls_mut.super_class = super_class.clone();
            cls_mut.instance_fields = all_instance_fields;
            cls_mut.fields = vec![Value::Nil; all_class_fields.len()];
            // println!("DEBUG: Assembled class {} with superclass {:?}", cls_mut.name, cls_mut.super_class.as_ref().map(|sc| sc.borrow().name.clone()));
        }

        // 3. Assemble methods
        for m_def in def.instance_methods {
            let method = self.assemble_method(m_def, cls.clone())?;
            let sig = method.signature.clone();
            cls.borrow_mut().methods.insert(sig.clone(), Rc::new(RefCell::new(method)));
            cls.borrow_mut().method_order.push(sig);
        }

        for m_def in def.class_methods {
            let method = self.assemble_method(m_def, metaclass.clone())?;
            let sig = method.signature.clone();
            metaclass.borrow_mut().methods.insert(sig.clone(), Rc::new(RefCell::new(method)));
            metaclass.borrow_mut().method_order.push(sig);
        }
        
        Ok(())
    }

    fn assemble_method(&self, def: MethodDef, holder: SomRef<SomClass>) -> Result<SomMethod> {
        let signature = def.signature.selector();
        let parameters = match &def.signature {
            Signature::Unary(_) => Vec::new(),
            Signature::Binary(_, arg) => vec![arg.clone()],
            Signature::Keyword(parts) => parts.iter().map(|(_, a)| a.clone()).collect(),
        };

        let key = format!("{}>>{}", holder.borrow().name, signature);
        let body = if let Some(f) = self.primitives.get(&key) {
            crate::object::MethodBody::Primitive(*f)
        } else {
            match def.body {
                MethodBody::Primitive => {
                    crate::object::MethodBody::Primitive(|_, _, _, _| Ok(crate::interpreter::ReturnValue::Value(Value::Nil)))
                }
                MethodBody::Block(b) => crate::object::MethodBody::Ast(b),
            }
        };

        Ok(SomMethod {
            name: signature.clone(),
            signature,
            holder,
            parameters,
            body,
        })
    }

    pub fn set_global(&self, name: &str, val: Value) {
        self.globals.borrow_mut().insert(name.to_string(), val);
    }

    pub fn get_global(&self, name: &str) -> Option<Value> {
        if let Some(val) = self.globals.borrow().get(name) {
            return Some(val.clone());
        }
        // Try to load as class
        match self.load_class(name) {
            Ok(cls) => Some(Value::Class(cls)),
            Err(_) => None,
        }
    }
}
