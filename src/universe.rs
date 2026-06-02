use std::collections::HashMap;
use std::cell::{RefCell, Cell};
use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use crate::object::*;
use crate::parser::Parser;
use crate::ast::{ClassDef, MethodDef, MethodBody, Signature};
use anyhow::{Result, anyhow};
use tokio::runtime::Runtime;

pub enum AsyncResult {
    SuccessNil,
    SuccessString(String),
    SuccessHandle(usize),
    Error(String),
}

pub struct AsyncEvent {
    pub callback_id: u64,
    pub result: AsyncResult,
}

pub struct AsyncCallbacks {
    pub success_block: Value,
    pub error_block: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VmState {
    Running,
    Paused,
    Stepping,
}

pub struct Universe {
    pub globals: RefCell<HashMap<String, Value>>,
    pub classpath: Vec<PathBuf>,
    pub primitives: HashMap<String, fn(&Value, Vec<Value>, &Universe, &crate::interpreter::Interpreter) -> Result<crate::interpreter::ReturnValue>>,

    // Async support
    pub tokio_rt: Arc<Runtime>,
    pub async_tx: mpsc::Sender<AsyncEvent>,
    pub async_rx: RefCell<mpsc::Receiver<AsyncEvent>>,
    pub async_callbacks: RefCell<HashMap<u64, AsyncCallbacks>>,
    pub next_async_id: Cell<u64>,

    // Debugger state
    pub vm_state: RefCell<VmState>,
    pub active_frames: RefCell<Vec<crate::object::SomRef<crate::bytecode_interpreter::Frame>>>,

    // AST Debugger state
    pub active_activations: RefCell<Vec<crate::object::SomRef<crate::object::Activation>>>,
    pub dbg_cmd_rx: Option<std::sync::mpsc::Receiver<crate::vm_runner::DebugCommand>>,
    pub dbg_event_tx: Option<std::sync::mpsc::Sender<crate::vm_runner::DebugEvent>>,
}

impl Universe {
    pub fn new(classpath: Vec<PathBuf>) -> Self {
        let mut globals = HashMap::new();
        
        // Initial stub for bootstrap: Metaclass
        let metaclass = som_ref(SomClass {
            name: "Metaclass".to_string(),
            class: None,
            super_class: None,
            instance_fields: Vec::new(),
            fields: Vec::new(),
            methods: HashMap::new(),
            method_order: Vec::new(),
        });
        metaclass.borrow_mut().class = Some(metaclass.clone());
        globals.insert("Metaclass".to_string(), Value::Class(metaclass));

        let tokio_rt = Arc::new(Runtime::new().expect("Failed to create tokio runtime"));
        let (tx, rx) = mpsc::channel();

        Self {
            globals: RefCell::new(globals),
            classpath,
            primitives: crate::primitives::get_primitives(),
            tokio_rt,
            async_tx: tx,
            async_rx: RefCell::new(rx),
            async_callbacks: RefCell::new(HashMap::new()),
            next_async_id: Cell::new(1),
            vm_state: RefCell::new(VmState::Running),
            active_frames: RefCell::new(Vec::new()),
            active_activations: RefCell::new(Vec::new()),
            dbg_cmd_rx: None,
            dbg_event_tx: None,
        }
    }

    pub fn push_frame(&self, frame: crate::object::SomRef<crate::bytecode_interpreter::Frame>) {
        self.active_frames.borrow_mut().push(frame);
    }

    pub fn pop_frame(&self) {
        self.active_frames.borrow_mut().pop();
    }

    pub fn push_activation(&self, act: crate::object::SomRef<crate::object::Activation>) {
        self.active_activations.borrow_mut().push(act);
    }

    pub fn pop_activation(&self) {
        self.active_activations.borrow_mut().pop();
    }

