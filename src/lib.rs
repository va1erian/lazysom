pub mod lexer;
pub mod ast;
pub mod parser;
pub mod object;
pub mod universe;
pub mod interpreter;
pub mod primitives;

use std::path::PathBuf;
use crate::universe::Universe;
use crate::object::{Value, SomObject};
use crate::interpreter::Interpreter;
use anyhow::Result;
use std::rc::Rc;
use std::cell::RefCell;

pub fn run_som(classpath_extra: Vec<PathBuf>, filename: &str, args: Vec<String>) -> Result<()> {
    let mut classpath = vec![
        PathBuf::from("SOM/Smalltalk"),
        PathBuf::from("SOM/TestSuite"),
        PathBuf::from("."),
        PathBuf::from("tests"),
    ];
    classpath.extend(classpath_extra);

    let universe = Universe::new(classpath);
    
    // Bootstrap: Load core classes
    universe.load_class("Object")?;
    universe.load_class("Class")?;
    universe.load_class("Metaclass")?;
    universe.load_class("TestCase")?;
    let sys_class = universe.load_class("System")?;
    
    // Create 'system' object
    let system_obj = Rc::new(RefCell::new(SomObject {
        class: sys_class.clone(),
        fields: Vec::new(),
    }));
    universe.set_global("system", Value::Object(system_obj.clone()));
    universe.set_global("nil", Value::Nil);
    universe.set_global("true", Value::Boolean(true));
    universe.set_global("false", Value::Boolean(false));

    let interpreter = Interpreter::new(&universe);

    let path = PathBuf::from(filename);
    let class_name = path.file_stem().unwrap().to_str().unwrap();
    let main_class = universe.load_class(class_name)?;
    
    // Instantiate main class
    let instance = Rc::new(RefCell::new(SomObject {
        class: main_class.clone(),
        fields: vec![Value::Nil; main_class.borrow().instance_fields.len()],
    }));

    // Prep arguments for SOM (Array of strings)
    let som_args: Vec<Value> = args.iter()
        .map(|s| Value::new_string(s.clone()))
        .collect();
    let args_array = Value::Array(Rc::new(RefCell::new(som_args)));

    // SOM convention: application run: args or application run
    if main_class.borrow().methods.contains_key("run:") {
        interpreter.dispatch(Value::Object(instance), "run:".to_string(), vec![args_array])?;
    } else {
        interpreter.dispatch(Value::Object(instance), "run".to_string(), Vec::new())?;
    }

    Ok(())
}
