mod lexer;
mod ast;
mod parser;
mod object;
mod universe;
mod interpreter;
mod primitives;

use std::path::PathBuf;
use crate::universe::Universe;
use crate::interpreter::Interpreter;
use crate::object::{Value, SomObject};
use anyhow::Result;
use clap::Parser as ClapParser;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(ClapParser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_name = "PATH")]
    classpath: Option<String>,

    #[arg(trailing_var_arg = true)]
    rest: Vec<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    let child = std::thread::Builder::new()
        .stack_size(128 * 1024 * 1024)
        .spawn(move || {
            run_with_args(args)
        })?;
    
    child.join().map_err(|e| anyhow::anyhow!("Thread panicked: {:?}", e))?
}

fn run_with_args(args: Args) -> Result<()> {
    let mut classpath = vec![
        PathBuf::from("SOM/Smalltalk"),
        PathBuf::from("SOM/TestSuite"),
        PathBuf::from("."),
    ];
    if let Some(cp) = args.classpath {
        classpath.extend(cp.split(';').map(PathBuf::from));
    }

    let universe = Universe::new(classpath);
    
    // Bootstrap: Load core classes
    universe.load_class("Object")?;
    universe.load_class("Class")?;
    universe.load_class("Metaclass")?;
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

    if !args.rest.is_empty() {
        let filename = &args.rest[0];
        let path = PathBuf::from(filename);
        let class_name = path.file_stem().unwrap().to_str().unwrap();
        let main_class = universe.load_class(class_name)?;
        
        // Instantiate main class
        let instance = Rc::new(RefCell::new(SomObject {
            class: main_class.clone(),
            fields: vec![Value::Nil; main_class.borrow().instance_fields.len()],
        }));

        // Prep arguments for SOM (Array of strings)
        let som_args: Vec<Value> = args.rest.iter()
            .map(|s| Value::new_string(s.clone()))
            .collect();
        let args_array = Value::Array(Rc::new(RefCell::new(som_args)));

        println!("Running {}...", class_name);
        
        // SOM convention: application run: args or application run
        if main_class.borrow().methods.contains_key("run:") {
            interpreter.dispatch(Value::Object(instance), "run:", vec![args_array])?;
        } else {
            interpreter.dispatch(Value::Object(instance), "run", Vec::new())?;
        }
    } else {
        use rustyline::error::ReadlineError;
        use rustyline::DefaultEditor;

        let mut rl = DefaultEditor::new()?;
        println!("SOM REPL (minimal). Type 'exit' to quit.");

        loop {
            let readline = rl.readline(">> ");
            match readline {
                Ok(line) => {
                    if line.trim() == "exit" { break; }
                    rl.add_history_entry(line.as_str())?;
                    
                    let mut parser = crate::parser::Parser::new(&line);
                    match parser.parse_expression() {
                        Ok(expr) => {
                            let activation = Rc::new(RefCell::new(crate::object::Activation {
                                holder: None,
                                self_val: Value::Nil,
                                args: std::collections::HashMap::new(),
                                locals: std::collections::HashMap::new(),
                                parent: None,
                                is_active: true,
                             }));
                            match interpreter.evaluate_expression(&expr, activation) {
                                Ok(val) => println!("{:?}", val),
                                Err(e) => println!("Error: {}", e),
                            }
                        }
                        Err(e) => println!("Parse Error: {}", e),
                    }
                }
                Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }
    }

    Ok(())
}
