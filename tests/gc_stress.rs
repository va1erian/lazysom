/// GC stress tests for lazySOM.
///
/// These tests exercise the tracing GC (gc::Gc<gc::GcCell<T>>) integrated
/// into the SOM object model.  They verify:
///
///  1. Basic allocation & collection
///  2. Cycle collection (the killer case for plain Rc)
///  3. Deep reference chains
///  4. Array / field mutation under pressure
///  5. Cross-object cycles (class ↔ metaclass ↔ superclass)
///  6. Many simultaneous live objects
///  7. Mixed live / dead object graph under forced collections
///
/// Run with:  cargo test gc_stress -- --nocapture

#[cfg(test)]
mod gc_stress {
    use gc::{Gc, GcCell, force_collect};
    use lazysom::object::{
        som_ref, Activation, SomBlock, SomClass, SomMethod, SomObject, Value,
    };
    use std::collections::HashMap;

    // -----------------------------------------------------------------------
    // helpers
    // -----------------------------------------------------------------------

    /// Build a minimal, self-contained SomClass (no super-class, no metaclass).
    fn make_class(name: &str) -> Gc<GcCell<SomClass>> {
        som_ref(SomClass {
            name: name.to_string(),
            class: None,
            super_class: None,
            instance_fields: vec!["x".to_string()],
            fields: vec![Value::Nil],
            methods: HashMap::new(),
            method_order: Vec::new(),
        })
    }

    /// Create an instance of a class.
    fn make_object(cls: Gc<GcCell<SomClass>>) -> Value {
        let num_fields = cls.borrow().instance_fields.len();
        Value::Object(som_ref(SomObject {
            class: cls,
            fields: vec![Value::Nil; num_fields],
        }))
    }

    // -----------------------------------------------------------------------
    // 1. Basic allocation and collection
    // -----------------------------------------------------------------------
    #[test]
    fn test_basic_alloc_collect() {
        // Allocate a bunch of objects and immediately drop them, then force a GC.
        for _ in 0..1_000 {
            let cls = make_class("Temp");
            let _obj = make_object(cls);
            // _obj and cls go out of scope here
        }
        force_collect();
        // If we get here without panicking the GC handled it.
    }

    // -----------------------------------------------------------------------
    // 2. Cycle collection — the primary motivation for using Gc over Rc
    //
    //   A → B → A  (simple two-node cycle)
    //
    // With plain Rc this would leak.  With Gc it should be collected.
    // -----------------------------------------------------------------------
    #[test]
    fn test_cycle_two_objects() {
        {
            let cls = make_class("Node");

            // obj_a.fields[0] will point to obj_b; obj_b.fields[0] will point back.
            let obj_a = som_ref(SomObject {
                class: cls.clone(),
                fields: vec![Value::Nil],
            });
            let obj_b = som_ref(SomObject {
                class: cls.clone(),
                fields: vec![Value::Nil],
            });

            obj_a.borrow_mut().fields[0] = Value::Object(obj_b.clone());
            obj_b.borrow_mut().fields[0] = Value::Object(obj_a.clone());

            // Both objects are referenced only by each other — cycle.
        } // drop local handles

        force_collect(); // must not hang or panic
    }

    // -----------------------------------------------------------------------
    // 3. Self-referential object
    // -----------------------------------------------------------------------
    #[test]
    fn test_self_cycle() {
        {
            let cls = make_class("Self");
            let obj = som_ref(SomObject {
                class: cls,
                fields: vec![Value::Nil],
            });
            obj.borrow_mut().fields[0] = Value::Object(obj.clone()); // points to itself
        }
        force_collect();
    }

    // -----------------------------------------------------------------------
    // 4. Closure (SomBlock) retaining activation creates a cycle
    //    when a block captures itself through its context.
    // -----------------------------------------------------------------------
    #[test]
    fn test_block_activation_cycle() {
        use lazysom::ast::{Block, Expression, Literal};
        use num_bigint::BigInt;

        let block_ast = Block {
            parameters: vec![],
            locals: vec![],
            body: vec![Expression::Literal(Literal::Integer(BigInt::from(42)))],
        };

        {
            let activation = som_ref(Activation {
                holder: None,
                self_val: Value::Nil,
                args: HashMap::new(),
                locals: HashMap::new(),
                parent: None,
                is_active: true,
            });

            let block_val = Value::Block(som_ref(SomBlock {
                body: block_ast.clone(),
                context: Some(activation.clone()),
            }));

            // Store the block inside the activation so it retains itself.
            activation.borrow_mut().locals.insert("blk".to_string(), block_val);
        }
        force_collect();
    }

