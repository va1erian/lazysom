use crate::interpreter::{Interpreter, ReturnValue};
use crate::object::*;
use crate::universe::Universe;
use anyhow::Result;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::collections::HashMap;

pub fn register(prims: &mut HashMap<String, fn(&Value, Vec<Value>, &Universe, &Interpreter) -> Result<ReturnValue>>) {
    prims.insert("System>>global:".to_string(), sys_global);
    prims.insert("System>>global:put:".to_string(), sys_global_put);
    prims.insert("System>>hasGlobal:".to_string(), sys_has_global);
    prims.insert("System>>load:".to_string(), sys_load);
    prims.insert("System>>exit:".to_string(), sys_exit);
    prims.insert("System>>printString:".to_string(), sys_print_string);
    prims.insert("System>>printNewline".to_string(), sys_print_newline);
    prims.insert("System>>time".to_string(), sys_time);
    prims.insert("System>>ticks".to_string(), sys_ticks);
    prims.insert("System>>fullGC".to_string(), sys_full_gc);
    prims.insert("System>>loadFile:".to_string(), sys_load_file);
    prims.insert("System>>evaluate:".to_string(), sys_evaluate);
    prims.insert("System>>compileMethod:inClass:".to_string(), sys_compile_method_in_class);
    prims.insert("System>>installMethod:inClass:".to_string(), sys_install_method_in_class);
    prims.insert("System>>defineClass:".to_string(), sys_define_class);
    prims.insert("System>>classNames".to_string(), sys_class_names);
    prims.insert("System>>serialize:format:".to_string(), sys_serialize_format);
    prims.insert("System>>deserialize:format:".to_string(), sys_deserialize_format);
    prims.insert("System>>readText:".to_string(), sys_file_read_text);
    prims.insert("System>>writeText:to:".to_string(), sys_file_write_text);
    prims.insert("System>>appendText:to:".to_string(), sys_file_append_text);

    prims.insert("System>>evaluateAsync:".to_string(), sys_evaluate_async);
    prims.insert("System>>bgTaskStatus".to_string(), sys_bg_task_status);
    prims.insert("System>>bgTaskResult".to_string(), sys_bg_task_result);
    prims.insert("System>>bgTaskError".to_string(), sys_bg_task_error);
    prims.insert("System>>bgTaskFrames".to_string(), sys_bg_task_frames);
    prims.insert("System>>bgTaskFrameVarsAt:".to_string(), sys_bg_task_frame_vars);
    prims.insert("System>>bgTaskFrameSourceAt:".to_string(), sys_bg_task_frame_source);
    prims.insert("System>>bgTaskCommand:".to_string(), sys_bg_task_command);
}

