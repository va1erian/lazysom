// Integration tests for Phase 0 of the roadmap: primitives that used to be declared
// `= primitive` in a .som file with no Rust implementation, and silently returned nil.
//
// See docs/ROADMAP.md "Phase 0 -- Make the reference interpreter honest".

use lazysom::interpreter::Interpreter;
use lazysom::object::{som_ref, SomObject, Value};
use lazysom::universe::Universe;
use anyhow::Result;
use num_bigint::BigInt;

/// `Value::String` compares by pointer identity (see object.rs), so string-valued
/// results need to be unwrapped to compare their contents.
fn as_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.borrow().clone(),
        Value::Symbol(s) => s.clone(),
        other => panic!("expected a String/Symbol value, got: {:?}", other),
    }
}

/// Boots a Universe with the core classes + a `system` global, mirroring what
/// main.rs does before running a program.
fn boot_universe(extra_classpath: &[&str]) -> Result<Universe> {
    let mut classpath = vec![
        std::path::PathBuf::from("SOM/Smalltalk"),
        std::path::PathBuf::from("SOM/TestSuite"),
        std::path::PathBuf::from("Tools"),
        std::path::PathBuf::from("tests"),
    ];
    classpath.extend(extra_classpath.iter().map(std::path::PathBuf::from));
    let universe = Universe::new(classpath);

    universe.load_class("Object")?;
    universe.load_class("Class")?;
    universe.load_class("Metaclass")?;
    let sys_class = universe.load_class("System")?;

    let system_obj = som_ref(SomObject {
        class: sys_class.clone(),
        fields: Vec::new(),
    });
    universe.set_global("system", Value::Object(system_obj));
    universe.set_global("nil", Value::Nil);
    universe.set_global("true", Value::Boolean(true));
    universe.set_global("false", Value::Boolean(false));

    Ok(universe)
}

#[test]
fn doesnotunderstand_stops_execution_with_an_error_naming_the_selector() -> Result<()> {
    let universe = boot_universe(&[])?;
    universe.load_class("Integer")?;
    let interpreter = Interpreter::new(&universe);

    let result = interpreter.dispatch(Value::Integer(BigInt::from(3)), "fooBar", vec![]);
    let err = result.expect_err("3 fooBar must not silently return a value");
    let msg = format!("{}", err);
    assert!(
        msg.contains("fooBar"),
        "error message should mention the missing selector, got: {}",
        msg
    );

    Ok(())
}

#[test]
fn class_with_unregistered_primitive_fails_to_load() -> Result<()> {
    let universe = boot_universe(&[])?;

    let result = universe.load_class("UnknownPrimitiveFixture");
    let err = result.expect_err("a class declaring an unimplemented primitive must fail to load");
    let msg = format!("{}", err);
    assert!(
        msg.contains("UnknownPrimitiveFixture"),
        "error should name the class, got: {}",
        msg
    );
    assert!(
        msg.contains("bogusPrimitive"),
        "error should name the selector, got: {}",
        msg
    );

    Ok(())
}

#[test]
fn object_error_returns_an_err_instead_of_exiting_the_process() -> Result<()> {
    // Upstream SOM's Object>>error: calls `system exit: 1`; it is overridden natively so the
    // error unwinds as an Err (this test would kill the whole test binary otherwise).
    let universe = boot_universe(&[])?;
    universe.load_class("Integer")?;
    let interpreter = Interpreter::new(&universe);

    let result = interpreter.dispatch(Value::Integer(BigInt::from(3)), "error:", vec![Value::new_string("boom".to_string())]);
    let err = result.expect_err("error: must return an Err that unwinds the program");
    assert!(format!("{}", err).contains("boom"));

    Ok(())
}

#[test]
fn system_error_print_and_println_and_stack_trace_do_not_panic() -> Result<()> {
    let universe = boot_universe(&[])?;
    let interpreter = Interpreter::new(&universe);
    let system_val = universe.get_global("system").unwrap();

    interpreter.dispatch(system_val.clone(), "errorPrint:", vec![Value::new_string("hi".to_string())])?;
    interpreter.dispatch(system_val.clone(), "errorPrintln:", vec![Value::new_string("hi".to_string())])?;
    interpreter.dispatch(system_val, "printStackTrace", vec![])?;

    Ok(())
}