    pub fn register_primitive(&mut self, class_name: &str, method_name: &str, func: fn(&Value, Vec<Value>, &Universe, &crate::interpreter::Interpreter) -> Result<crate::interpreter::ReturnValue>) {
        let key = format!("{}>>{}", class_name, method_name);
        self.primitives.insert(key, func);
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
                let s = som_ref(SomClass {
                    name: name.to_string(),
                    class: None,
                    super_class: None,
                    instance_fields: Vec::new(),
                    fields: Vec::new(),
                    methods: HashMap::new(),
                    method_order: Vec::new(),
                });
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
                            stub.borrow_mut().methods.insert("__loading__".to_string(), som_ref(SomMethod {
                                name: "".to_string(),
                                signature: "".to_string(),
                                holder: stub.clone(),
                                parameters: vec![],
                                body: crate::object::MethodBody::Primitive(|_, _, _, _| Ok(crate::interpreter::ReturnValue::Value(Value::Nil))),
                                source: None,
                            }));
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

    pub fn assemble_class_into(&self, def: ClassDef, cls: SomRef<SomClass>) -> Result<()> {
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

        let metaclass = som_ref(SomClass {
            name: mc_name,
            class: Some(self.load_class("Metaclass")?),
            super_class: mc_super,
            instance_fields: all_class_fields.clone(),
            fields: vec![Value::Nil; all_class_fields.len()],
            methods: std::collections::HashMap::new(),
            method_order: Vec::new(),
        });

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

        if def.name == "Object" {
            let halt_def = MethodDef {
                signature: Signature::Unary("halt".to_string()),
                body: MethodBody::Primitive,
                source: None,
            };
            let halt_method = self.assemble_method(halt_def, cls.clone())?;
            let sig_o = halt_method.signature.clone();
            cls.borrow_mut().methods.insert(sig_o.clone(), crate::object::som_ref(halt_method));
            // Do not insert into method_order to avoid failing standard tests that check the first method
        }

        // Dynamically add IDE primitive stubs to System class
        if def.name == "System" {
            let eval_def = MethodDef {
                signature: Signature::Keyword(vec![("evaluate:".to_string(), "code".to_string())]),
                body: MethodBody::Primitive,
                source: None,
            };
            let eval_method = self.assemble_method(eval_def, cls.clone())?;
            let sig1 = eval_method.signature.clone();
            cls.borrow_mut().methods.insert(sig1.clone(), crate::object::som_ref(eval_method));
            cls.borrow_mut().method_order.push(sig1);

            let class_names_def = MethodDef {
                signature: Signature::Unary("classNames".to_string()),
                body: MethodBody::Primitive,
                source: None,
            };
            let class_names_method = self.assemble_method(class_names_def, cls.clone())?;
            let sig2 = class_names_method.signature.clone();
            cls.borrow_mut().methods.insert(sig2.clone(), crate::object::som_ref(class_names_method));
            cls.borrow_mut().method_order.push(sig2);


            let serialize_def = MethodDef {
                signature: Signature::Keyword(vec![("serialize:".to_string(), "object".to_string()), ("format:".to_string(), "formatString".to_string())]),
                body: MethodBody::Primitive,
                source: None,
            };
            let serialize_method = self.assemble_method(serialize_def, cls.clone())?;
            let sig3 = serialize_method.signature.clone();
            cls.borrow_mut().methods.insert(sig3.clone(), crate::object::som_ref(serialize_method));
            cls.borrow_mut().method_order.push(sig3);

            let deserialize_def = MethodDef {
                signature: Signature::Keyword(vec![("deserialize:".to_string(), "data".to_string()), ("format:".to_string(), "formatString".to_string())]),
                body: MethodBody::Primitive,
                source: None,
            };
            let deserialize_method = self.assemble_method(deserialize_def, cls.clone())?;
            let sig4 = deserialize_method.signature.clone();
            cls.borrow_mut().methods.insert(sig4.clone(), crate::object::som_ref(deserialize_method));
            cls.borrow_mut().method_order.push(sig4);

            let compile_def = MethodDef {
                signature: Signature::Keyword(vec![("compileMethod:".to_string(), "code".to_string()), ("inClass:".to_string(), "cls".to_string())]),
                body: MethodBody::Primitive,
                source: None,
            };
            let compile_method = self.assemble_method(compile_def, cls.clone())?;
            let sig3 = compile_method.signature.clone();
            cls.borrow_mut().methods.insert(sig3.clone(), crate::object::som_ref(compile_method));
            cls.borrow_mut().method_order.push(sig3);

            let install_def = MethodDef {
                signature: Signature::Keyword(vec![("installMethod:".to_string(), "method".to_string()), ("inClass:".to_string(), "cls".to_string())]),
                body: MethodBody::Primitive,
                source: None,
            };
            let install_method = self.assemble_method(install_def, cls.clone())?;
            let sig4 = install_method.signature.clone();
            cls.borrow_mut().methods.insert(sig4.clone(), crate::object::som_ref(install_method));
            cls.borrow_mut().method_order.push(sig4);

            let define_class_def = MethodDef {
                signature: Signature::Keyword(vec![("defineClass:".to_string(), "code".to_string())]),
                body: MethodBody::Primitive,
                source: None,
            };
            let define_class_method = self.assemble_method(define_class_def, cls.clone())?;
            let sig_dc = define_class_method.signature.clone();
            cls.borrow_mut().methods.insert(sig_dc.clone(), crate::object::som_ref(define_class_method));
            cls.borrow_mut().method_order.push(sig_dc);

            let read_def = MethodDef {
                signature: Signature::Keyword(vec![("readText:".to_string(), "path".to_string())]),
                body: MethodBody::Primitive,
                source: None,
            };
            let read_method = self.assemble_method(read_def, cls.clone())?;
            let sig5 = read_method.signature.clone();
            cls.borrow_mut().methods.insert(sig5.clone(), crate::object::som_ref(read_method));
            cls.borrow_mut().method_order.push(sig5);

            let write_def = MethodDef {
                signature: Signature::Keyword(vec![("writeText:".to_string(), "content".to_string()), ("to:".to_string(), "path".to_string())]),
                body: MethodBody::Primitive,
                source: None,
            };
            let write_method = self.assemble_method(write_def, cls.clone())?;
            let sig6 = write_method.signature.clone();
            cls.borrow_mut().methods.insert(sig6.clone(), crate::object::som_ref(write_method));
            cls.borrow_mut().method_order.push(sig6);

            let append_def = MethodDef {
                signature: Signature::Keyword(vec![("appendText:".to_string(), "content".to_string()), ("to:".to_string(), "path".to_string())]),
                body: MethodBody::Primitive,
                source: None,
            };
            let append_method = self.assemble_method(append_def, cls.clone())?;
            let sig_app = append_method.signature.clone();
            cls.borrow_mut().methods.insert(sig_app.clone(), crate::object::som_ref(append_method));
            cls.borrow_mut().method_order.push(sig_app);

            let eval_async_def = MethodDef {
                signature: Signature::Keyword(vec![("evaluateAsync:".to_string(), "code".to_string())]),
                body: MethodBody::Primitive,
                source: None,
            };
            let eval_async_method = self.assemble_method(eval_async_def, cls.clone())?;
            let sig_ea = eval_async_method.signature.clone();
            cls.borrow_mut().methods.insert(sig_ea.clone(), crate::object::som_ref(eval_async_method));
            cls.borrow_mut().method_order.push(sig_ea);

            let bg_status_def = MethodDef {
                signature: Signature::Unary("bgTaskStatus".to_string()),
                body: MethodBody::Primitive,
                source: None,
            };
            let bg_status_method = self.assemble_method(bg_status_def, cls.clone())?;
            let sig_bgs = bg_status_method.signature.clone();
            cls.borrow_mut().methods.insert(sig_bgs.clone(), crate::object::som_ref(bg_status_method));
            cls.borrow_mut().method_order.push(sig_bgs);

            let bg_result_def = MethodDef {
                signature: Signature::Unary("bgTaskResult".to_string()),
                body: MethodBody::Primitive,
                source: None,
            };
            let bg_result_method = self.assemble_method(bg_result_def, cls.clone())?;
            let sig_bgr = bg_result_method.signature.clone();
            cls.borrow_mut().methods.insert(sig_bgr.clone(), crate::object::som_ref(bg_result_method));
            cls.borrow_mut().method_order.push(sig_bgr);

            let bg_error_def = MethodDef {
                signature: Signature::Unary("bgTaskError".to_string()),
                body: MethodBody::Primitive,
                source: None,
            };
            let bg_error_method = self.assemble_method(bg_error_def, cls.clone())?;
            let sig_bge = bg_error_method.signature.clone();
            cls.borrow_mut().methods.insert(sig_bge.clone(), crate::object::som_ref(bg_error_method));
            cls.borrow_mut().method_order.push(sig_bge);

            let bg_frames_def = MethodDef {
                signature: Signature::Unary("bgTaskFrames".to_string()),
                body: MethodBody::Primitive,
                source: None,
            };
            let bg_frames_method = self.assemble_method(bg_frames_def, cls.clone())?;
            let sig_bgf = bg_frames_method.signature.clone();
            cls.borrow_mut().methods.insert(sig_bgf.clone(), crate::object::som_ref(bg_frames_method));
            cls.borrow_mut().method_order.push(sig_bgf);

            let bg_vars_def = MethodDef {
                signature: Signature::Keyword(vec![("bgTaskFrameVarsAt:".to_string(), "idx".to_string())]),
                body: MethodBody::Primitive,
                source: None,
            };
            let bg_vars_method = self.assemble_method(bg_vars_def, cls.clone())?;
            let sig_bgv = bg_vars_method.signature.clone();
            cls.borrow_mut().methods.insert(sig_bgv.clone(), crate::object::som_ref(bg_vars_method));
            cls.borrow_mut().method_order.push(sig_bgv);

            let bg_src_def = MethodDef {
                signature: Signature::Keyword(vec![("bgTaskFrameSourceAt:".to_string(), "idx".to_string())]),
                body: MethodBody::Primitive,
                source: None,
            };
            let bg_src_method = self.assemble_method(bg_src_def, cls.clone())?;
            let sig_bgsrc = bg_src_method.signature.clone();
            cls.borrow_mut().methods.insert(sig_bgsrc.clone(), crate::object::som_ref(bg_src_method));
            cls.borrow_mut().method_order.push(sig_bgsrc);

            let bg_cmd_def = MethodDef {
                signature: Signature::Keyword(vec![("bgTaskCommand:".to_string(), "cmd".to_string())]),
                body: MethodBody::Primitive,
                source: None,
            };
            let bg_cmd_method = self.assemble_method(bg_cmd_def, cls.clone())?;
            let sig_bgcmd = bg_cmd_method.signature.clone();
            cls.borrow_mut().methods.insert(sig_bgcmd.clone(), crate::object::som_ref(bg_cmd_method));
            cls.borrow_mut().method_order.push(sig_bgcmd);

            let bg_output_def = MethodDef {
                signature: Signature::Unary("bgTaskOutput".to_string()),
                body: MethodBody::Primitive,
                source: None,
            };
            let bg_output_method = self.assemble_method(bg_output_def, cls.clone())?;
            let sig_bgout = bg_output_method.signature.clone();
            cls.borrow_mut().methods.insert(sig_bgout.clone(), crate::object::som_ref(bg_output_method));
            cls.borrow_mut().method_order.push(sig_bgout);

            let reg_gui_def = MethodDef {
                signature: Signature::Keyword(vec![("registerGui:".to_string(), "title".to_string()), ("code:".to_string(), "code".to_string())]),
                body: MethodBody::Primitive,
                source: None,
            };
            let reg_gui_method = self.assemble_method(reg_gui_def, cls.clone())?;
            let sig_reggui = reg_gui_method.signature.clone();
            cls.borrow_mut().methods.insert(sig_reggui.clone(), crate::object::som_ref(reg_gui_method));
            cls.borrow_mut().method_order.push(sig_reggui);

            let reg_gui_class_def = MethodDef {
                signature: Signature::Keyword(vec![("registerGuiClass:".to_string(), "className".to_string())]),
                body: MethodBody::Primitive,
                source: None,
            };
            let reg_gui_class_method = self.assemble_method(reg_gui_class_def, cls.clone())?;
            let sig_reggui_class = reg_gui_class_method.signature.clone();
            cls.borrow_mut().methods.insert(sig_reggui_class.clone(), crate::object::som_ref(reg_gui_class_method));
            cls.borrow_mut().method_order.push(sig_reggui_class);
        }

        // Add pause primitives to Debugger
        if def.name == "Debugger" {
            let resume_def = MethodDef {
                signature: Signature::Unary("resume".to_string()),
                body: MethodBody::Primitive,
                source: None,
            };
            let resume_method = self.assemble_method(resume_def, cls.clone())?;
            let sig1 = resume_method.signature.clone();
            cls.borrow_mut().methods.insert(sig1.clone(), crate::object::som_ref(resume_method));
            cls.borrow_mut().method_order.push(sig1);

            let halt_def = MethodDef {
                signature: Signature::Unary("halt".to_string()),
                body: MethodBody::Primitive,
                source: None,
            };
            let halt_method = self.assemble_method(halt_def, metaclass.clone())?;
            let sig_h = halt_method.signature.clone();
            metaclass.borrow_mut().methods.insert(sig_h.clone(), crate::object::som_ref(halt_method));
            metaclass.borrow_mut().method_order.push(sig_h);

            let step_into_def = MethodDef {
                signature: Signature::Unary("stepInto".to_string()),
                body: MethodBody::Primitive,
                source: None,
            };
            let step_into_method = self.assemble_method(step_into_def, cls.clone())?;
            let sig2 = step_into_method.signature.clone();
            cls.borrow_mut().methods.insert(sig2.clone(), crate::object::som_ref(step_into_method));
            cls.borrow_mut().method_order.push(sig2);

            let current_frames_def = MethodDef {
                signature: Signature::Unary("currentFrames".to_string()),
                body: MethodBody::Primitive,
                source: None,
            };
            let current_frames_method = self.assemble_method(current_frames_def, cls.clone())?;
            let sig3 = current_frames_method.signature.clone();
            cls.borrow_mut().methods.insert(sig3.clone(), crate::object::som_ref(current_frames_method));
            cls.borrow_mut().method_order.push(sig3);

        }


        for m_def in def.instance_methods {
            let method = self.assemble_method(m_def, cls.clone())?;
            let sig = method.signature.clone();
            cls.borrow_mut().methods.insert(sig.clone(), som_ref(method));
            cls.borrow_mut().method_order.push(sig);
        }

        for m_def in def.class_methods {
            let method = self.assemble_method(m_def, metaclass.clone())?;
            let sig = method.signature.clone();
            metaclass.borrow_mut().methods.insert(sig.clone(), som_ref(method));
            metaclass.borrow_mut().method_order.push(sig);
        }

        if def.name == "Method" || def.name == "Primitive" {
            let source_def = MethodDef {
                signature: Signature::Unary("source".to_string()),
                body: MethodBody::Primitive,
                source: None,
            };
            let source_method = self.assemble_method(source_def, cls.clone())?;
            let sig_s = source_method.signature.clone();
            cls.borrow_mut().methods.insert(sig_s.clone(), crate::object::som_ref(source_method));
            cls.borrow_mut().method_order.push(sig_s);
        }
        
        Ok(())
    }

    pub fn assemble_method(&self, def: MethodDef, holder: SomRef<SomClass>) -> Result<SomMethod> {
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
            source: def.source,
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