fn sys_print_string(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(arg) = args.get(0) {
        match arg {
            Value::String(s) => print!("{}", s.borrow()),
            Value::Symbol(s) => print!("{}", s),
            _ => {}
        }
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn sys_print_newline(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    println!();
    Ok(ReturnValue::Value(Value::Nil))
}

fn sys_global(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(arg) = args.get(0) {
        let name = match arg {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Nil)),
        };
        if let Some(val) = universe.get_global(&name) {
            return Ok(ReturnValue::Value(val));
        }
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn sys_global_put(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(arg), Some(val)) = (args.get(0), args.get(1)) {
        let name = match arg {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Nil)),
        };
        universe.set_global(&name, val.clone());
        return Ok(ReturnValue::Value(val.clone()));
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn sys_has_global(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(arg) = args.get(0) {
        let name = match arg {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
        };
        return Ok(ReturnValue::Value(Value::Boolean(universe.get_global(&name).is_some())));
    }
    Ok(ReturnValue::Value(Value::Boolean(false)))
}

fn sys_exit(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(Value::Integer(code)) = args.get(0) {
        std::process::exit(code.to_i32().unwrap_or(0));
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn sys_load(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(arg) = args.get(0) {
        let name = match arg {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Nil)),
        };
        match universe.load_class(&name) {
            Ok(cls) => return Ok(ReturnValue::Value(Value::Class(cls))),
            Err(_) => return Ok(ReturnValue::Value(Value::Nil)),
        }
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn sys_time(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    Ok(ReturnValue::Value(Value::Integer(BigInt::from(now))))
}

fn sys_ticks(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros();
    Ok(ReturnValue::Value(Value::Integer(BigInt::from(now))))
}

fn sys_full_gc(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    gc::force_collect();
    Ok(ReturnValue::Value(Value::Boolean(true)))
}

fn sys_serialize_format(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if args.len() != 2 { return Ok(ReturnValue::Value(Value::Nil)); }
    let obj = &args[0];
    let format_val = &args[1];
    let format_str = match format_val {
        Value::String(s) => s.borrow().clone(),
        Value::Symbol(s) => s.clone(),
        _ => return Ok(ReturnValue::Value(Value::Nil)),
    };
    if format_str == "json" {
        match crate::serialize::to_json(obj) {
            Ok(s) => Ok(ReturnValue::Value(Value::new_string(s))),
            Err(_) => Ok(ReturnValue::Value(Value::Nil)),
        }
    } else if format_str == "msgpack" {
        match crate::serialize::to_msgpack(obj) {
            Ok(bytes) => {
                let elements = bytes.into_iter().map(|b| Value::Integer(num_bigint::BigInt::from(b))).collect();
                Ok(ReturnValue::Value(Value::Array(crate::object::som_ref(elements))))
            }
            Err(_) => Ok(ReturnValue::Value(Value::Nil)),
        }
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn sys_deserialize_format(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if args.len() != 2 { return Ok(ReturnValue::Value(Value::Nil)); }
    let data = &args[0];
    let format_val = &args[1];
    let format_str = match format_val {
        Value::String(s) => s.borrow().clone(),
        Value::Symbol(s) => s.clone(),
        _ => return Ok(ReturnValue::Value(Value::Nil)),
    };
    if format_str == "json" {
        let json_str = match data {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Nil)),
        };
        let ir: crate::serialize::SerializedValue = match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(_) => return Ok(ReturnValue::Value(Value::Nil)),
        };
        let mut deserializer = crate::serialize::SomDeserializer::new(universe);
        match deserializer.deserialize(&ir) {
            Ok(val) => Ok(ReturnValue::Value(val)),
            Err(_) => Ok(ReturnValue::Value(Value::Nil)),
        }
    } else if format_str == "msgpack" {
        let bytes = match data {
            Value::Array(arr) => arr.borrow().iter().filter_map(|v| if let Value::Integer(i) = v { i.to_u8() } else { None }).collect::<Vec<u8>>(),
            _ => return Ok(ReturnValue::Value(Value::Nil)),
        };
        let ir: crate::serialize::SerializedValue = match rmp_serde::from_slice(&bytes) {
            Ok(v) => v,
            Err(_) => return Ok(ReturnValue::Value(Value::Nil)),
        };
        let mut deserializer = crate::serialize::SomDeserializer::new(universe);
        match deserializer.deserialize(&ir) {
            Ok(val) => Ok(ReturnValue::Value(val)),
            Err(_) => Ok(ReturnValue::Value(Value::Nil)),
        }
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn sys_load_file(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(arg) = args.get(0) {
        let file_name = match arg {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Nil)),
        };
        if let Ok(content) = std::fs::read_to_string(file_name) {
            return Ok(ReturnValue::Value(Value::new_string(content)));
        }
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn sys_evaluate(_: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let Some(Value::String(code)) = args.get(0) {
        match interpreter.evaluate_snippet(code.borrow().as_str()) {
            Ok(val) => return Ok(ReturnValue::Value(val)),
            Err(e) => return Ok(ReturnValue::Value(Value::new_string(format!("Error: {}", e)))),
        }
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn sys_compile_method_in_class(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(Value::String(code)), Some(Value::Class(cls))) = (args.get(0), args.get(1)) {
        let code_str = code.borrow().clone();
        let mut parser = crate::parser::Parser::new(&code_str);
        match parser.parse_method() {
            Ok(method_def) => match universe.assemble_method(method_def, cls.clone()) {
                Ok(method) => return Ok(ReturnValue::Value(Value::Method(crate::object::som_ref(method)))),
                Err(e) => return Ok(ReturnValue::Value(Value::new_string(format!("Error compiling: {}", e)))),
            },
            Err(e) => return Ok(ReturnValue::Value(Value::new_string(format!("Error parsing: {}", e)))),
        }
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn sys_install_method_in_class(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(Value::Method(method)), Some(Value::Class(cls))) = (args.get(0), args.get(1)) {
        let sig = method.borrow().signature.clone();
        cls.borrow_mut().methods.insert(sig.clone(), method.clone());
        if !cls.borrow().method_order.contains(&sig) { cls.borrow_mut().method_order.push(sig); }
        return Ok(ReturnValue::Value(Value::Boolean(true)));
    }
    Ok(ReturnValue::Value(Value::Boolean(false)))
}

fn sys_define_class(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(Value::String(code)) = args.get(0) {
        let code_str = code.borrow().clone();
        let mut parser = crate::parser::Parser::new(&code_str);
        match parser.parse_class() {
            Ok(class_def) => {
                let name = class_def.name.clone();
                let existing_cls = if let Some(Value::Class(cls)) = universe.globals.borrow().get(&name) {
                    Some(cls.clone())
                } else {
                    None
                };
                
                let stub = match existing_cls {
                    Some(cls) => cls,
                    None => {
                        let s = som_ref(SomClass {
                            name: name.to_string(),
                            class: None,
                            super_class: None,
                            instance_fields: Vec::new(),
                            fields: Vec::new(),
                            methods: HashMap::new(),
                            method_order: Vec::new(),
                        });
                        universe.globals.borrow_mut().insert(name.to_string(), Value::Class(s.clone()));
                        s
                    }
                };
                match universe.assemble_class_into(class_def, stub.clone()) {
                    Ok(_) => return Ok(ReturnValue::Value(Value::Class(stub))),
                    Err(e) => return Ok(ReturnValue::Value(Value::new_string(format!("Error assembling class: {}", e)))),
                }
            }
            Err(e) => return Ok(ReturnValue::Value(Value::new_string(format!("Error parsing class: {}", e)))),
        }
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn sys_class_names(_: &Value, _: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    let mut names_set = std::collections::HashSet::new();
    {
        let globals = universe.globals.borrow();
        for (name, val) in globals.iter() {
            if let Value::Class(_) = val { names_set.insert(name.clone()); }
        }
    }
    for path in &universe.classpath {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map(|e| e == "som").unwrap_or(false) {
                    if let Some(name) = p.file_stem().and_then(|s| s.to_str()) {
                        if !name.contains(' ') { names_set.insert(name.to_string()); }
                    }
                }
            }
        }
    }
    let mut names_vec: Vec<String> = names_set.into_iter().collect();
    names_vec.sort();
    let names: Vec<Value> = names_vec.into_iter().map(|name| Value::Symbol(name)).collect();
    Ok(ReturnValue::Value(Value::Array(som_ref(names))))
}

fn sys_file_read_text(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(Value::String(path)) = args.get(0) {
        let path_str = path.borrow().clone();
        if let Ok(content) = std::fs::read_to_string(path_str) {
            return Ok(ReturnValue::Value(Value::new_string(content)));
        }
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn sys_file_write_text(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(Value::String(content)), Some(Value::String(path))) = (args.get(0), args.get(1)) {
        let path_str = path.borrow().clone();
        let content_str = content.borrow().clone();
        if path_str.contains("..") { return Ok(ReturnValue::Value(Value::new_string("Error: Path traversal not allowed".to_string()))); }
        if !path_str.starts_with("Tools/") && !path_str.starts_with("User/") { return Ok(ReturnValue::Value(Value::new_string("Error: Can only write to Tools/ or User/ directory".to_string()))); }
        if std::fs::write(&path_str, content_str).is_ok() { return Ok(ReturnValue::Value(Value::Boolean(true))); }
    }
    Ok(ReturnValue::Value(Value::Boolean(false)))
}

fn sys_file_append_text(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(Value::String(content)), Some(Value::String(path))) = (args.get(0), args.get(1)) {
        let path_str = path.borrow().clone();
        let content_str = content.borrow().clone();
        if path_str.contains("..") { return Ok(ReturnValue::Value(Value::new_string("Error: Path traversal not allowed".to_string()))); }
        if !path_str.starts_with("Tools/") && !path_str.starts_with("User/") { return Ok(ReturnValue::Value(Value::new_string("Error: Can only write to Tools/ or User/ directory".to_string()))); }
        use std::io::Write;
        let file_res = std::fs::OpenOptions::new().write(true).append(true).create(true).open(&path_str);
        if let Ok(mut file) = file_res {
            if file.write_all(content_str.as_bytes()).is_ok() { return Ok(ReturnValue::Value(Value::Boolean(true))); }
        } else {
            return Ok(ReturnValue::Value(Value::new_string("Error: Could not open file".to_string())));
        }
    }
    Ok(ReturnValue::Value(Value::Boolean(false)))
}

fn sys_evaluate_async(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(Value::String(code)) = args.get(0) {
        let code_str = code.borrow().clone();
        let mut runner = crate::vm_runner::VM_RUNNER.lock().unwrap();
        runner.start_snippet(code_str);
        return Ok(ReturnValue::Value(Value::Boolean(true)));
    }
    Ok(ReturnValue::Value(Value::Boolean(false)))
}

fn sys_bg_task_status(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    let mut runner = crate::vm_runner::VM_RUNNER.lock().unwrap();
    runner.poll_events();
    let status = runner.get_status_str();
    Ok(ReturnValue::Value(Value::new_string(status)))
}

fn sys_bg_task_result(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    let runner = crate::vm_runner::VM_RUNNER.lock().unwrap();
    let result = runner.get_result();
    Ok(ReturnValue::Value(Value::new_string(result)))
}

fn sys_bg_task_error(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    let runner = crate::vm_runner::VM_RUNNER.lock().unwrap();
    let err = runner.get_error();
    Ok(ReturnValue::Value(Value::new_string(err)))
}

fn sys_bg_task_frames(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    let runner = crate::vm_runner::VM_RUNNER.lock().unwrap();
    let stack = runner.get_stack();
    let mut frames = Vec::new();
    for f in stack {
        frames.push(Value::new_string(f.name.clone()));
    }
    Ok(ReturnValue::Value(Value::Array(crate::object::som_ref(frames))))
}

fn sys_bg_task_frame_vars(_: &Value, args: Vec<Value>, _universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(Value::Integer(idx_val)) = args.get(0) {
        let idx = idx_val.to_usize().unwrap_or(0);
        let runner = crate::vm_runner::VM_RUNNER.lock().unwrap();
        let stack = runner.get_stack();
        if let Some(frame) = stack.get(idx) {
            let mut vars_list = Vec::new();
            
            let mut add_vars = |name: &str, val_type: &str, val_str: &str| {
                let var_triple = vec![
                    Value::new_string(name.to_string()),
                    Value::new_string(val_type.to_string()),
                    Value::new_string(val_str.to_string()),
                ];
                vars_list.push(Value::Array(crate::object::som_ref(var_triple)));
            };

            add_vars("self", &frame.class_name, &frame.receiver_str);

            for var in &frame.args {
                add_vars(&var.name, &var.val_type, &var.val_str);
            }

            for var in &frame.locals {
                add_vars(&var.name, &var.val_type, &var.val_str);
            }

            return Ok(ReturnValue::Value(Value::Array(crate::object::som_ref(vars_list))));
        }
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn sys_bg_task_frame_source(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(Value::Integer(idx_val)) = args.get(0) {
        let idx = idx_val.to_usize().unwrap_or(0);
        let runner = crate::vm_runner::VM_RUNNER.lock().unwrap();
        let stack = runner.get_stack();
        if let Some(frame) = stack.get(idx) {
            if let Some(src) = &frame.source {
                return Ok(ReturnValue::Value(Value::new_string(src.clone())));
            }
        }
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn sys_bg_task_command(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(Value::String(cmd_val)) = args.get(0) {
        let cmd = cmd_val.borrow().clone();
        let mut runner = crate::vm_runner::VM_RUNNER.lock().unwrap();
        match cmd.as_str() {
            "resume" => runner.resume(),
            "stepInto" => runner.step_into(),
            "stop" => runner.stop(),
            _ => {}
        }
        return Ok(ReturnValue::Value(Value::Boolean(true)));
    }
    Ok(ReturnValue::Value(Value::Boolean(false)))
}
