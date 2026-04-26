use lazysom::universe::Universe;
use lazysom::interpreter::{Interpreter, ReturnValue};
use lazysom::object::{Value, SomObject, som_ref};
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

    let system_obj = som_ref(SomObject {
        class: sys_class.clone(),
        fields: Vec::new(),
    });
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

#[test]
fn test_serialization_primitives() -> Result<()> {
    let classpath = vec![
        std::path::PathBuf::from("SOM/Smalltalk"),
        std::path::PathBuf::from("SOM/TestSuite"),
    ];
    let universe = Universe::new(classpath);
    let interpreter = Interpreter::new(&universe);
    universe.load_class("System")?;
    universe.load_class("Array")?;
    universe.load_class("String")?;
    universe.load_class("Integer")?;

    let system_obj = som_ref(SomObject {
        class: universe.load_class("System")?,
        fields: Vec::new(),
    });
    universe.set_global("system", Value::Object(system_obj.clone()));

    let snippet_code = "[ | arr serialized deserialized |
                arr := Array new: 3.
        arr at: 1 put: 42.
        arr at: 2 put: 'hello'.
        arr at: 3 put: true.

        serialized := system serialize: arr format: 'json'.
        deserialized := system deserialize: serialized format: 'json'.

        deserialized ] value
    ";
    let result = interpreter.evaluate_snippet(snippet_code)?;

    match result {
        Value::Array(arr) => {
            let arr_ref = arr.borrow();
            assert_eq!(arr_ref.len(), 3);

            match &arr_ref[0] {
                Value::Integer(i) => assert_eq!(i.to_string(), "42"),
                _ => panic!("Expected Integer 42"),
            }

            match &arr_ref[1] {
                Value::String(s) => assert_eq!(*s.borrow(), "hello"),
                _ => panic!("Expected String 'hello'"),
            }

            match &arr_ref[2] {
                Value::Boolean(b) => assert_eq!(*b, true),
                _ => panic!("Expected Boolean true"),
            }
        }
        _ => panic!("Expected Array value from snippet"),
    }

    Ok(())
}

#[test]
fn test_serialization_circular_reference() -> Result<()> {
    let classpath = vec![
        std::path::PathBuf::from("SOM/Smalltalk"),
        std::path::PathBuf::from("SOM/TestSuite"),
    ];
    let universe = Universe::new(classpath);
    let interpreter = Interpreter::new(&universe);
    universe.load_class("System")?;
    universe.load_class("Array")?;

    let system_obj = som_ref(SomObject {
        class: universe.load_class("System")?,
        fields: Vec::new(),
    });
    universe.set_global("system", Value::Object(system_obj.clone()));

    let snippet_code = "[ | arr1 arr2 serialized deserialized |
                arr1 := Array new: 1.
        arr2 := Array new: 1.

        arr1 at: 1 put: arr2.
        arr2 at: 1 put: arr1.

        serialized := system serialize: arr1 format: 'json'.
        deserialized := system deserialize: serialized format: 'json'.

        deserialized ] value
    ";
    let result = interpreter.evaluate_snippet(snippet_code)?;

    match result {
        Value::Array(arr) => {
            let arr_ref = arr.borrow();
            assert_eq!(arr_ref.len(), 1);

            match &arr_ref[0] {
                Value::Array(inner_arr) => {
                    let inner_arr_ref = inner_arr.borrow();
                    assert_eq!(inner_arr_ref.len(), 1);

                    // Verify the circular reference logic works
                    // The inner array's first element should point back to the outer array
                    match &inner_arr_ref[0] {
                        Value::Array(circular_arr) => {
                             // Compare underlying pointers
                             assert_eq!(gc::Gc::as_ptr(&arr), gc::Gc::as_ptr(circular_arr));
                        }
                        _ => panic!("Expected Array (circular reference)"),
                    }
                }
                _ => panic!("Expected Array"),
            }
        }
        _ => panic!("Expected Array value from snippet"),
    }

    Ok(())
}

#[test]
fn test_serialization_shared_reference() -> Result<()> {
    let classpath = vec![
        std::path::PathBuf::from("SOM/Smalltalk"),
        std::path::PathBuf::from("SOM/TestSuite"),
    ];
    let universe = Universe::new(classpath);
    let interpreter = Interpreter::new(&universe);
    universe.load_class("System")?;
    universe.load_class("Array")?;
    universe.load_class("String")?;

    let system_obj = som_ref(SomObject {
        class: universe.load_class("System")?,
        fields: Vec::new(),
    });
    universe.set_global("system", Value::Object(system_obj.clone()));

    let snippet_code = "[ | arr str serialized deserialized |
                arr := Array new: 2.
        str := 'shared'.

        arr at: 1 put: str.
        arr at: 2 put: str.

        serialized := system serialize: arr format: 'json'.
        deserialized := system deserialize: serialized format: 'json'.

        deserialized ] value
    ";
    let result = interpreter.evaluate_snippet(snippet_code)?;

    match result {
        Value::Array(arr) => {
            let arr_ref = arr.borrow();
            assert_eq!(arr_ref.len(), 2);

            if let (Value::String(s1), Value::String(s2)) = (&arr_ref[0], &arr_ref[1]) {
                // Assert that they are the exact same pointer, preserving shared reference identity
                assert_eq!(gc::Gc::as_ptr(s1), gc::Gc::as_ptr(s2));
            } else {
                panic!("Expected String elements");
            }
        }
        _ => panic!("Expected Array value from snippet"),
    }

    Ok(())
}

#[test]
fn test_serialization_msgpack() -> Result<()> {
    let classpath = vec![
        std::path::PathBuf::from("SOM/Smalltalk"),
        std::path::PathBuf::from("SOM/TestSuite"),
    ];
    let universe = Universe::new(classpath);
    let interpreter = Interpreter::new(&universe);
    universe.load_class("System")?;
    universe.load_class("Array")?;
    universe.load_class("String")?;
    universe.load_class("Integer")?;

    let system_obj = som_ref(SomObject {
        class: universe.load_class("System")?,
        fields: Vec::new(),
    });
    universe.set_global("system", Value::Object(system_obj.clone()));

    let snippet_code = "[ | arr serialized deserialized |
                arr := Array new: 3.
        arr at: 1 put: 42.
        arr at: 2 put: 'hello'.
        arr at: 3 put: true.

        serialized := system serialize: arr format: 'msgpack'.
        deserialized := system deserialize: serialized format: 'msgpack'.

        deserialized ] value
    ";
    let result = interpreter.evaluate_snippet(snippet_code)?;

    match result {
        Value::Array(arr) => {
            let arr_ref = arr.borrow();
            assert_eq!(arr_ref.len(), 3);

            match &arr_ref[0] {
                Value::Integer(i) => assert_eq!(i.to_string(), "42"),
                _ => panic!("Expected Integer 42"),
            }

            match &arr_ref[1] {
                Value::String(s) => assert_eq!(*s.borrow(), "hello"),
                _ => panic!("Expected String 'hello'"),
            }

            match &arr_ref[2] {
                Value::Boolean(b) => assert_eq!(*b, true),
                _ => panic!("Expected Boolean true"),
            }
        }
        _ => panic!("Expected Array value from snippet"),
    }

    Ok(())
}