    // -----------------------------------------------------------------------
    // 5. Deep chain of parent activations (linked list of frames)
    // -----------------------------------------------------------------------
    #[test]
    fn test_deep_activation_chain() {
        const DEPTH: usize = 500;

        {
            let mut parent: Option<Gc<GcCell<Activation>>> = None;
            for i in 0..DEPTH {
                let act = som_ref(Activation {
                    holder: None,
                    self_val: Value::Integer(num_bigint::BigInt::from(i as i64)),
                    args: HashMap::new(),
                    locals: HashMap::new(),
                    parent: parent.clone(),
                    is_active: true,
                });
                parent = Some(act);
            }
            // parent chain goes out of scope
        }
        force_collect();
    }

    // -----------------------------------------------------------------------
    // 6. Class ↔ metaclass cycle (the real SOM bootstrap creates these)
    // -----------------------------------------------------------------------
    #[test]
    fn test_class_metaclass_cycle() {
        {
            let cls = som_ref(SomClass {
                name: "MyClass".to_string(),
                class: None,
                super_class: None,
                instance_fields: vec![],
                fields: vec![],
                methods: HashMap::new(),
                method_order: Vec::new(),
            });
            let metacls = som_ref(SomClass {
                name: "MyClass class".to_string(),
                class: Some(cls.clone()), // metaclass points to class
                super_class: None,
                instance_fields: vec![],
                fields: vec![],
                methods: HashMap::new(),
                method_order: Vec::new(),
            });
            cls.borrow_mut().class = Some(metacls.clone()); // class points to metaclass → cycle
        }
        force_collect();
    }

    // -----------------------------------------------------------------------
    // 7. Superclass chain with shared metaclass pointers
    //    Object → nil  (root)
    //    MyClass → Object
    //    SubClass → MyClass
    // -----------------------------------------------------------------------
    #[test]
    fn test_class_hierarchy_collection() {
        {
            let object_cls = make_class("Object");
            let my_cls = som_ref(SomClass {
                name: "MyClass".to_string(),
                class: None,
                super_class: Some(object_cls.clone()),
                instance_fields: vec![],
                fields: vec![],
                methods: HashMap::new(),
                method_order: Vec::new(),
            });
            let sub_cls = som_ref(SomClass {
                name: "SubClass".to_string(),
                class: None,
                super_class: Some(my_cls.clone()),
                instance_fields: vec![],
                fields: vec![],
                methods: HashMap::new(),
                method_order: Vec::new(),
            });

            // Build ten instances of the subclass
            for _ in 0..10 {
                let _inst = make_object(sub_cls.clone());
            }
        }
        force_collect();
    }

