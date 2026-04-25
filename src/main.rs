#![allow(non_local_definitions)]  // gc_derive's Finalize macro
#![allow(unsafe_op_in_unsafe_fn)] // custom_trace! macro under Rust 2024

use std::path::PathBuf;
use lazysom::universe::Universe;
use lazysom::interpreter::Interpreter;
use lazysom::object::{Value, SomObject, som_ref};
use lazysom::{compiler, bytecode, bytecode_interpreter};
use anyhow::Result;
use clap::Parser as ClapParser;

#[derive(ClapParser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_name = "PATH")]
    classpath: Option<String>,

    #[arg(long, help = "Compile the application to an image")]
    compile_image: Option<String>,

    #[arg(long, help = "Run an application from a compiled image")]
    run_image: Option<String>,

    #[arg(long, help = "Run the application with eframe/egui GUI")]
    gui: bool,

    #[arg(trailing_var_arg = true)]
    rest: Vec<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.gui {
        // Run in main thread for winit event loop requirements
        return run_with_args(args);
    }

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

    if let Some(image_path) = args.compile_image {
        let core_classes = ["Object", "Class", "Metaclass", "System", "Integer", "String", "True", "False", "Nil", "Double", "Array", "Block", "Symbol", "Method", "Primitive"];
        for cls in core_classes {
            let _ = universe.load_class(cls);
        }

        let mut initial_classes_str = Vec::new();
        if !args.rest.is_empty() {
            let p = PathBuf::from(&args.rest[0]);
            initial_classes_str.push(p.file_stem().unwrap().to_str().unwrap().to_string());
        }
        let initial_classes: Vec<&str> = initial_classes_str.iter().map(|s| s.as_str()).collect();

        println!("Compiling image to {}...", image_path);
        let image = compiler::compile_image(&universe, &initial_classes)?;
        let file = std::fs::File::create(image_path)?;
        bincode::serialize_into(file, &image)?;
        println!("Image compiled successfully.");
        return Ok(());
    }

    if let Some(image_path) = args.run_image {
        if args.rest.is_empty() {
            return Err(anyhow::anyhow!("Class name required to run from image"));
        }

        let file = std::fs::File::open(image_path)?;
        let image: bytecode::Image = bincode::deserialize_from(file)?;

        universe.load_class("Object")?;
        universe.load_class("Class")?;
        universe.load_class("Metaclass")?;
        let sys_class = universe.load_class("System")?;

        let system_obj = som_ref(SomObject {
            class: sys_class.clone(),
            fields: Vec::new(),
        });
        universe.set_global("system", Value::Object(system_obj.clone()));
        universe.set_global("nil", Value::Nil);
        universe.set_global("true", Value::Boolean(true));
        universe.set_global("false", Value::Boolean(false));

        let class_name = &args.rest[0];
        println!("Running {} from image...", class_name);

        let interp = bytecode_interpreter::BytecodeInterpreter::new(&universe, image);
        interp.run(class_name, args.rest[1..].to_vec())?;
        return Ok(());
    }

    universe.load_class("Object")?;
    universe.load_class("Class")?;
    universe.load_class("Metaclass")?;
    let sys_class = universe.load_class("System")?;

    let system_obj = som_ref(SomObject {
        class: sys_class.clone(),
        fields: Vec::new(),
    });
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

        let instance = som_ref(SomObject {
            class: main_class.clone(),
            fields: vec![Value::Nil; main_class.borrow().instance_fields.len()],
        });

        let som_args: Vec<Value> = args.rest.iter()
            .map(|s| Value::new_string(s.clone()))
            .collect();
        let args_array = Value::Array(som_ref(som_args));

        if args.gui {
            println!("Starting GUI for {}...", class_name);
            // Run initialization
            if main_class.borrow().methods.contains_key("run:") {
                interpreter.dispatch(Value::Object(instance.clone()), "run:", vec![args_array])?;
            } else if main_class.borrow().methods.contains_key("run") {
                interpreter.dispatch(Value::Object(instance.clone()), "run", Vec::new())?;
            }

            let app = lazysom::gui::SomGuiApp::new(std::sync::Arc::new(universe), Value::Object(instance));
            let options = eframe::NativeOptions::default();
            eframe::run_native(
                "LazySOM GUI",
                options,
                Box::new(|_cc| Ok(Box::new(app))),
            ).map_err(|e| anyhow::anyhow!("eframe error: {:?}", e))?;
            return Ok(());
        }

        println!("Running {}...", class_name);

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

                    match interpreter.evaluate_snippet(&line) {
                        Ok(val) => println!("{:?}", val),
                        Err(e) => println!("Error: {}", e),
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
