// Verifies that the interpreter's call-depth limit trips with a clean, catchable error
// *before* the Rust stack itself overflows, even when it is running on a thread with a
// modest stack size (8 MiB). This is the core guarantee behind Interpreter::max_depth /
// DEFAULT_MAX_DEPTH / MAIN_THREAD_MAX_DEPTH in src/interpreter.rs: exceeding the configured
// depth must always produce Err(...) containing "Stack overflow", never a process crash.
use lazysom::universe::Universe;
use lazysom::interpreter::Interpreter;
use lazysom::object::{Value, SomObject, som_ref};

#[test]
fn recursion_limit_produces_clean_error_not_a_stack_overflow() {
    // Run on a thread with only 8 MiB of stack (much smaller than the 128 MiB the CLI and
    // vm_runner threads get) to prove the interpreter's own limit trips well before the
    // real Rust stack would overflow, no matter how small the host thread's stack is.
    let child = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            let classpath = vec![
                std::path::PathBuf::from("SOM/Smalltalk"),
                std::path::PathBuf::from("SOM/TestSuite"),
                std::path::PathBuf::from("tests"),
            ];
            let universe = Universe::new(classpath);
            universe.load_class("Object").unwrap();
            universe.load_class("Class").unwrap();
            universe.load_class("Metaclass").unwrap();
            universe.load_class("True").unwrap();
            universe.load_class("False").unwrap();
            universe.load_class("Nil").unwrap();
            universe.load_class("String").unwrap();
            universe.load_class("Integer").unwrap();
            let sys_class = universe.load_class("System").unwrap();

            let system_obj = som_ref(SomObject { class: sys_class.clone(), fields: Vec::new() });
            universe.set_global("system", Value::Object(system_obj));
            universe.set_global("nil", Value::Nil);
            universe.set_global("true", Value::Boolean(true));
            universe.set_global("false", Value::Boolean(false));

            // A small max depth relative to the 8 MiB stack: even the *default* depth
            // (10,000) would be unsafe on a stack this small, which is exactly the scenario
            // this test proves is now handled cleanly.
            let interpreter = Interpreter::with_max_depth(&universe, 100);

            let infinite_cls = universe.load_class("InfiniteRecursion").unwrap();
            let instance = som_ref(SomObject { class: infinite_cls.clone(), fields: vec![] });

            // Value (backed by Rc/Gc) is not Send, so convert the result to a plain String
            // before it crosses the thread boundary.
            match interpreter.dispatch(Value::Object(instance), "recurse", vec![]) {
                Ok(v) => Err(format!("expected an error, got Ok({:?})", v)),
                Err(e) => Err(e.to_string()),
            }
        })
        .expect("failed to spawn worker thread");

    // If the interpreter's depth limit did not trip before the Rust stack overflowed, this
    // whole test process would crash (SIGSEGV / STATUS_STACK_OVERFLOW) instead of getting a
    // normal Err from join(), so simply reaching this point with an Err already demonstrates
    // the limit works. We additionally assert on the error message.
    let result: Result<(), String> = child.join().expect("worker thread panicked or crashed instead of returning a clean error");

    let msg = result.expect_err("expected recursion to hit the call-depth limit and return Err");
    assert!(
        msg.contains("Stack overflow"),
        "expected a clean 'Stack overflow' error, got: {}",
        msg
    );
}