    // -----------------------------------------------------------------------
    // 8. Large array of objects, many collected after partial release
    // -----------------------------------------------------------------------
    #[test]
    fn test_large_array_partial_release() {
        const N: usize = 2_000;
        let cls = make_class("Item");
        let mut live: Vec<Value> = Vec::new();

        for i in 0..N {
            let mut fields = vec![Value::Nil];
            fields[0] = Value::Integer(num_bigint::BigInt::from(i as i64));
            let obj = Value::Object(som_ref(SomObject {
                class: cls.clone(),
                fields,
            }));
            live.push(obj);
        }

        // Drop the odd-indexed ones — half the objects become unreachable.
        live.retain(|v| {
            if let Value::Object(o) = v {
                if let Value::Integer(n) = &o.borrow().fields[0] {
                    return n.to_string().parse::<usize>().unwrap_or(0) % 2 == 0;
                }
            }
            true
        });

        force_collect();

        // The remaining (even-indexed) objects must still be intact.
        for (idx, v) in live.iter().enumerate() {
            if let Value::Object(o) = v {
                assert!(
                    matches!(&o.borrow().fields[0], Value::Integer(_)),
                    "item {} has lost its integer field after GC",
                    idx
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // 9. Stress: many small cycles interleaved with forced collections
    // -----------------------------------------------------------------------
    #[test]
    fn test_many_cycles_with_intermittent_gc() {
        let cls = make_class("Node");

        for round in 0..50 {
            // Create 40 two-node cycles per round
            for _ in 0..(40usize) {
                let a = som_ref(SomObject {
                    class: cls.clone(),
                    fields: vec![Value::Nil],
                });
                let b = som_ref(SomObject {
                    class: cls.clone(),
                    fields: vec![Value::Nil],
                });
                a.borrow_mut().fields[0] = Value::Object(b.clone());
                b.borrow_mut().fields[0] = Value::Object(a.clone());
                // a and b go out of scope here forming a dead cycle
            }
            if round % 10 == 0 {
                force_collect();
            }
        }
        force_collect();
    }

    // -----------------------------------------------------------------------
    // 10. Value::Array holding Values — verify array elements are traced
    // -----------------------------------------------------------------------
    #[test]
    fn test_array_elements_traced() {
        let cls = make_class("El");
        let arr_ref = som_ref(Vec::<Value>::new());

        {
            // Fill array with objects
            for i in 0..100 {
                let obj = som_ref(SomObject {
                    class: cls.clone(),
                    fields: vec![Value::Integer(num_bigint::BigInt::from(i))],
                });
                arr_ref.borrow_mut().push(Value::Object(obj));
            }
        }

        // Force a collection — array is still live via arr_ref.
        force_collect();

        // All elements must still be alive.
        let arr = arr_ref.borrow();
        assert_eq!(arr.len(), 100);
        for (i, v) in arr.iter().enumerate() {
            if let Value::Object(o) = v {
                if let Value::Integer(n) = &o.borrow().fields[0] {
                    assert_eq!(n, &num_bigint::BigInt::from(i as i64));
                } else {
                    panic!("element {} lost its integer field", i);
                }
            } else {
                panic!("element {} is not an Object", i);
            }
        }
    }

    // -----------------------------------------------------------------------
    // 11. Method stored in class referencing its holder (cycle)
    // -----------------------------------------------------------------------
    #[test]
    fn test_method_holder_cycle() {
        use lazysom::ast::{Block, Expression, Literal};
        use lazysom::object::MethodBody;
        use num_bigint::BigInt;

        {
            let cls = make_class("WithMethod");
            let method = som_ref(SomMethod {
                name: "test".to_string(),
                signature: "test".to_string(),
                holder: cls.clone(), // method → class
                parameters: vec![],
                body: MethodBody::Ast(Block {
                    parameters: vec![],
                    locals: vec![],
                    body: vec![Expression::Literal(Literal::Integer(BigInt::from(1)))],
                }),
            });
            // class → method → class  (cycle through the holder field)
            cls.borrow_mut()
                .methods
                .insert("test".to_string(), method.clone());
            cls.borrow_mut().method_order.push("test".to_string());
        }
        force_collect();
    }

    // -----------------------------------------------------------------------
    // 12. Mixed live / dead graph — live objects must survive
    // -----------------------------------------------------------------------
    #[test]
    fn test_live_objects_survive_gc() {
        let cls = make_class("Survivor");
        let sentinel = Value::Integer(num_bigint::BigInt::from(0xDEAD_BEEFi64));

        let live_obj = som_ref(SomObject {
            class: cls.clone(),
            fields: vec![sentinel.clone()],
        });

        // Create many dead cycles around it
        for _ in 0..200 {
            let a = som_ref(SomObject {
                class: cls.clone(),
                fields: vec![Value::Nil],
            });
            a.borrow_mut().fields[0] = Value::Object(a.clone()); // self-cycle
        }

        force_collect();

        // live_obj must still hold the sentinel value
        if let Value::Integer(n) = &live_obj.borrow().fields[0] {
            assert_eq!(*n, num_bigint::BigInt::from(0xDEAD_BEEFi64));
        } else {
            panic!("live object lost its field after GC");
        }
    }
}
