use crate::object::*;
use crate::universe::Universe;
use crate::interpreter::{Interpreter, ReturnValue};
use anyhow::{Result, anyhow};
use gc::Gc;
use num_bigint::BigInt;
use num_traits::{ToPrimitive, Zero, Signed};
use num_integer::Integer;


    fn extract_f32(val: &Value) -> f32 {
        match val {
            Value::Integer(i) => i.to_f32().unwrap_or(0.0),
            Value::Double(d) => *d as f32,
            _ => 0.0,
        }
    }

    fn extract_u8(val: &Value) -> u8 {
        match val {
            Value::Integer(i) => i.to_u8().unwrap_or(255),
            Value::Double(d) => *d as u8,
            _ => 255,
        }
    }

    // GUI Primitives
    fn gui_ctx_is_key_down(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::NativeHandle(ctx_ptr), Some(Value::String(key_str))) = (self_val, args.get(0)) {
            let ctx = unsafe { &*(*ctx_ptr as *const eframe::egui::Context) };

            let key = match key_str.borrow().as_str() {
                "Left" => Some(eframe::egui::Key::ArrowLeft),
                "Right" => Some(eframe::egui::Key::ArrowRight),
                "Up" => Some(eframe::egui::Key::ArrowUp),
                "Down" => Some(eframe::egui::Key::ArrowDown),
                "Space" => Some(eframe::egui::Key::Space),
                "A" => Some(eframe::egui::Key::A),
                "D" => Some(eframe::egui::Key::D),
                "W" => Some(eframe::egui::Key::W),
                "S" => Some(eframe::egui::Key::S),
                _ => None,
            };

            if let Some(k) = key {
                let is_down = ctx.input(|i| i.key_down(k));
                return Ok(ReturnValue::Value(Value::Boolean(is_down)));
            }
        }
        Ok(ReturnValue::Value(Value::Boolean(false)))
    }

    fn gui_ctx_request_repaint(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::NativeHandle(ctx_ptr) = self_val {
            let ctx = unsafe { &*(*ctx_ptr as *const eframe::egui::Context) };
            ctx.request_repaint();
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn sys_compile_method_in_class(_self_val: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Some(Value::String(code)), Some(Value::Class(cls))) = (args.get(0), args.get(1)) {
            let code_str = code.borrow().clone();
            let mut parser = crate::parser::Parser::new(&code_str);
            match parser.parse_method() {
                Ok(method_def) => {
                    match universe.assemble_method(method_def, cls.clone()) {
                        Ok(method) => return Ok(ReturnValue::Value(Value::Method(crate::object::som_ref(method)))),
                        Err(e) => return Ok(ReturnValue::Value(Value::new_string(format!("Error compiling: {}", e)))),
                    }
                }
                Err(e) => return Ok(ReturnValue::Value(Value::new_string(format!("Error parsing: {}", e)))),
            }
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn sys_install_method_in_class(_self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Some(Value::Method(method)), Some(Value::Class(cls))) = (args.get(0), args.get(1)) {
            let sig = method.borrow().signature.clone();
            cls.borrow_mut().methods.insert(sig.clone(), method.clone());
            if !cls.borrow().method_order.contains(&sig) {
                cls.borrow_mut().method_order.push(sig);
            }
            return Ok(ReturnValue::Value(Value::Boolean(true)));
        }
        Ok(ReturnValue::Value(Value::Boolean(false)))
    }

    fn gui_painter_fill_rect(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::NativeHandle(ui_ptr) = self_val {
            let ui = unsafe { &mut *(*ui_ptr as *mut eframe::egui::Ui) };

            let x = extract_f32(&args[0]); let y = extract_f32(&args[1]);
            let w = extract_f32(&args[2]); let h = extract_f32(&args[3]);
            let r = extract_u8(&args[4]);  let g = extract_u8(&args[5]);
            let b = extract_u8(&args[6]);  let a = extract_u8(&args[7]);

            let rect = eframe::egui::Rect::from_min_size(
                eframe::egui::pos2(x, y),
                eframe::egui::vec2(w, h)
            );
            let color = eframe::egui::Color32::from_rgba_unmultiplied(r, g, b, a);

            ui.painter().rect_filled(rect, 0.0, color);
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn gui_painter_fill_circle(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::NativeHandle(ui_ptr) = self_val {
            let ui = unsafe { &mut *(*ui_ptr as *mut eframe::egui::Ui) };

            let x = extract_f32(&args[0]); let y = extract_f32(&args[1]);
            let radius = extract_f32(&args[2]);
            let r = extract_u8(&args[3]); let g = extract_u8(&args[4]);
            let b = extract_u8(&args[5]); let a = extract_u8(&args[6]);

            let color = eframe::egui::Color32::from_rgba_unmultiplied(r, g, b, a);

            ui.painter().circle_filled(eframe::egui::pos2(x, y), radius, color);
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn gui_painter_draw_line(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::NativeHandle(ui_ptr) = self_val {
            let ui = unsafe { &mut *(*ui_ptr as *mut eframe::egui::Ui) };

            let x1 = extract_f32(&args[0]); let y1 = extract_f32(&args[1]);
            let x2 = extract_f32(&args[2]); let y2 = extract_f32(&args[3]);
            let width = extract_f32(&args[4]);
            let r = extract_u8(&args[5]); let g = extract_u8(&args[6]);
            let b = extract_u8(&args[7]); let a = extract_u8(&args[8]);

            let color = eframe::egui::Color32::from_rgba_unmultiplied(r, g, b, a);
            let stroke = eframe::egui::Stroke::new(width, color);

            ui.painter().line_segment([eframe::egui::pos2(x1, y1), eframe::egui::pos2(x2, y2)], stroke);
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    // GUI Primitives
    fn gui_window_do(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let (Value::NativeHandle(ctx_ptr), Some(Value::String(title)), Some(Value::Block(block))) = (self_val, args.get(0), args.get(1)) {
            let ctx = unsafe { &*(*ctx_ptr as *const eframe::egui::Context) };

            let _closed = false;
            let mut ret = Ok(ReturnValue::Value(Value::Nil));
            eframe::egui::Window::new(title.borrow().as_str()).show(ctx, |ui| {
                let ui_ptr = ui as *const eframe::egui::Ui as usize;
                match interpreter.run_block(block.clone(), vec![Value::NativeHandle(ui_ptr)]) {
                    Ok(res) => ret = Ok(res),
                    Err(e) => ret = Err(e),
                }
            });
            return ret;
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn gui_label(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::NativeHandle(ui_ptr), Some(Value::String(text))) = (self_val, args.get(0)) {
            let ui = unsafe { &mut *(*ui_ptr as *mut eframe::egui::Ui) };
            ui.label(text.borrow().as_str());
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn gui_button(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::NativeHandle(ui_ptr), Some(Value::String(text))) = (self_val, args.get(0)) {
            let ui = unsafe { &mut *(*ui_ptr as *mut eframe::egui::Ui) };
            let clicked = ui.button(text.borrow().as_str()).clicked();
            return Ok(ReturnValue::Value(Value::Boolean(clicked)));
        }
        Ok(ReturnValue::Value(Value::Boolean(false)))
    }

    fn gui_text_edit_multiline(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::NativeHandle(ui_ptr), Some(Value::String(text_ref))) = (self_val, args.get(0)) {
            let ui = unsafe { &mut *(*ui_ptr as *mut eframe::egui::Ui) };
            let mut s = text_ref.borrow().clone();
            ui.text_edit_multiline(&mut s);
            *text_ref.borrow_mut() = s;
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn gui_horizontal(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let (Value::NativeHandle(ui_ptr), Some(Value::Block(block))) = (self_val, args.get(0)) {
            let ui = unsafe { &mut *(*ui_ptr as *mut eframe::egui::Ui) };
            let mut ret = Ok(ReturnValue::Value(Value::Nil));
            ui.horizontal(|ui| {
                let inner_ui_ptr = ui as *const eframe::egui::Ui as usize;
                match interpreter.run_block(block.clone(), vec![Value::NativeHandle(inner_ui_ptr)]) {
                    Ok(res) => ret = Ok(res),
                    Err(e) => ret = Err(e),
                }
            });
            return ret;
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn gui_vertical(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let (Value::NativeHandle(ui_ptr), Some(Value::Block(block))) = (self_val, args.get(0)) {
            let ui = unsafe { &mut *(*ui_ptr as *mut eframe::egui::Ui) };
            let mut ret = Ok(ReturnValue::Value(Value::Nil));
            ui.vertical(|ui| {
                let inner_ui_ptr = ui as *const eframe::egui::Ui as usize;
                match interpreter.run_block(block.clone(), vec![Value::NativeHandle(inner_ui_ptr)]) {
                    Ok(res) => ret = Ok(res),
                    Err(e) => ret = Err(e),
                }
            });
            return ret;
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn sys_file_read_text(_self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(Value::String(path)) = args.get(0) {
            let path_str = path.borrow().clone();
            // Optional: verify it is in Tools/ or User/ ? Wait, reading can be from anywhere.
            if let Ok(content) = std::fs::read_to_string(path_str) {
                return Ok(ReturnValue::Value(Value::new_string(content)));
            }
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn sys_file_append_text(_self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        // signature: appendText:to:
        if let (Some(Value::String(content)), Some(Value::String(path))) = (args.get(0), args.get(1)) {
            let path_str = path.borrow().clone();
            let content_str = content.borrow().clone();

            // Check constraint: Must write to Tools/ or User/
            if path_str.contains("..") {
                return Ok(ReturnValue::Value(Value::new_string("Error: Path traversal not allowed".to_string())));
            }
            if !path_str.starts_with("Tools/") && !path_str.starts_with("User/") {
                return Ok(ReturnValue::Value(Value::new_string("Error: Can only write to Tools/ or User/ directory".to_string())));
            }

            use std::io::Write;
            let file_res = std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(&path_str);

            if let Ok(mut file) = file_res {
                if file.write_all(content_str.as_bytes()).is_ok() {
                    return Ok(ReturnValue::Value(Value::Boolean(true)));
                }
            } else {
                return Ok(ReturnValue::Value(Value::new_string("Error: Could not open file".to_string())));
            }
        }
        Ok(ReturnValue::Value(Value::Boolean(false)))
    }

    fn sys_file_write_text(_self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        // The signature is `writeText:to:`, so arg 0 is content, arg 1 is path
        if let (Some(Value::String(content)), Some(Value::String(path))) = (args.get(0), args.get(1)) {
            let path_str = path.borrow().clone();
            let content_str = content.borrow().clone();

            // Check constraint: Must write to Tools/ or User/
            if path_str.contains("..") {
                return Ok(ReturnValue::Value(Value::new_string("Error: Path traversal not allowed".to_string())));
            }
            if !path_str.starts_with("Tools/") && !path_str.starts_with("User/") {
                return Ok(ReturnValue::Value(Value::new_string("Error: Can only write to Tools/ or User/ directory".to_string())));
            }

            if std::fs::write(&path_str, content_str).is_ok() {
                return Ok(ReturnValue::Value(Value::Boolean(true)));
            }
        }
        Ok(ReturnValue::Value(Value::Boolean(false)))
    }

    fn sys_evaluate(_self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let Some(Value::String(code)) = args.get(0) {
            match interpreter.evaluate_snippet(code.borrow().as_str()) {
                Ok(val) => return Ok(ReturnValue::Value(val)),
                Err(e) => return Ok(ReturnValue::Value(Value::new_string(format!("Error: {}", e)))),
            }
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn sys_class_names(_self_val: &Value, _: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let globals = universe.globals.borrow();
        let mut names = Vec::new();
        for (name, val) in globals.iter() {
            if let Value::Class(_) = val {
                names.push(Value::Symbol(name.clone()));
            }
        }
        Ok(ReturnValue::Value(Value::Array(som_ref(names))))
    }

    // --- Async IO Primitives ---

    fn async_process_events(_: &Value, _: Vec<Value>, universe: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        let mut processed = false;
        loop {
            let event = {
                let rx = universe.async_rx.borrow();
                if let Ok(e) = rx.try_recv() {
                    Some(e)
                } else {
                    None
                }
            };
            if let Some(event) = event {
                processed = true;
                let blocks = universe.async_callbacks.borrow_mut().remove(&event.callback_id);
                if let Some(blocks) = blocks {
                    match event.result {
                        crate::universe::AsyncResult::SuccessNil => {
                            let _ = interpreter.dispatch(blocks.success_block, "value", vec![]);
                        }
                        crate::universe::AsyncResult::SuccessString(s) => {
                            let val = Value::new_string(s);
                            let _ = interpreter.dispatch(blocks.success_block, "value:", vec![val]);
                        }
                        crate::universe::AsyncResult::SuccessHandle(h) => {
                            let val = Value::Integer(num_bigint::BigInt::from(h));
                            let _ = interpreter.dispatch(blocks.success_block, "value:", vec![val]);
                        }
                        crate::universe::AsyncResult::Error(err) => {
                            let val = Value::new_string(err);
                            let _ = interpreter.dispatch(blocks.error_block, "value:", vec![val]);
                        }
                    }
                }
            } else {
                break;
            }
        }
        Ok(ReturnValue::Value(Value::Boolean(processed)))
    }

    fn async_file_read(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Some(Value::String(path)), Some(success_block), Some(error_block)) = (args.get(0), args.get(1), args.get(2)) {
            let path_str = path.borrow().clone();
            let cb_id = universe.next_async_id.get();
            universe.next_async_id.set(cb_id + 1);

            universe.async_callbacks.borrow_mut().insert(cb_id, crate::universe::AsyncCallbacks {
                success_block: success_block.clone(),
                error_block: error_block.clone(),
            });

            let tx = universe.async_tx.clone();
            universe.tokio_rt.spawn(async move {
                match tokio::fs::read_to_string(path_str).await {
                    Ok(s) => {
                        let _ = tx.send(crate::universe::AsyncEvent {
                            callback_id: cb_id,
                            result: crate::universe::AsyncResult::SuccessString(s),
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(crate::universe::AsyncEvent {
                            callback_id: cb_id,
                            result: crate::universe::AsyncResult::Error(e.to_string()),
                        });
                    }
                }
            });
            return Ok(ReturnValue::Value(Value::Nil));
        }
        Err(anyhow!("Invalid arguments for AsyncFile>>read:then:error:"))
    }

    fn async_file_write(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Some(Value::String(path)), Some(Value::String(content)), Some(success_block), Some(error_block)) = (args.get(0), args.get(1), args.get(2), args.get(3)) {
            let path_str = path.borrow().clone();
            let content_str = content.borrow().clone();
            let cb_id = universe.next_async_id.get();
            universe.next_async_id.set(cb_id + 1);

            universe.async_callbacks.borrow_mut().insert(cb_id, crate::universe::AsyncCallbacks {
                success_block: success_block.clone(),
                error_block: error_block.clone(),
            });

            let tx = universe.async_tx.clone();
            universe.tokio_rt.spawn(async move {
                match tokio::fs::write(path_str, content_str).await {
                    Ok(_) => {
                        let _ = tx.send(crate::universe::AsyncEvent {
                            callback_id: cb_id,
                            result: crate::universe::AsyncResult::SuccessNil,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(crate::universe::AsyncEvent {
                            callback_id: cb_id,
                            result: crate::universe::AsyncResult::Error(e.to_string()),
                        });
                    }
                }
            });
            return Ok(ReturnValue::Value(Value::Nil));
        }
        Err(anyhow!("Invalid arguments for AsyncFile>>write:content:then:error:"))
    }

    // A simple global registry for TCP handles since we cannot easily store them in SOM's GC memory.
    use tokio::sync::Mutex as TokioMutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    lazy_static::lazy_static! {
        static ref TCP_READERS: std::sync::Mutex<std::collections::HashMap<usize, std::sync::Arc<TokioMutex<tokio::net::tcp::OwnedReadHalf>>>> = std::sync::Mutex::new(std::collections::HashMap::new());
        static ref TCP_WRITERS: std::sync::Mutex<std::collections::HashMap<usize, std::sync::Arc<TokioMutex<tokio::net::tcp::OwnedWriteHalf>>>> = std::sync::Mutex::new(std::collections::HashMap::new());
        static ref TCP_LISTENERS: std::sync::Mutex<std::collections::HashMap<usize, std::sync::Arc<TokioMutex<tokio::net::TcpListener>>>> = std::sync::Mutex::new(std::collections::HashMap::new());
        static ref NEXT_TCP_HANDLE: AtomicUsize = AtomicUsize::new(1);
    }

    fn async_tcp_connect(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Some(Value::String(host)), Some(Value::Integer(port)), Some(success_block), Some(error_block)) = (args.get(0), args.get(1), args.get(2), args.get(3)) {
            let host_str = host.borrow().clone();
            use num_traits::ToPrimitive;
            let port_u16 = port.to_u16().unwrap_or(0);
            let addr = format!("{}:{}", host_str, port_u16);
            let cb_id = universe.next_async_id.get();
            universe.next_async_id.set(cb_id + 1);

            universe.async_callbacks.borrow_mut().insert(cb_id, crate::universe::AsyncCallbacks {
                success_block: success_block.clone(),
                error_block: error_block.clone(),
            });

            let tx = universe.async_tx.clone();
            universe.tokio_rt.spawn(async move {
                match tokio::net::TcpStream::connect(&addr).await {
                    Ok(stream) => {
                        let (rh, wh) = stream.into_split();
                        let handle = NEXT_TCP_HANDLE.fetch_add(1, Ordering::SeqCst);
                        TCP_READERS.lock().unwrap().insert(handle, std::sync::Arc::new(TokioMutex::new(rh)));
                        TCP_WRITERS.lock().unwrap().insert(handle, std::sync::Arc::new(TokioMutex::new(wh)));
                        let _ = tx.send(crate::universe::AsyncEvent {
                            callback_id: cb_id,
                            result: crate::universe::AsyncResult::SuccessHandle(handle),
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(crate::universe::AsyncEvent {
                            callback_id: cb_id,
                            result: crate::universe::AsyncResult::Error(e.to_string()),
                        });
                    }
                }
            });
            return Ok(ReturnValue::Value(Value::Nil));
        }
        Err(anyhow!("Invalid arguments for AsyncTcpSocket>>connectTo:port:then:error:"))
    }

    fn async_tcp_listen(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Some(Value::String(host)), Some(Value::Integer(port)), Some(success_block), Some(error_block)) = (args.get(0), args.get(1), args.get(2), args.get(3)) {
            let host_str = host.borrow().clone();
            use num_traits::ToPrimitive;
            let port_u16 = port.to_u16().unwrap_or(0);
            let addr = format!("{}:{}", host_str, port_u16);
            let cb_id = universe.next_async_id.get();
            universe.next_async_id.set(cb_id + 1);

            universe.async_callbacks.borrow_mut().insert(cb_id, crate::universe::AsyncCallbacks {
                success_block: success_block.clone(),
                error_block: error_block.clone(),
            });

            let tx = universe.async_tx.clone();
            universe.tokio_rt.spawn(async move {
                match tokio::net::TcpListener::bind(&addr).await {
                    Ok(listener) => {
                        let handle = NEXT_TCP_HANDLE.fetch_add(1, Ordering::SeqCst);
                        TCP_LISTENERS.lock().unwrap().insert(handle, std::sync::Arc::new(TokioMutex::new(listener)));
                        let _ = tx.send(crate::universe::AsyncEvent {
                            callback_id: cb_id,
                            result: crate::universe::AsyncResult::SuccessHandle(handle),
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(crate::universe::AsyncEvent {
                            callback_id: cb_id,
                            result: crate::universe::AsyncResult::Error(e.to_string()),
                        });
                    }
                }
            });
            return Ok(ReturnValue::Value(Value::Nil));
        }
        Err(anyhow!("Invalid arguments for AsyncTcpSocket>>listenOn:port:then:error:"))
    }

    fn async_tcp_accept(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Some(Value::Integer(handle)), Some(success_block), Some(error_block)) = (args.get(0), args.get(1), args.get(2)) {
            use num_traits::ToPrimitive;
            let h = handle.to_usize().unwrap_or(0);

            let listener_arc = {
                let listeners = TCP_LISTENERS.lock().unwrap();
                listeners.get(&h).cloned()
            };

            if let Some(listener_arc) = listener_arc {
                let cb_id = universe.next_async_id.get();
                universe.next_async_id.set(cb_id + 1);

                universe.async_callbacks.borrow_mut().insert(cb_id, crate::universe::AsyncCallbacks {
                    success_block: success_block.clone(),
                    error_block: error_block.clone(),
                });

                let tx = universe.async_tx.clone();
                universe.tokio_rt.spawn(async move {
                    let listener = listener_arc.lock().await;
                    match listener.accept().await {
                        Ok((stream, _)) => {
                            let (rh, wh) = stream.into_split();
                            let stream_handle = NEXT_TCP_HANDLE.fetch_add(1, Ordering::SeqCst);
                            TCP_READERS.lock().unwrap().insert(stream_handle, std::sync::Arc::new(TokioMutex::new(rh)));
                            TCP_WRITERS.lock().unwrap().insert(stream_handle, std::sync::Arc::new(TokioMutex::new(wh)));
                            let _ = tx.send(crate::universe::AsyncEvent {
                                callback_id: cb_id,
                                result: crate::universe::AsyncResult::SuccessHandle(stream_handle),
                            });
                        }
                        Err(e) => {
                            let _ = tx.send(crate::universe::AsyncEvent {
                                callback_id: cb_id,
                                result: crate::universe::AsyncResult::Error(e.to_string()),
                            });
                        }
                    }
                });
                return Ok(ReturnValue::Value(Value::Nil));
            } else {
                return Err(anyhow!("Invalid listener handle"));
            }
        }
        Err(anyhow!("Invalid arguments for AsyncTcpSocket>>accept:then:error:"))
    }

    fn async_tcp_read(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Some(Value::Integer(handle)), Some(success_block), Some(error_block)) = (args.get(0), args.get(1), args.get(2)) {
            use num_traits::ToPrimitive;
            let h = handle.to_usize().unwrap_or(0);

            let reader_arc = {
                let readers = TCP_READERS.lock().unwrap();
                readers.get(&h).cloned()
            };

            if let Some(reader_arc) = reader_arc {
                let cb_id = universe.next_async_id.get();
                universe.next_async_id.set(cb_id + 1);

                universe.async_callbacks.borrow_mut().insert(cb_id, crate::universe::AsyncCallbacks {
                    success_block: success_block.clone(),
                    error_block: error_block.clone(),
                });

                let tx = universe.async_tx.clone();
                universe.tokio_rt.spawn(async move {
                    use tokio::io::AsyncReadExt;
                    let mut reader = reader_arc.lock().await;
                    let mut buf = vec![0; 8192];
                    match reader.read(&mut buf).await {
                        Ok(0) => {
                            // EOF
                            let _ = tx.send(crate::universe::AsyncEvent {
                                callback_id: cb_id,
                                result: crate::universe::AsyncResult::SuccessString("".to_string()),
                            });
                        }
                        Ok(n) => {
                            let s = String::from_utf8_lossy(&buf[..n]).into_owned();
                            let _ = tx.send(crate::universe::AsyncEvent {
                                callback_id: cb_id,
                                result: crate::universe::AsyncResult::SuccessString(s),
                            });
                        }
                        Err(e) => {
                            let _ = tx.send(crate::universe::AsyncEvent {
                                callback_id: cb_id,
                                result: crate::universe::AsyncResult::Error(e.to_string()),
                            });
                        }
                    }
                });
                return Ok(ReturnValue::Value(Value::Nil));
            } else {
                return Err(anyhow!("Invalid reader handle"));
            }
        }
        Err(anyhow!("Invalid arguments for AsyncTcpSocket>>read:then:error:"))
    }

    fn async_tcp_write(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Some(Value::Integer(handle)), Some(Value::String(content)), Some(success_block), Some(error_block)) = (args.get(0), args.get(1), args.get(2), args.get(3)) {
            use num_traits::ToPrimitive;
            let h = handle.to_usize().unwrap_or(0);
            let content_str = content.borrow().clone();

            let writer_arc = {
                let writers = TCP_WRITERS.lock().unwrap();
                writers.get(&h).cloned()
            };

            if let Some(writer_arc) = writer_arc {
                let cb_id = universe.next_async_id.get();
                universe.next_async_id.set(cb_id + 1);

                universe.async_callbacks.borrow_mut().insert(cb_id, crate::universe::AsyncCallbacks {
                    success_block: success_block.clone(),
                    error_block: error_block.clone(),
                });

                let tx = universe.async_tx.clone();
                universe.tokio_rt.spawn(async move {
                    use tokio::io::AsyncWriteExt;
                    let mut writer = writer_arc.lock().await;
                    match writer.write_all(content_str.as_bytes()).await {
                        Ok(_) => {
                            let _ = tx.send(crate::universe::AsyncEvent {
                                callback_id: cb_id,
                                result: crate::universe::AsyncResult::SuccessNil,
                            });
                        }
                        Err(e) => {
                            let _ = tx.send(crate::universe::AsyncEvent {
                                callback_id: cb_id,
                                result: crate::universe::AsyncResult::Error(e.to_string()),
                            });
                        }
                    }
                });
                return Ok(ReturnValue::Value(Value::Nil));
            } else {
                return Err(anyhow!("Invalid writer handle"));
            }
        }
        Err(anyhow!("Invalid arguments for AsyncTcpSocket>>write:content:then:error:"))
    }

    fn async_tcp_close(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(Value::Integer(handle)) = args.get(0) {
            use num_traits::ToPrimitive;
            let h = handle.to_usize().unwrap_or(0);
            TCP_READERS.lock().unwrap().remove(&h);
            TCP_WRITERS.lock().unwrap().remove(&h);
            TCP_LISTENERS.lock().unwrap().remove(&h);
            return Ok(ReturnValue::Value(Value::Nil));
        }
        Err(anyhow!("Invalid arguments for AsyncTcpSocket>>close:"))
    }

    fn dbg_halt(_self_val: &Value, _: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        // Find the current active frame to pause on.
        // For primitive, we don't have direct access to the `Frame`, but we can set Stepping so next bytecode instruction pauses.
        *universe.vm_state.borrow_mut() = crate::universe::VmState::Stepping;
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn dbg_resume(_self_val: &Value, _: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        *universe.vm_state.borrow_mut() = crate::universe::VmState::Running;
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn dbg_step_into(_self_val: &Value, _: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        *universe.vm_state.borrow_mut() = crate::universe::VmState::Stepping;
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn dbg_current_frames(_self_val: &Value, _: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let frames = universe.active_frames.borrow();
        let mut ret = Vec::new();
        // Return frames reversed (top of stack first)
        for f in frames.iter().rev() {
            let name = f.borrow().method_name.clone();
            ret.push(Value::new_string(name));
        }
        Ok(ReturnValue::Value(Value::Array(crate::object::som_ref(ret))))
    }

pub fn get_primitives() -> std::collections::HashMap<String, fn(&Value, Vec<Value>, &Universe, &Interpreter) -> Result<ReturnValue>> {
    let mut prims: std::collections::HashMap<String, fn(&Value, Vec<Value>, &Universe, &Interpreter) -> Result<ReturnValue>> = std::collections::HashMap::new();

    fn sys_print_string(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(arg) = args.get(0) {
            match arg {
                Value::String(s) => print!("{}", s.borrow()),
                Value::Symbol(s) => print!("{}", s),
                _ => {}
            }
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn sys_print_newline(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        println!();
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn sys_global(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(arg) = args.get(0) {
            let name = match arg {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            if let Some(val) = universe.get_global(&name) {
                return Ok(ReturnValue::Value(val));
            }
        }
        Ok(ReturnValue::Value(Value::Nil))
    }
    
    fn sys_global_put(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Some(arg), Some(val)) = (args.get(0), args.get(1)) {
            let name = match arg {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            universe.set_global(&name, val.clone());
            return Ok(ReturnValue::Value(val.clone()));
        }
        Ok(ReturnValue::Value(Value::Nil))
    }
    
    fn sys_has_global(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(arg) = args.get(0) {
            let name = match arg {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
            };
            return Ok(ReturnValue::Value(Value::Boolean(universe.get_global(&name).is_some())));
        }
        Ok(ReturnValue::Value(Value::Boolean(false)))
    }

    fn sys_exit(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(Value::Integer(code)) = args.get(0) {
            std::process::exit(code.to_i32().unwrap_or(0));
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn sys_load(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(arg) = args.get(0) {
            let name = match arg {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            match universe.load_class(&name) {
                Ok(cls) => return Ok(ReturnValue::Value(Value::Class(cls))),
                Err(_) => return Ok(ReturnValue::Value(Value::Nil)),
            }
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn sys_time(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
        Ok(ReturnValue::Value(Value::Integer(BigInt::from(now))))
    }

    fn sys_ticks(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        // Dummy implementation of ticks
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros();
        Ok(ReturnValue::Value(Value::Integer(BigInt::from(now))))
    }

    fn sys_full_gc(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        gc::force_collect();
        Ok(ReturnValue::Value(Value::Boolean(true)))
    }

    fn sys_serialize_format(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if args.len() != 2 {
            return Ok(ReturnValue::Value(Value::Nil));
        }
        let obj = &args[0];
        let format_val = &args[1];

        let format_str = match format_val {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Nil)),
        };

        if format_str == "json" {
            match crate::serialize::to_json(obj) {
                Ok(s) => Ok(ReturnValue::Value(Value::new_string(s))),
                Err(_) => Ok(ReturnValue::Value(Value::Nil)),
            }
        } else if format_str == "msgpack" {
            // We'd return a ByteArray here, but we can return it as a string of bytes for simplicity,
            // or an Array of integers. Let's return an array of integers representing the bytes.
            match crate::serialize::to_msgpack(obj) {
                Ok(bytes) => {
                    let elements = bytes.into_iter().map(|b| Value::Integer(num_bigint::BigInt::from(b))).collect();
                    Ok(ReturnValue::Value(Value::Array(crate::object::som_ref(elements))))
                }
                Err(_) => Ok(ReturnValue::Value(Value::Nil)),
            }
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn sys_deserialize_format(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if args.len() != 2 {
            return Ok(ReturnValue::Value(Value::Nil));
        }
        let data = &args[0];
        let format_val = &args[1];

        let format_str = match format_val {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Nil)),
        };

        if format_str == "json" {
            let json_str = match data {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };

            let ir: crate::serialize::SerializedValue = match serde_json::from_str(&json_str) {
                Ok(v) => v,
                Err(_) => return Ok(ReturnValue::Value(Value::Nil)),
            };

            let mut deserializer = crate::serialize::SomDeserializer::new(universe);
            match deserializer.deserialize(&ir) {
                Ok(val) => Ok(ReturnValue::Value(val)),
                Err(_) => Ok(ReturnValue::Value(Value::Nil)),
            }
        } else if format_str == "msgpack" {
            let bytes = match data {
                Value::Array(arr) => {
                    arr.borrow().iter().filter_map(|v| {
                        if let Value::Integer(i) = v {
                            use num_traits::ToPrimitive;
                            i.to_u8()
                        } else {
                            None
                        }
                    }).collect::<Vec<u8>>()
                }
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };

            let ir: crate::serialize::SerializedValue = match rmp_serde::from_slice(&bytes) {
                Ok(v) => v,
                Err(_) => return Ok(ReturnValue::Value(Value::Nil)),
            };

            let mut deserializer = crate::serialize::SomDeserializer::new(universe);
            match deserializer.deserialize(&ir) {
                Ok(val) => Ok(ReturnValue::Value(val)),
                Err(_) => Ok(ReturnValue::Value(Value::Nil)),
            }
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn sys_load_file(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(arg) = args.get(0) {
            let file_name = match arg {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            if let Ok(content) = std::fs::read_to_string(file_name) {
                return Ok(ReturnValue::Value(Value::new_string(content)));
            }
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn int_plus(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            Ok(ReturnValue::Value(Value::Integer(a + b)))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af + *b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_minus(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            Ok(ReturnValue::Value(Value::Integer(a - b)))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af - *b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_mul(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            Ok(ReturnValue::Value(Value::Integer(a * b)))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af * *b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_div(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            if b.is_zero() { return Err(anyhow!("Division by zero")); }
            Ok(ReturnValue::Value(Value::Integer(a.div_floor(b))))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af / *b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_mod(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            if b.is_zero() { return Err(anyhow!("Modulo by zero")); }
            Ok(ReturnValue::Value(Value::Integer(a.mod_floor(b))))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af % *b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_rem(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            if b.is_zero() { return Err(anyhow!("Modulo by zero")); }
            Ok(ReturnValue::Value(Value::Integer(a % b)))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af % *b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_float_div(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            if b.is_zero() { return Err(anyhow!("Division by zero")); }
            let af = a.to_f64().unwrap_or(0.0);
            let bf = b.to_f64().unwrap_or(1.0);
            Ok(ReturnValue::Value(Value::Double(af / bf)))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af / *b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_eq(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(arg)) = (self_val, args.get(0)) {
            match arg {
                Value::Integer(b) => Ok(ReturnValue::Value(Value::Boolean(a == b))),
                Value::Double(b) => Ok(ReturnValue::Value(Value::Boolean(a.to_f64().unwrap_or(0.0) == *b))),
                _ => Ok(ReturnValue::Value(Value::Boolean(false))),
            }
        } else {
            Ok(ReturnValue::Value(Value::Boolean(false)))
        }
    }

    fn int_lt(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(arg)) = (self_val, args.get(0)) {
            match arg {
                Value::Integer(b) => Ok(ReturnValue::Value(Value::Boolean(a < b))),
                Value::Double(b) => Ok(ReturnValue::Value(Value::Boolean(a.to_f64().unwrap_or(0.0) < *b))),
                _ => Ok(ReturnValue::Value(Value::Boolean(false))),
            }
        } else {
            Ok(ReturnValue::Value(Value::Boolean(false)))
        }
    }
    
    fn int_le(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(arg)) = (self_val, args.get(0)) {
            match arg {
                Value::Integer(b) => Ok(ReturnValue::Value(Value::Boolean(a <= b))),
                Value::Double(b) => Ok(ReturnValue::Value(Value::Boolean(a.to_f64().unwrap_or(0.0) <= *b))),
                _ => Ok(ReturnValue::Value(Value::Boolean(false))),
            }
        } else {
            Ok(ReturnValue::Value(Value::Boolean(false)))
        }
    }

    fn int_bit_and(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            Ok(ReturnValue::Value(Value::Integer(a & b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_bit_xor(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            Ok(ReturnValue::Value(Value::Integer(a ^ b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_shl(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            let shift = b.to_u32().unwrap_or(0);
            Ok(ReturnValue::Value(Value::Integer(a << shift)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_min(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            Ok(ReturnValue::Value(Value::Integer(a.clone().min(b.clone()))))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af.min(*b))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_max(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            Ok(ReturnValue::Value(Value::Integer(a.clone().max(b.clone()))))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af.max(*b))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_shr(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            let shift = b.to_u32().unwrap_or(0);
            let mask = BigInt::from(0xFFFFFFFFFFFFFFFFu64);
            let truncated = a & mask;
            let val_u64 = truncated.to_u64().unwrap_or(0);
            Ok(ReturnValue::Value(Value::Integer(BigInt::from(val_u64 >> shift))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_as_32bit_signed(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Integer(a) = self_val {
            let mask = BigInt::from(0xFFFFFFFFu64);
            let truncated = a & mask;
            let val_u32 = truncated.to_u32().unwrap_or(0);
            let val_i32 = val_u32 as i32;
            Ok(ReturnValue::Value(Value::Integer(BigInt::from(val_i32))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_as_32bit_unsigned(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Integer(a) = self_val {
            let mask = BigInt::from(0xFFFFFFFFu64);
            let truncated = a & mask;
            let val_u32 = truncated.to_u32().unwrap_or(0);
            Ok(ReturnValue::Value(Value::Integer(BigInt::from(val_u32))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_as_double(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Integer(a) = self_val {
            Ok(ReturnValue::Value(Value::Double(a.to_f64().unwrap_or(0.0))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_at_random(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Integer(a) = self_val {
            let limit = a.to_i64().unwrap_or(1);
            let rand_val = if limit > 0 { (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() % limit as u128) as i64 + 1 } else { 1 };
            Ok(ReturnValue::Value(Value::Integer(BigInt::from(rand_val))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_sqrt(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Integer(a) = self_val {
            let f = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Integer(BigInt::from(f.sqrt() as i64))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_from_string(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(arg) = args.get(0) {
            let s_owned: String;
            let s = match arg {
                Value::String(s) => {
                    s_owned = s.borrow().clone();
                    s_owned.as_str()
                },
                Value::Symbol(s) => s.as_str(),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            if let Ok(i) = s.parse::<BigInt>() {
                Ok(ReturnValue::Value(Value::Integer(i)))
            } else {
                Ok(ReturnValue::Value(Value::Nil))
            }
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_as_string(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Integer(i) = self_val {
            Ok(ReturnValue::Value(Value::new_string(i.to_string())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_round(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        Ok(ReturnValue::Value(self_val.clone()))
    }
    fn str_concat(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let s1 = match self_val {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Nil)),
        };
        if let Some(arg) = args.get(0) {
            let s2 = match arg {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            Ok(ReturnValue::Value(Value::new_string(format!("{}{}", s1, s2))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn str_len(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let len = match self_val {
            Value::String(s) => s.borrow().len(),
            Value::Symbol(s) => s.len(),
            _ => return Ok(ReturnValue::Value(Value::Nil)),
        };
        Ok(ReturnValue::Value(Value::Integer(BigInt::from(len))))
    }

    fn str_eq(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let s1 = match self_val {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
        };
        if let Some(arg) = args.get(0) {
            let s2 = match arg {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
            };
            Ok(ReturnValue::Value(Value::Boolean(s1 == s2)))
        } else {
            Ok(ReturnValue::Value(Value::Boolean(false)))
        }
    }

    fn obj_hashcode(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let h = match self_val {
            Value::Integer(i) => (i % BigInt::from(0x7FFFFFFF)).to_i64().unwrap_or(0),
            Value::Double(d) => d.to_bits() as i64,
            Value::String(s) => {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                s.borrow().hash(&mut hasher);
                hasher.finish() as i64
            }
            Value::Symbol(s) => {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                s.hash(&mut hasher);
                hasher.finish() as i64
            }
            Value::Boolean(b) => if *b { 1 } else { 0 },
            Value::Nil => 0,
            Value::Object(obj) => Gc::as_ptr(obj) as i64,
            Value::Class(cls) => Gc::as_ptr(cls) as i64,
            Value::Array(arr) => Gc::as_ptr(arr) as i64,
            Value::Method(m)  => Gc::as_ptr(m) as i64,
            Value::Block(b)   => Gc::as_ptr(b) as i64,
            Value::CompiledBlock(b) => Gc::as_ptr(b) as i64,
            Value::NativeHandle(h) => *h as i64,
        };
        Ok(ReturnValue::Value(Value::Integer(BigInt::from(h & 0x7FFFFFFF))))
    }

    fn obj_object_size(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        // Dummy implementation
        Ok(ReturnValue::Value(Value::Integer(BigInt::from(16))))
    }

    fn str_is_whitespace(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let s = match self_val {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
        };
        if s.is_empty() { return Ok(ReturnValue::Value(Value::Boolean(false))); }
        Ok(ReturnValue::Value(Value::Boolean(s.chars().all(|c| c.is_whitespace()))))
    }

    fn str_is_letters(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let s = match self_val {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
        };
        if s.is_empty() { return Ok(ReturnValue::Value(Value::Boolean(false))); }
        Ok(ReturnValue::Value(Value::Boolean(s.chars().all(|c| c.is_alphabetic()))))
    }

    fn str_is_digits(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let s = match self_val {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
        };
        if s.is_empty() { return Ok(ReturnValue::Value(Value::Boolean(false))); }
        Ok(ReturnValue::Value(Value::Boolean(s.chars().all(|c| c.is_ascii_digit()))))
    }

    fn str_as_symbol(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        match self_val {
            Value::String(s) => Ok(ReturnValue::Value(Value::Symbol(s.borrow().clone()))),
            Value::Symbol(s) => Ok(ReturnValue::Value(Value::Symbol(s.clone()))),
            _ => Ok(ReturnValue::Value(Value::Nil)),
        }
    }

    fn symbol_as_string(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        match self_val {
            Value::String(s) => Ok(ReturnValue::Value(Value::new_string(s.borrow().clone()))),
            Value::Symbol(s) => Ok(ReturnValue::Value(Value::new_string(s.clone()))),
            _ => Ok(ReturnValue::Value(Value::Nil)),
        }
    }

    fn arr_new(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(Value::Integer(len)) = args.get(0) {
            let l = len.to_usize().unwrap_or(0);
            Ok(ReturnValue::Value(Value::Array(som_ref(vec![Value::Nil; l]))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn str_substring(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let s = match self_val {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Nil)),
        };
        if let (Some(Value::Integer(start)), Some(Value::Integer(end))) = (args.get(0), args.get(1)) {
            let start_idx = start.to_usize().unwrap_or(1);
            let end_idx = end.to_usize().unwrap_or(0);
            if start_idx == 0 || end_idx > s.len() {
                return Ok(ReturnValue::Value(Value::Nil));
            }
            if end_idx < start_idx {
                return Ok(ReturnValue::Value(Value::new_string("".to_string())));
            }
            Ok(ReturnValue::Value(Value::new_string(s[start_idx-1..end_idx].to_string())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn arr_at(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Array(arr), Some(Value::Integer(idx))) = (self_val, args.get(0)) {
            let i = idx.to_usize().unwrap_or(0);
            Ok(ReturnValue::Value(arr.borrow().get(i - 1).cloned().unwrap_or(Value::Nil)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn arr_at_put(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Array(arr), Some(Value::Integer(idx)), Some(val)) = (self_val, args.get(0), args.get(1)) {
            let i = idx.to_usize().unwrap_or(0);
            arr.borrow_mut()[i - 1] = val.clone();
            Ok(ReturnValue::Value(val.clone()))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn arr_len(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Array(arr) = self_val {
            Ok(ReturnValue::Value(Value::Integer(BigInt::from(arr.borrow().len()))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn bool_if_true(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Boolean(true), Some(Value::Block(b))) = (self_val, args.get(0)) {
            interpreter.run_block(b.clone(), Vec::new())
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn bool_if_false(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Boolean(false), Some(Value::Block(b))) = (self_val, args.get(0)) {
            interpreter.run_block(b.clone(), Vec::new())
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn bool_if_true_if_false(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Boolean(b), Some(Value::Block(true_block)), Some(Value::Block(false_block))) = (self_val, args.get(0), args.get(1)) {
            if *b {
                interpreter.run_block(true_block.clone(), Vec::new())
            } else {
                interpreter.run_block(false_block.clone(), Vec::new())
            }
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn block_while_true(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Block(cond), Some(Value::Block(body))) = (self_val, args.get(0)) {
            loop {
                match interpreter.run_block(cond.clone(), Vec::new())? {
                    ReturnValue::Value(Value::Boolean(true)) => {
                        match interpreter.run_block(body.clone(), Vec::new())? {
                            ReturnValue::Restart => continue,
                            ReturnValue::NonLocalReturn(v, target) => return Ok(ReturnValue::NonLocalReturn(v, target)),
                            _ => {}
                        }
                    }
                    _ => break,
                }
            }
            Ok(ReturnValue::Value(Value::Nil))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn block_while_false(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Block(cond), Some(Value::Block(body))) = (self_val, args.get(0)) {
            loop {
                match interpreter.run_block(cond.clone(), Vec::new())? {
                    ReturnValue::Value(Value::Boolean(false)) => {
                        match interpreter.run_block(body.clone(), Vec::new())? {
                            ReturnValue::Restart => continue,
                            ReturnValue::NonLocalReturn(v, target) => return Ok(ReturnValue::NonLocalReturn(v, target)),
                            _ => {}
                        }
                    }
                    _ => break,
                }
            }
            Ok(ReturnValue::Value(Value::Nil))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn block_restart(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        Ok(ReturnValue::Restart)
    }

    fn block_value(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let Value::Block(b) = self_val {
            interpreter.run_block(b.clone(), args)
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn obj_perform(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let Some(arg) = args.get(0) {
            let selector = match arg {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            let mut perform_args = Vec::new();
            if args.len() > 1 {
                if let Value::Array(arr) = &args[1] {
                    perform_args.extend(arr.borrow().iter().cloned());
                } else {
                    perform_args.push(args[1].clone());
                }
            }
            interpreter.dispatch_internal(self_val.clone(), &selector, perform_args)
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn obj_perform_in_superclass(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let (Some(arg_sel), Some(Value::Class(cls))) = (args.get(0), args.get(1)) {
            let selector = match arg_sel {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            let method = interpreter.lookup_method(cls.clone(), &selector)?;
            interpreter.run_method_internal(method, self_val.clone(), Vec::new())
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn obj_inst_var_at(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Object(obj), Some(Value::Integer(idx))) = (self_val, args.get(0)) {
            let i = idx.to_usize().unwrap_or(0);
            if i > 0 && i <= obj.borrow().fields.len() {
                return Ok(ReturnValue::Value(obj.borrow().fields[i - 1].clone()));
            }
        } else if let (Value::Class(cls), Some(Value::Integer(idx))) = (self_val, args.get(0)) {
            let i = idx.to_usize().unwrap_or(0);
            if i > 0 && i <= cls.borrow().fields.len() {
                return Ok(ReturnValue::Value(cls.borrow().fields[i - 1].clone()));
            }
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn obj_inst_var_at_put(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Object(obj), Some(Value::Integer(idx)), Some(val)) = (self_val, args.get(0), args.get(1)) {
            let i = idx.to_usize().unwrap_or(0);
            if i > 0 && i <= obj.borrow().fields.len() {
                obj.borrow_mut().fields[i - 1] = val.clone();
                return Ok(ReturnValue::Value(val.clone()));
            }
        } else if let (Value::Class(cls), Some(Value::Integer(idx)), Some(val)) = (self_val, args.get(0), args.get(1)) {
            let i = idx.to_usize().unwrap_or(0);
            if i > 0 && i <= cls.borrow().fields.len() {
                cls.borrow_mut().fields[i - 1] = val.clone();
                return Ok(ReturnValue::Value(val.clone()));
            }
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn obj_class(self_val: &Value, _: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let cls_name = match self_val {
            Value::Integer(_) => "Integer",
            Value::String(_) => "String",
            Value::Boolean(true) => "True",
            Value::Boolean(false) => "False",
            Value::Nil => "Nil",
            Value::Double(_) => "Double",
            Value::Object(obj) => return Ok(ReturnValue::Value(Value::Class(obj.borrow().class.clone()))),
            Value::Array(_) => "Array",
            Value::Class(cls) => return Ok(ReturnValue::Value(Value::Class(cls.borrow().class.as_ref().unwrap().clone()))),
            Value::Block(b) => {
                let params = b.borrow().body.parameters.len();
                match params {
                    0 => "Block1",
                    1 => "Block2",
                    2 => "Block3",
                    _ => "Block", // Fallback for many arguments
                }
            }
            Value::CompiledBlock(b) => {
                let params = b.borrow().block.num_args;
                match params {
                    0 => "Block1",
                    1 => "Block2",
                    2 => "Block3",
                    _ => "Block", // Fallback for many arguments
                }
            }
            Value::Symbol(_) => "Symbol",
            Value::Method(_) => "Method",
            Value::NativeHandle(_) => "Object",
        };
        Ok(ReturnValue::Value(Value::Class(universe.load_class(cls_name)?)))
    }

    fn obj_eq(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(other) = args.get(0) {
            Ok(ReturnValue::Value(Value::Boolean(self_val == other)))
        } else {
            Ok(ReturnValue::Value(Value::Boolean(false)))
        }
    }

    fn class_new(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Class(cls) = self_val {
            let instance = som_ref(SomObject {
                class: cls.clone(),
                fields: vec![Value::Nil; cls.borrow().instance_fields.len()],
            });
            Ok(ReturnValue::Value(Value::Object(instance)))
        } else {
            Err(anyhow!("new can only be sent to classes"))
        }
    }

    fn class_name(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Class(cls) = self_val {
            Ok(ReturnValue::Value(Value::Symbol(cls.borrow().name.clone())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn class_superclass(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Class(cls) = self_val {
            match &cls.borrow().super_class {
                Some(sc) => Ok(ReturnValue::Value(Value::Class(sc.clone()))),
                None => Ok(ReturnValue::Value(Value::Nil)),
            }
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn class_fields(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Class(cls) = self_val {
            let fields: Vec<Value> = cls.borrow().instance_fields.iter()
                .map(|f| Value::Symbol(f.clone()))
                .collect();
            Ok(ReturnValue::Value(Value::Array(som_ref(fields))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn class_methods(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Class(cls) = self_val {
            let cls_ref = cls.borrow();
            let methods: Vec<Value> = cls_ref.method_order.iter()
                .map(|name| Value::Method(cls_ref.methods.get(name).unwrap().clone()))
                .collect();
            Ok(ReturnValue::Value(Value::Array(som_ref(methods))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn class_has_method(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Class(cls), Some(arg)) = (self_val, args.get(0)) {
            let selector = match arg {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
            };
            Ok(ReturnValue::Value(Value::Boolean(cls.borrow().methods.contains_key(&selector))))
        } else {
            Ok(ReturnValue::Value(Value::Boolean(false)))
        }
    }

    fn class_selectors(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Class(cls) = self_val {
            let selectors: Vec<Value> = cls.borrow().methods.keys()
                .map(|s| Value::Symbol(s.clone()))
                .collect();
            Ok(ReturnValue::Value(Value::Array(som_ref(selectors))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn obj_responds_to(self_val: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(arg) = args.get(0) {
            let selector = match arg {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
            };
            let mut cls_opt = match self_val {
                Value::Integer(_) => universe.get_global("Integer"),
                Value::Double(_) => universe.get_global("Double"),
                Value::String(_) => universe.get_global("String"),
                Value::Symbol(_) => universe.get_global("Symbol"),
                Value::Boolean(true) => universe.get_global("True"),
                Value::Boolean(false) => universe.get_global("False"),
                Value::Nil => universe.get_global("Nil"),
                Value::Array(_) => universe.get_global("Array"),
                Value::Block(_) => universe.get_global("Block"),
                Value::CompiledBlock(_) => universe.get_global("Block"),
                Value::Object(obj) => Some(Value::Class(obj.borrow().class.clone())),
                Value::Class(cls) => cls.borrow().class.as_ref().map(|mc| Value::Class(mc.clone())),
                Value::Method(_) => universe.get_global("Method"),
                Value::NativeHandle(_) => universe.get_global("Object"),
            };

            while let Some(Value::Class(cls)) = cls_opt {
                if cls.borrow().methods.contains_key(&selector) {
                    return Ok(ReturnValue::Value(Value::Boolean(true)));
                }
                cls_opt = cls.borrow().super_class.as_ref().map(|s| Value::Class(s.clone()));
            }
        }
        Ok(ReturnValue::Value(Value::Boolean(false)))
    }

    fn method_signature(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Method(m) = self_val {
            Ok(ReturnValue::Value(Value::Symbol(m.borrow().signature.clone())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn method_holder(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Method(m) = self_val {
            Ok(ReturnValue::Value(Value::Class(m.borrow().holder.clone())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn nil_as_string(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        Ok(ReturnValue::Value(Value::new_string("nil".to_string())))
    }

    fn double_plus(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(a), Some(arg)) = (self_val, args.get(0)) {
            let b = match arg {
                Value::Double(v) => *v,
                Value::Integer(v) => v.to_f64().unwrap_or(0.0),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            Ok(ReturnValue::Value(Value::Double(a + b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_minus(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(a), Some(arg)) = (self_val, args.get(0)) {
            let b = match arg {
                Value::Double(v) => *v,
                Value::Integer(v) => v.to_f64().unwrap_or(0.0),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            Ok(ReturnValue::Value(Value::Double(a - b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_mul(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(a), Some(arg)) = (self_val, args.get(0)) {
            let b = match arg {
                Value::Double(v) => *v,
                Value::Integer(v) => v.to_f64().unwrap_or(0.0),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            Ok(ReturnValue::Value(Value::Double(a * b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_float_div(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(a), Some(arg)) = (self_val, args.get(0)) {
            let b = match arg {
                Value::Double(v) => *v,
                Value::Integer(v) => v.to_f64().unwrap_or(0.0),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            Ok(ReturnValue::Value(Value::Double(a / b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_mod(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(a), Some(arg)) = (self_val, args.get(0)) {
            let b = match arg {
                Value::Double(v) => *v,
                Value::Integer(v) => v.to_f64().unwrap_or(0.0),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            Ok(ReturnValue::Value(Value::Double(a % b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_eq(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(a), Some(arg)) = (self_val, args.get(0)) {
            match arg {
                Value::Double(b) => Ok(ReturnValue::Value(Value::Boolean(a == b))),
                Value::Integer(b) => Ok(ReturnValue::Value(Value::Boolean(*a == b.to_f64().unwrap_or(0.0)))),
                _ => Ok(ReturnValue::Value(Value::Boolean(false))),
            }
        } else {
            Ok(ReturnValue::Value(Value::Boolean(false)))
        }
    }

    fn double_lt(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(a), Some(arg)) = (self_val, args.get(0)) {
            let b = match arg {
                Value::Double(v) => *v,
                Value::Integer(v) => v.to_f64().unwrap_or(0.0),
                _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
            };
            Ok(ReturnValue::Value(Value::Boolean(*a < b)))
        } else {
            Ok(ReturnValue::Value(Value::Boolean(false)))
        }
    }

    fn double_as_integer(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Double(a) = self_val {
            Ok(ReturnValue::Value(Value::Integer(BigInt::from(*a as i64))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_as_string(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Double(a) = self_val {
            Ok(ReturnValue::Value(Value::new_string(a.to_string())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_sqrt(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Double(a) = self_val {
            Ok(ReturnValue::Value(Value::Double(a.sqrt())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_round(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Double(a) = self_val {
            Ok(ReturnValue::Value(Value::Integer(BigInt::from(a.round() as i64))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_cos(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Double(a) = self_val {
            Ok(ReturnValue::Value(Value::Double(a.cos())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_sin(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Double(a) = self_val {
            Ok(ReturnValue::Value(Value::Double(a.sin())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_from_string(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(Value::String(s)) = args.get(0) {
            if let Ok(f) = s.borrow().parse::<f64>() {
                Ok(ReturnValue::Value(Value::Double(f)))
            } else {
                Ok(ReturnValue::Value(Value::Nil))
            }
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_pos_inf(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        Ok(ReturnValue::Value(Value::Double(f64::INFINITY)))
    }

    fn int_to_do(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(start), Some(other), Some(Value::Block(block))) = (self_val, args.get(0), args.get(1)) {
            let limit = match other {
                Value::Integer(i) => i.clone(),
                Value::Double(d) => BigInt::from(d.to_i64().unwrap_or(0)),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            let mut i = start.clone();
            while i <= limit {
                match interpreter.run_block(block.clone(), vec![Value::Integer(i.clone())])? {
                    ReturnValue::Restart => continue,
                    ReturnValue::NonLocalReturn(v, target) => return Ok(ReturnValue::NonLocalReturn(v, target)),
                    _ => {}
                }
                i += 1;
            }
            return Ok(ReturnValue::Value(self_val.clone()));
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn int_down_to_do(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(start), Some(other), Some(Value::Block(block))) = (self_val, args.get(0), args.get(1)) {
            let limit = match other {
                Value::Integer(i) => i.clone(),
                Value::Double(d) => BigInt::from(d.to_i64().unwrap_or(0)),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            let mut i = start.clone();
            while i >= limit {
                match interpreter.run_block(block.clone(), vec![Value::Integer(i.clone())])? {
                    ReturnValue::Restart => continue,
                    ReturnValue::NonLocalReturn(v, target) => return Ok(ReturnValue::NonLocalReturn(v, target)),
                    _ => {}
                }
                i -= 1;
            }
            return Ok(ReturnValue::Value(self_val.clone()));
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn double_to_do(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(start), Some(other), Some(Value::Block(block))) = (self_val, args.get(0), args.get(1)) {
            let limit = match other {
                Value::Integer(i) => i.to_f64().unwrap_or(0.0),
                Value::Double(d) => *d,
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            let mut i = *start;
            while i <= limit {
                match interpreter.run_block(block.clone(), vec![Value::Double(i)])? {
                    ReturnValue::Restart => continue,
                    ReturnValue::NonLocalReturn(v, target) => return Ok(ReturnValue::NonLocalReturn(v, target)),
                    _ => {}
                }
                i += 1.0;
            }
            return Ok(ReturnValue::Value(self_val.clone()));
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn double_down_to_do(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(start), Some(other), Some(Value::Block(block))) = (self_val, args.get(0), args.get(1)) {
            let limit = match other {
                Value::Integer(i) => i.to_f64().unwrap_or(0.0),
                Value::Double(d) => *d,
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            let mut i = *start;
            while i >= limit {
                match interpreter.run_block(block.clone(), vec![Value::Double(i)])? {
                    ReturnValue::Restart => continue,
                    ReturnValue::NonLocalReturn(v, target) => return Ok(ReturnValue::NonLocalReturn(v, target)),
                    _ => {}
                }
                i -= 1.0;
            }
            return Ok(ReturnValue::Value(self_val.clone()));
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn true_not(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        Ok(ReturnValue::Value(Value::Boolean(false)))
    }

    fn false_not(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        Ok(ReturnValue::Value(Value::Boolean(true)))
    }

    fn int_abs(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Integer(i) = self_val {
            Ok(ReturnValue::Value(Value::Integer(i.abs())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_abs(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Double(d) = self_val {
            Ok(ReturnValue::Value(Value::Double(d.abs())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    prims.insert("System>>global:".to_string(), sys_global);
    prims.insert("System>>global:put:".to_string(), sys_global_put);
    prims.insert("System>>hasGlobal:".to_string(), sys_has_global);
    prims.insert("System>>load:".to_string(), sys_load);
    prims.insert("System>>exit:".to_string(), sys_exit);
    prims.insert("System>>printString:".to_string(), sys_print_string);
    prims.insert("System>>printNewline".to_string(), sys_print_newline);
    prims.insert("System>>time".to_string(), sys_time);
    prims.insert("System>>ticks".to_string(), sys_ticks);
    prims.insert("System>>fullGC".to_string(), sys_full_gc);
    prims.insert("System>>loadFile:".to_string(), sys_load_file);

    prims.insert("Integer>>+".to_string(), int_plus);
    prims.insert("Integer>>-".to_string(), int_minus);
    prims.insert("Integer>>*".to_string(), int_mul);
    prims.insert("Integer>>/".to_string(), int_div);
    prims.insert("Integer>>//".to_string(), int_float_div);
    prims.insert("Integer>>%".to_string(), int_mod);
    prims.insert("Integer>>rem:".to_string(), int_rem);
    prims.insert("Integer>>min:".to_string(), int_min);
    prims.insert("Integer>>max:".to_string(), int_max);
    prims.insert("Integer>>=".to_string(), int_eq);
    prims.insert("Integer>><".to_string(), int_lt);
    prims.insert("Integer>><=".to_string(), int_le);
    prims.insert("Integer>>&".to_string(), int_bit_and);
    prims.insert("Integer>>bitXor:".to_string(), int_bit_xor);
    prims.insert("Integer>><<".to_string(), int_shl);
    prims.insert("Integer>>>>>".to_string(), int_shr);
    prims.insert("Integer>>sqrt".to_string(), int_sqrt);
    prims.insert("Integer>>as32BitSignedValue".to_string(), int_as_32bit_signed);
    prims.insert("Integer>>as32BitUnsignedValue".to_string(), int_as_32bit_unsigned);
    prims.insert("Integer>>asDouble".to_string(), int_as_double);
    prims.insert("Integer>>atRandom".to_string(), int_at_random);
    prims.insert("Integer class>>fromString:".to_string(), int_from_string);
    prims.insert("Integer>>asString".to_string(), int_as_string);
    prims.insert("Integer>>round".to_string(), int_round);
    prims.insert("Integer>>to:do:".to_string(), int_to_do);
    prims.insert("Integer>>downTo:do:".to_string(), int_down_to_do);
    prims.insert("Integer>>abs".to_string(), int_abs);

    prims.insert("String>>concatenate:".to_string(), str_concat);
    prims.insert("String>>length".to_string(), str_len);
    prims.insert("String>>=".to_string(), str_eq);
    prims.insert("String>>asSymbol".to_string(), str_as_symbol);
    prims.insert("String>>hashcode".to_string(), obj_hashcode);
    prims.insert("String>>isWhiteSpace".to_string(), str_is_whitespace);
    prims.insert("String>>isLetters".to_string(), str_is_letters);
    prims.insert("String>>isDigits".to_string(), str_is_digits);
    prims.insert("String>>primSubstringFrom:to:".to_string(), str_substring);

    prims.insert("Array>>at:".to_string(), arr_at);
    prims.insert("Array>>at:put:".to_string(), arr_at_put);
    prims.insert("Array>>length".to_string(), arr_len);
    prims.insert("Array class>>new:".to_string(), arr_new);

    prims.insert("True>>ifTrue:".to_string(), bool_if_true);
    prims.insert("False>>ifTrue:".to_string(), bool_if_true);
    prims.insert("False>>ifFalse:".to_string(), bool_if_false);
    prims.insert("True>>ifFalse:".to_string(), bool_if_false);
    prims.insert("Boolean>>ifTrue:ifFalse:".to_string(), bool_if_true_if_false);

    prims.insert("Block>>whileTrue:".to_string(), block_while_true);
    prims.insert("Block>>whileFalse:".to_string(), block_while_false);
    prims.insert("Block>>restart".to_string(), block_restart);
    prims.insert("Block>>value".to_string(), block_value);
    prims.insert("Block>>value:".to_string(), block_value);
    prims.insert("Block>>value:with:".to_string(), block_value);

    prims.insert("Object>>perform:".to_string(), obj_perform);
    prims.insert("Object>>perform:withArguments:".to_string(), obj_perform);
    prims.insert("Object>>perform:inSuperclass:".to_string(), obj_perform_in_superclass);
    prims.insert("Object>>instVarAt:".to_string(), obj_inst_var_at);
    prims.insert("Object>>instVarAt:put:".to_string(), obj_inst_var_at_put);
    prims.insert("Object>>class".to_string(), obj_class);
    prims.insert("Object>>==".to_string(), obj_eq);
    prims.insert("Object>>hashcode".to_string(), obj_hashcode);
    prims.insert("Object>>objectSize".to_string(), obj_object_size);

    prims.insert("Class>>new".to_string(), class_new);
    prims.insert("Class>>name".to_string(), class_name);
    prims.insert("Class>>superclass".to_string(), class_superclass);
    prims.insert("Class>>fields".to_string(), class_fields);
    prims.insert("Class>>methods".to_string(), class_methods);
    prims.insert("Class>>hasMethod:".to_string(), class_has_method);
    prims.insert("Class>>selectors".to_string(), class_selectors);

    prims.insert("Object>>respondsTo:".to_string(), obj_responds_to);

    prims.insert("Method>>signature".to_string(), method_signature);
    prims.insert("Method>>holder".to_string(), method_holder);
    prims.insert("Primitive>>signature".to_string(), method_signature);
    prims.insert("Primitive>>holder".to_string(), method_holder);

    prims.insert("Nil>>asString".to_string(), nil_as_string);

    prims.insert("Symbol>>asString".to_string(), symbol_as_string);

    prims.insert("Double>>+".to_string(), double_plus);
    prims.insert("Double>>-".to_string(), double_minus);
    prims.insert("Double>>*".to_string(), double_mul);
    prims.insert("Double>>//".to_string(), double_float_div);
    prims.insert("Double>>%".to_string(), double_mod);
    prims.insert("Double>>=".to_string(), double_eq);
    prims.insert("Double>><".to_string(), double_lt);
    prims.insert("Double>>asInteger".to_string(), double_as_integer);
    prims.insert("Double>>asString".to_string(), double_as_string);
    prims.insert("Double>>sqrt".to_string(), double_sqrt);
    prims.insert("Double>>round".to_string(), double_round);
    prims.insert("Double>>cos".to_string(), double_cos);
    prims.insert("Double>>sin".to_string(), double_sin);
    prims.insert("Double class>>fromString:".to_string(), double_from_string);
    prims.insert("Double class>>PositiveInfinity".to_string(), double_pos_inf);
    prims.insert("Double>>to:do:".to_string(), double_to_do);
    prims.insert("Double>>downTo:do:".to_string(), double_down_to_do);
    prims.insert("Double>>abs".to_string(), double_abs);

    prims.insert("True>>not".to_string(), true_not);
    prims.insert("False>>not".to_string(), false_not);


    prims.insert("EguiContext>>isKeyDown:".to_string(), gui_ctx_is_key_down);
    prims.insert("EguiContext>>requestRepaint".to_string(), gui_ctx_request_repaint);
    prims.insert("EguiPainter>>fillRectX:y:w:h:r:g:b:a:".to_string(), gui_painter_fill_rect);
    prims.insert("EguiPainter>>fillCircleX:y:radius:r:g:b:a:".to_string(), gui_painter_fill_circle);
    prims.insert("EguiPainter>>drawLineX1:y1:x2:y2:width:r:g:b:a:".to_string(), gui_painter_draw_line);
    prims.insert("EguiContext>>window:do:".to_string(), gui_window_do);
    prims.insert("EguiUi>>label:".to_string(), gui_label);
    prims.insert("EguiUi>>button:".to_string(), gui_button);
    prims.insert("EguiUi>>textEditMultiline:".to_string(), gui_text_edit_multiline);
    prims.insert("EguiUi>>primHorizontal:".to_string(), gui_horizontal);
    prims.insert("EguiUi>>primVertical:".to_string(), gui_vertical);
    prims.insert("System>>evaluate:".to_string(), sys_evaluate);
    prims.insert("System>>compileMethod:inClass:".to_string(), sys_compile_method_in_class);
    prims.insert("System>>installMethod:inClass:".to_string(), sys_install_method_in_class);
    prims.insert("System>>classNames".to_string(), sys_class_names);
    prims.insert("System>>serialize:format:".to_string(), sys_serialize_format);
    prims.insert("System>>deserialize:format:".to_string(), sys_deserialize_format);

    prims.insert("System>>serialize:format:".to_string(), sys_serialize_format);
    prims.insert("System>>deserialize:format:".to_string(), sys_deserialize_format);


    prims.insert("System>>readText:".to_string(), sys_file_read_text);
    prims.insert("System>>writeText:to:".to_string(), sys_file_write_text);
    prims.insert("System>>appendText:to:".to_string(), sys_file_append_text);


    prims.insert("AsyncIO class>>processEvents".to_string(), async_process_events);
    prims.insert("AsyncFile class>>read:then:error:".to_string(), async_file_read);
    prims.insert("AsyncFile class>>write:content:then:error:".to_string(), async_file_write);
    prims.insert("AsyncTcpSocket class>>primConnectTo:port:then:error:".to_string(), async_tcp_connect);
    prims.insert("AsyncTcpSocket class>>primListenOn:port:then:error:".to_string(), async_tcp_listen);
    prims.insert("AsyncTcpSocket class>>primAccept:then:error:".to_string(), async_tcp_accept);
    prims.insert("AsyncTcpSocket class>>read:then:error:".to_string(), async_tcp_read);
    prims.insert("AsyncTcpSocket class>>write:content:then:error:".to_string(), async_tcp_write);
    prims.insert("AsyncTcpSocket class>>close:".to_string(), async_tcp_close);

    prims.insert("Debugger>>resume".to_string(), dbg_resume);
    prims.insert("Debugger>>stepInto".to_string(), dbg_step_into);
    prims.insert("Debugger>>currentFrames".to_string(), dbg_current_frames);
    prims.insert("Debugger class>>halt".to_string(), dbg_halt);
    prims.insert("Object>>halt".to_string(), dbg_halt);

    prims
}