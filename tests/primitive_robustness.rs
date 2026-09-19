//! Fuzzes every registered primitive with a pool of "hostile" receivers and
//! arguments (wrong types, extreme values, empty/huge collections, garbage
//! native handles, missing/extra args) and asserts that none of them panics.
//!
//! Primitives are Rust `fn(&Value, Vec<Value>, &Universe, &Interpreter) ->
//! Result<ReturnValue>`. They must never panic — a panic in release mode
//! aborts the whole process (see `panic = "abort"` in Cargo.toml's release
//! profile) with no chance for the caller to recover, and a language-level
//! error should instead surface as `Err(anyhow!(...))`.
//!
//! A handful of primitives are intentionally skipped (see `is_skipped`
//! below) because calling them with garbage input is either meaningless
//! (they would legitimately exit the process, block, or need a real GUI
//! backing store) or relies on unavoidable unsafe raw-pointer casts into a
//! real `egui` frame that no amount of argument validation can make safe to
//! fuzz with a fake `NativeHandle`.

use anyhow::Result;
use lazysom::interpreter::Interpreter;
use lazysom::object::{som_ref, SomObject, Value};
use lazysom::primitives::get_primitives;
use lazysom::universe::Universe;
use num_bigint::BigInt;
use std::panic::{self, AssertUnwindSafe};
use std::str::FromStr;

/// Primitives that must not be called with arbitrary/garbage input:
/// - `System>>exit:` really calls `std::process::exit`, which would kill the
///   test runner itself.
/// - The `EguiContext`/`EguiUi`/`EguiPainter` primitives all reinterpret a
///   `NativeHandle`/`Object` as a raw pointer into a live `egui::Context` /
///   `egui::Ui` frame (`unsafe { &*(ptr as *const _) }`) with no way to
///   validate the pointer from the SOM side. Feeding them a fake handle
///   (e.g. `NativeHandle(12345)`) is not a panic - it is a real out-of-bounds
///   memory read that crashes the process outright (unlike a panic, this is
///   not something `catch_unwind` can protect against). These are only ever
///   safe to call with a pointer egui itself handed out for the current
///   frame, so they are excluded from this generic fuzz test rather than
///   given a false sense of safety.
fn is_skipped(selector: &str) -> bool {
    if selector == "System>>exit:" {
        return true;
    }
    if selector.starts_with("EguiContext>>")
        || selector.starts_with("EguiUi>>")
        || selector.starts_with("EguiPainter>>")
    {
        return true;
    }
    false
}

fn build_universe() -> Universe {
    let classpath = vec![
        std::path::PathBuf::from("SOM/Smalltalk"),
        std::path::PathBuf::from("SOM/TestSuite"),
    ];
    let universe = Universe::new(classpath);

    // Boot enough of the core hierarchy that `Object>>class`,
    // `respondsTo:`, etc. can resolve without erroring on missing globals.
    for cls in [
        "Object", "Class", "Metaclass", "True", "False", "Nil", "String", "Integer", "Double",
        "Array", "Block", "Symbol", "Method", "Primitive", "System",
    ] {
        let _ = universe.load_class(cls);
    }

    let sys_class = universe
        .load_class("System")
        .expect("System class must load");
    let system_obj = som_ref(SomObject {
        class: sys_class,
        fields: Vec::new(),
    });
    universe.set_global("system", Value::Object(system_obj));
    universe.set_global("nil", Value::Nil);
    universe.set_global("true", Value::Boolean(true));
    universe.set_global("false", Value::Boolean(false));

    universe
}

/// The pool of hostile receiver/argument values used for every primitive.
fn build_pool(universe: &Universe) -> Vec<Value> {
    let huge = BigInt::from_str("123456789012345678901234567890123456789012345678901234567890")
        .unwrap();
    let object_class = universe
        .load_class("Object")
        .expect("Object class must load for the fuzz pool");

    vec![
        Value::Nil,
        Value::Boolean(true),
        Value::Boolean(false),
        Value::Integer(BigInt::from(0)),
        Value::Integer(BigInt::from(-1)),
        Value::Integer(huge),
        Value::Double(f64::NAN),
        Value::Double(f64::INFINITY),
        Value::Double(-1.5),
        Value::new_string("".to_string()),
        Value::new_string("héllo".to_string()),
        Value::Symbol("aSymbol".to_string()),
        Value::Array(som_ref(Vec::new())),
        Value::Array(som_ref(vec![Value::Integer(BigInt::from(1)), Value::Integer(BigInt::from(2)), Value::Integer(BigInt::from(3))])),
        Value::Class(object_class),
        Value::NativeHandle(0),
        Value::NativeHandle(12345),
    ]
}

/// 0/1/2/3-argument combinations drawn from `pool`, covering "too few args"
/// (the empty case, exercised against every primitive regardless of its real
/// arity) as well as "too many args" for 0/1-arg primitives.
fn build_arg_variations(pool: &[Value]) -> Vec<Vec<Value>> {
    let mut variations = vec![Vec::new()];
    for v in pool {
        variations.push(vec![v.clone()]);
    }
    let n = pool.len();
    for i in 0..n {
        let j = (i + 1) % n;
        variations.push(vec![pool[i].clone(), pool[j].clone()]);
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let k = (i + 2) % n;
        variations.push(vec![pool[i].clone(), pool[j].clone(), pool[k].clone()]);
    }
    variations
}

#[test]
fn no_primitive_panics_on_hostile_input() -> Result<()> {
    let universe = build_universe();
    let interpreter = Interpreter::new(&universe);
    let pool = build_pool(&universe);
    let arg_variations = build_arg_variations(&pool);

    let primitives = get_primitives();
    let mut failures: Vec<String> = Vec::new();
    let mut calls: u64 = 0;

    // Panics from inside primitives would otherwise print their default
    // panic handler message for every single failure; silence that so the
    // final assertion message stays readable, restoring it before we exit.
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));

    for (selector, func) in primitives.iter() {
        if is_skipped(selector) {
            continue;
        }
        'receivers: for receiver in &pool {
            for args in &arg_variations {
                calls += 1;
                let result = panic::catch_unwind(AssertUnwindSafe(|| {
                    func(receiver, args.clone(), &universe, &interpreter)
                }));
                if result.is_err() {
                    failures.push(format!(
                        "{} (receiver={:?}, args={:?})",
                        selector, receiver, args
                    ));
                    // One confirmed panic for this selector is enough detail;
                    // move on to the next primitive instead of flooding the
                    // failure list with every variation that hits it.
                    continue 'receivers;
                }
            }
        }
    }

    panic::set_hook(default_hook);

    assert!(
        failures.is_empty(),
        "{} primitive call(s) panicked out of {} total calls across {} primitives.\nFailing Class>>selector (receiver, args):\n{}",
        failures.len(),
        calls,
        primitives.len(),
        failures.join("\n")
    );

    Ok(())
}
