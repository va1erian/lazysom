use lazysom::universe::Universe;
use lazysom::interpreter::{Interpreter, ReturnValue};
use lazysom::object::{Value, SomObject};
use std::rc::Rc;
use std::cell::RefCell;
use anyhow::Result;

#[test]
fn test_scripting_capabilities() -> Result<()> {
    // 1. Create universe
    let classpath = vec![
        std::path::PathBuf::from("SOM/Smalltalk"),
        std::path::PathBuf::from("SOM/TestSuite"),
        std::path::PathBuf::from("tests"), // for Hello.som
    ];
    let mut universe = Universe::new(classpath);

    // 2. Register custom primitive that can be called from SOM code
    // We will attach it to Hello>>rustGreeting:
    universe.register_primitive("Hello", "rustGreeting:", |_receiver, args, _universe, _interpreter| {
        if let Some(arg) = args.get(0) {
            match arg {
                Value::String(s) => {
                    let greeting = format!("Hello from Rust, {}!", s.borrow());
                    return Ok(ReturnValue::Value(Value::new_string(greeting)));
                }
                _ => {}
            }
        }
        Ok(ReturnValue::Value(Value::Nil))
    });

    // 3. Boot basic classes
    universe.load_class("Object")?;
    universe.load_class("Class")?;
    universe.load_class("Metaclass")?;
    universe.load_class("True")?;
    universe.load_class("False")?;
    universe.load_class("String")?;
    let sys_class = universe.load_class("System")?;

    let system_obj = Rc::new(RefCell::new(SomObject {
        class: sys_class.clone(),
        fields: Vec::new(),
    }));
    universe.set_global("system", Value::Object(system_obj.clone()));
    universe.set_global("nil", Value::Nil);
    universe.set_global("true", Value::Boolean(true));
    universe.set_global("false", Value::Boolean(false));

    // 4. Create an interpreter
    let interpreter = Interpreter::new(&universe);

    // 5. Test evaluate_snippet: run arbitrary SOM code snippet and get result
    // This snippet uses the custom primitive we just registered
    // Boot Hello class
    universe.load_class("Hello")?;
    let snippet_code = "Hello new rustGreeting: 'World'";
    let result = interpreter.evaluate_snippet(snippet_code)?;

    match result {
        Value::String(s) => {
            assert_eq!(*s.borrow(), "Hello from Rust, World!");
        }
        _ => panic!("Expected String value from snippet"),
    }

    // 6. Test calling SOM method from Rust via dispatch
    // We send asString to true
    let dispatch_result = interpreter.dispatch(Value::Boolean(true), "asString", vec![])?;
    match dispatch_result {
        Value::String(s) => {
            assert_eq!(*s.borrow(), "true");
        }
        _ => panic!("Expected String value from dispatch"),
    }

    Ok(())
}