#[test]
fn object_inspect_returns_receiver_without_panicking() -> Result<()> {
    let universe = boot_universe(&[])?;
    universe.load_class("PrimFixtureFields")?;
    let interpreter = Interpreter::new(&universe);

    let obj = interpreter.dispatch(
        interpreter.evaluate_snippet("PrimFixtureFields new")?,
        "value:",
        vec![Value::Integer(BigInt::from(7))],
    )?;

    let inspected = interpreter.dispatch(obj.clone(), "inspect", vec![])?;
    assert_eq!(inspected, obj);

    Ok(())
}

#[test]
fn object_inst_var_named_reads_a_field_by_name() -> Result<()> {
    let universe = boot_universe(&[])?;
    universe.load_class("PrimFixtureFields")?;
    let interpreter = Interpreter::new(&universe);

    let obj = interpreter.evaluate_snippet("[ | o | o := PrimFixtureFields new. o value: 42. o ] value")?;

    let result = interpreter.dispatch(obj.clone(), "instVarNamed:", vec![Value::Symbol("value".to_string())])?;
    assert_eq!(result, Value::Integer(BigInt::from(42)));

    let missing = interpreter.dispatch(obj, "instVarNamed:", vec![Value::Symbol("nope".to_string())]);
    assert!(missing.is_err(), "instVarNamed: on an unknown field must be an error");

    Ok(())
}

#[test]
fn object_perform_with_arguments_in_superclass_dispatches_to_the_superclass_method() -> Result<()> {
    let universe = boot_universe(&[])?;
    universe.load_class("PrimFixtureBase")?;
    let derived_cls = universe.load_class("PrimFixtureDerived")?;
    let base_cls = universe.load_class("PrimFixtureBase")?;
    let interpreter = Interpreter::new(&universe);

    let instance = som_ref(lazysom::object::SomObject {
        class: derived_cls.clone(),
        fields: Vec::new(),
    });

    // Normal dispatch picks the overridden method.
    let normal = interpreter.dispatch(Value::Object(instance.clone()), "describe", vec![])?;
    assert_eq!(as_string(&normal), "derived");

    // perform:withArguments:inSuperclass: must reach into the superclass instead.
    let args_array = Value::Array(som_ref(Vec::new()));
    let via_super = interpreter.dispatch(
        Value::Object(instance),
        "perform:withArguments:inSuperclass:",
        vec![Value::Symbol("describe".to_string()), args_array, Value::Class(base_cls)],
    )?;
    assert_eq!(as_string(&via_super), "base");

    Ok(())
}

#[test]
fn method_invoke_on_with_runs_a_user_method_on_another_receiver() -> Result<()> {
    let universe = boot_universe(&[])?;
    let cls = universe.load_class("PrimFixtureFields")?;
    let interpreter = Interpreter::new(&universe);

    let method = cls.borrow().methods.get("value").expect("PrimFixtureFields>>value should exist").clone();

    let obj = interpreter.evaluate_snippet("[ | o | o := PrimFixtureFields new. o value: 99. o ] value")?;

    let result = interpreter.dispatch(
        Value::Method(method),
        "invokeOn:with:",
        vec![obj, Value::Array(som_ref(Vec::new()))],
    )?;
    assert_eq!(result, Value::Integer(BigInt::from(99)));

    Ok(())
}

#[test]
fn primitive_invoke_on_with_runs_a_native_primitive_on_another_receiver() -> Result<()> {
    let universe = boot_universe(&[])?;
    universe.load_class("Integer")?;
    let int_cls = universe.load_class("Integer")?;
    let interpreter = Interpreter::new(&universe);

    let plus_method = int_cls.borrow().methods.get("+").expect("Integer>>+ should exist").clone();
    assert!(plus_method.borrow().is_primitive());

    let result = interpreter.dispatch(
        Value::Method(plus_method),
        "invokeOn:with:",
        vec![Value::Integer(BigInt::from(3)), Value::Array(som_ref(vec![Value::Integer(BigInt::from(4))]))],
    )?;
    assert_eq!(result, Value::Integer(BigInt::from(7)));

    Ok(())
}

#[test]
fn file_class_read_text_shares_the_system_read_text_implementation() -> Result<()> {
    let universe = boot_universe(&[])?;
    universe.load_class("String")?;
    let interpreter = Interpreter::new(&universe);

    let content = interpreter.dispatch(
        universe.get_global("File").unwrap(),
        "readText:",
        vec![Value::new_string("Tools/ide_config.txt".to_string())],
    )?;

    let expected = std::fs::read_to_string("Tools/ide_config.txt").unwrap();
    assert_eq!(as_string(&content), expected);

    Ok(())
}
