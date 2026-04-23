use std::path::PathBuf;
use anyhow::Result;
use clap::Parser as ClapParser;
use lazysom::universe::Universe;
use lazysom::interpreter::Interpreter;
use lazysom::object::{Value, SomObject};
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
    run_with_args(args)
}

fn run_with_args(args: Args) -> Result<()> {
    let mut classpath_extra = Vec::new();
    if let Some(cp) = args.classpath {
        classpath_extra.extend(cp.split(';').map(PathBuf::from));
    }

    if !args.rest.is_empty() {
        let filename = args.rest[0].clone();
        println!("Running {}...", filename);
        lazysom::run_som(classpath_extra, &filename, args.rest)
    } else {
        // Keep REPL logic here for now as it's more specific to the CLI
        let mut classpath = vec![
            PathBuf::from("SOM/Smalltalk"),
            PathBuf::from("SOM/TestSuite"),
            PathBuf::from("."),
        ];
        classpath.extend(classpath_extra);

        let universe = Universe::new(classpath);
        universe.load_class("Object")?;
        universe.load_class("Class")?;
        universe.load_class("Metaclass")?;
        let sys_class = universe.load_class("System")?;
        
        let system_obj = Rc::new(RefCell::new(SomObject {
            class: sys_class.clone(),
            fields: Vec::new(),
        }));
        universe.set_global("system", Value::Object(system_obj.clone()));
        universe.set_global("nil", Value::Nil);
        universe.set_global("true", Value::Boolean(true));
        universe.set_global("false", Value::Boolean(false));

        let interpreter = Interpreter::new(&universe);

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
                    
                    let mut parser = lazysom::parser::Parser::new(&line);
                    match parser.parse_expression() {
                        Ok(expr) => {
                            let activation = Rc::new(RefCell::new(lazysom::object::Activation {
                                holder: None,
                                self_val: Value::Nil,
                                args: std::collections::HashMap::new(),
                                locals: std::collections::HashMap::new(),
                                parent: None,
                                is_active: true,
                             }));
                            match interpreter.evaluate_expression(&expr, activation, false) {
                                Ok(lazysom::interpreter::ReturnValue::Value(val)) => println!("{:?}", val),
                                Ok(lazysom::interpreter::ReturnValue::TailCall(recv, sel, args, sup)) => {
                                    match interpreter.dispatch_internal_with_super(recv, &sel, args, sup) {
                                        Ok(lazysom::interpreter::ReturnValue::Value(val)) => println!("{:?}", val),
                                        res => println!("{:?}", res),
                                    }
                                }
                                Ok(res) => println!("{:?}", res),
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
        Ok(())
    }
}
