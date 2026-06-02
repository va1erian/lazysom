use crate::interpreter::{Interpreter, ReturnValue};
use crate::object::*;
use crate::universe::Universe;
use anyhow::{Result, anyhow};
use num_traits::ToPrimitive;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Mutex as TokioMutex;

lazy_static::lazy_static! {
    static ref TCP_READERS: std::sync::Mutex<std::collections::HashMap<usize, std::sync::Arc<TokioMutex<tokio::net::tcp::OwnedReadHalf>>>> = std::sync::Mutex::new(std::collections::HashMap::new());
    static ref TCP_WRITERS: std::sync::Mutex<std::collections::HashMap<usize, std::sync::Arc<TokioMutex<tokio::net::tcp::OwnedWriteHalf>>>> = std::sync::Mutex::new(std::collections::HashMap::new());
    static ref TCP_LISTENERS: std::sync::Mutex<std::collections::HashMap<usize, std::sync::Arc<TokioMutex<tokio::net::TcpListener>>>> = std::sync::Mutex::new(std::collections::HashMap::new());
    static ref NEXT_TCP_HANDLE: AtomicUsize = AtomicUsize::new(1);
}

pub fn register(prims: &mut HashMap<String, fn(&Value, Vec<Value>, &Universe, &Interpreter) -> Result<ReturnValue>>) {
    prims.insert("AsyncIO class>>processEvents".to_string(), async_process_events);
    prims.insert("AsyncFile class>>read:then:error:".to_string(), async_file_read);
    prims.insert("AsyncFile class>>write:content:then:error:".to_string(), async_file_write);
    prims.insert("AsyncTcpSocket class>>primConnectTo:port:then:error:".to_string(), async_tcp_connect);
    prims.insert("AsyncTcpSocket class>>primListenOn:port:then:error:".to_string(), async_tcp_listen);
    prims.insert("AsyncTcpSocket class>>primAccept:then:error:".to_string(), async_tcp_accept);
    prims.insert("AsyncTcpSocket class>>read:then:error:".to_string(), async_tcp_read);
    prims.insert("AsyncTcpSocket class>>write:content:then:error:".to_string(), async_tcp_write);
    prims.insert("AsyncTcpSocket class>>close:".to_string(), async_tcp_close);
}

fn async_process_events(_: &Value, _: Vec<Value>, universe: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    let mut processed = false;
    loop {
        let event = {
            let rx = universe.async_rx.borrow();
            rx.try_recv().ok()
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
        universe.async_callbacks.borrow_mut().insert(cb_id, crate::universe::AsyncCallbacks { success_block: success_block.clone(), error_block: error_block.clone() });
        let tx = universe.async_tx.clone();
        universe.tokio_rt.spawn(async move {
            match tokio::fs::read_to_string(path_str).await {
                Ok(s) => { let _ = tx.send(crate::universe::AsyncEvent { callback_id: cb_id, result: crate::universe::AsyncResult::SuccessString(s) }); }
                Err(e) => { let _ = tx.send(crate::universe::AsyncEvent { callback_id: cb_id, result: crate::universe::AsyncResult::Error(e.to_string()) }); }
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
        universe.async_callbacks.borrow_mut().insert(cb_id, crate::universe::AsyncCallbacks { success_block: success_block.clone(), error_block: error_block.clone() });
        let tx = universe.async_tx.clone();
        universe.tokio_rt.spawn(async move {
            match tokio::fs::write(path_str, content_str).await {
                Ok(_) => { let _ = tx.send(crate::universe::AsyncEvent { callback_id: cb_id, result: crate::universe::AsyncResult::SuccessNil }); }
                Err(e) => { let _ = tx.send(crate::universe::AsyncEvent { callback_id: cb_id, result: crate::universe::AsyncResult::Error(e.to_string()) }); }
            }
        });
        return Ok(ReturnValue::Value(Value::Nil));
    }
    Err(anyhow!("Invalid arguments for AsyncFile>>write:content:then:error:"))
}

fn async_tcp_connect(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(Value::String(host)), Some(Value::Integer(port)), Some(success_block), Some(error_block)) = (args.get(0), args.get(1), args.get(2), args.get(3)) {
        let host_str = host.borrow().clone();
        let port_u16 = port.to_u16().unwrap_or(0);
        let addr = format!("{}:{}", host_str, port_u16);
        let cb_id = universe.next_async_id.get();
        universe.next_async_id.set(cb_id + 1);
        universe.async_callbacks.borrow_mut().insert(cb_id, crate::universe::AsyncCallbacks { success_block: success_block.clone(), error_block: error_block.clone() });
        let tx = universe.async_tx.clone();
        universe.tokio_rt.spawn(async move {
            match tokio::net::TcpStream::connect(&addr).await {
                Ok(stream) => {
                    let (rh, wh) = stream.into_split();
                    let handle = NEXT_TCP_HANDLE.fetch_add(1, Ordering::SeqCst);
                    TCP_READERS.lock().unwrap().insert(handle, std::sync::Arc::new(TokioMutex::new(rh)));
                    TCP_WRITERS.lock().unwrap().insert(handle, std::sync::Arc::new(TokioMutex::new(wh)));
                    let _ = tx.send(crate::universe::AsyncEvent { callback_id: cb_id, result: crate::universe::AsyncResult::SuccessHandle(handle) });
                }
                Err(e) => { let _ = tx.send(crate::universe::AsyncEvent { callback_id: cb_id, result: crate::universe::AsyncResult::Error(e.to_string()) }); }
            }
        });
        return Ok(ReturnValue::Value(Value::Nil));
    }
    Err(anyhow!("Invalid arguments for AsyncTcpSocket>>connectTo:port:then:error:"))
}

fn async_tcp_listen(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(Value::String(host)), Some(Value::Integer(port)), Some(success_block), Some(error_block)) = (args.get(0), args.get(1), args.get(2), args.get(3)) {
        let host_str = host.borrow().clone();
        let port_u16 = port.to_u16().unwrap_or(0);
        let addr = format!("{}:{}", host_str, port_u16);
        let cb_id = universe.next_async_id.get();
        universe.next_async_id.set(cb_id + 1);
        universe.async_callbacks.borrow_mut().insert(cb_id, crate::universe::AsyncCallbacks { success_block: success_block.clone(), error_block: error_block.clone() });
        let tx = universe.async_tx.clone();
        universe.tokio_rt.spawn(async move {
            match tokio::net::TcpListener::bind(&addr).await {
                Ok(listener) => {
                    let handle = NEXT_TCP_HANDLE.fetch_add(1, Ordering::SeqCst);
                    TCP_LISTENERS.lock().unwrap().insert(handle, std::sync::Arc::new(TokioMutex::new(listener)));
                    let _ = tx.send(crate::universe::AsyncEvent { callback_id: cb_id, result: crate::universe::AsyncResult::SuccessHandle(handle) });
                }
                Err(e) => { let _ = tx.send(crate::universe::AsyncEvent { callback_id: cb_id, result: crate::universe::AsyncResult::Error(e.to_string()) }); }
            }
        });
        return Ok(ReturnValue::Value(Value::Nil));
    }
    Err(anyhow!("Invalid arguments for AsyncTcpSocket>>listenOn:port:then:error:"))
}

fn async_tcp_accept(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(Value::Integer(handle)), Some(success_block), Some(error_block)) = (args.get(0), args.get(1), args.get(2)) {
        let h = handle.to_usize().unwrap_or(0);
        let listener_arc = { let listeners = TCP_LISTENERS.lock().unwrap(); listeners.get(&h).cloned() };
        if let Some(listener_arc) = listener_arc {
            let cb_id = universe.next_async_id.get();
            universe.next_async_id.set(cb_id + 1);
            universe.async_callbacks.borrow_mut().insert(cb_id, crate::universe::AsyncCallbacks { success_block: success_block.clone(), error_block: error_block.clone() });
            let tx = universe.async_tx.clone();
            universe.tokio_rt.spawn(async move {
                let listener = listener_arc.lock().await;
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let (rh, wh) = stream.into_split();
                        let stream_handle = NEXT_TCP_HANDLE.fetch_add(1, Ordering::SeqCst);
                        TCP_READERS.lock().unwrap().insert(stream_handle, std::sync::Arc::new(TokioMutex::new(rh)));
                        TCP_WRITERS.lock().unwrap().insert(stream_handle, std::sync::Arc::new(TokioMutex::new(wh)));
                        let _ = tx.send(crate::universe::AsyncEvent { callback_id: cb_id, result: crate::universe::AsyncResult::SuccessHandle(stream_handle) });
                    }
                    Err(e) => { let _ = tx.send(crate::universe::AsyncEvent { callback_id: cb_id, result: crate::universe::AsyncResult::Error(e.to_string()) }); }
                }
            });
            return Ok(ReturnValue::Value(Value::Nil));
        } else { return Err(anyhow!("Invalid listener handle")); }
    }
    Err(anyhow!("Invalid arguments for AsyncTcpSocket>>accept:then:error:"))
}

fn async_tcp_read(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(Value::Integer(handle)), Some(success_block), Some(error_block)) = (args.get(0), args.get(1), args.get(2)) {
        let h = handle.to_usize().unwrap_or(0);
        let reader_arc = { let readers = TCP_READERS.lock().unwrap(); readers.get(&h).cloned() };
        if let Some(reader_arc) = reader_arc {
            let cb_id = universe.next_async_id.get();
            universe.next_async_id.set(cb_id + 1);
            universe.async_callbacks.borrow_mut().insert(cb_id, crate::universe::AsyncCallbacks { success_block: success_block.clone(), error_block: error_block.clone() });
            let tx = universe.async_tx.clone();
            universe.tokio_rt.spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut reader = reader_arc.lock().await;
                let mut buf = vec![0; 8192];
                match reader.read(&mut buf).await {
                    Ok(0) => { let _ = tx.send(crate::universe::AsyncEvent { callback_id: cb_id, result: crate::universe::AsyncResult::SuccessString("".to_string()) }); }
                    Ok(n) => {
                        let s = String::from_utf8_lossy(&buf[..n]).into_owned();
                        let _ = tx.send(crate::universe::AsyncEvent { callback_id: cb_id, result: crate::universe::AsyncResult::SuccessString(s) });
                    }
                    Err(e) => { let _ = tx.send(crate::universe::AsyncEvent { callback_id: cb_id, result: crate::universe::AsyncResult::Error(e.to_string()) }); }
                }
            });
            return Ok(ReturnValue::Value(Value::Nil));
        } else { return Err(anyhow!("Invalid reader handle")); }
    }
    Err(anyhow!("Invalid arguments for AsyncTcpSocket>>read:then:error:"))
}

fn async_tcp_write(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(Value::Integer(handle)), Some(Value::String(content)), Some(success_block), Some(error_block)) = (args.get(0), args.get(1), args.get(2), args.get(3)) {
        let h = handle.to_usize().unwrap_or(0);
        let content_str = content.borrow().clone();
        let writer_arc = { let writers = TCP_WRITERS.lock().unwrap(); writers.get(&h).cloned() };
        if let Some(writer_arc) = writer_arc {
            let cb_id = universe.next_async_id.get();
            universe.next_async_id.set(cb_id + 1);
            universe.async_callbacks.borrow_mut().insert(cb_id, crate::universe::AsyncCallbacks { success_block: success_block.clone(), error_block: error_block.clone() });
            let tx = universe.async_tx.clone();
            universe.tokio_rt.spawn(async move {
                use tokio::io::AsyncWriteExt;
                let mut writer = writer_arc.lock().await;
                match writer.write_all(content_str.as_bytes()).await {
                    Ok(_) => { let _ = tx.send(crate::universe::AsyncEvent { callback_id: cb_id, result: crate::universe::AsyncResult::SuccessNil }); }
                    Err(e) => { let _ = tx.send(crate::universe::AsyncEvent { callback_id: cb_id, result: crate::universe::AsyncResult::Error(e.to_string()) }); }
                }
            });
            return Ok(ReturnValue::Value(Value::Nil));
        } else { return Err(anyhow!("Invalid writer handle")); }
    }
    Err(anyhow!("Invalid arguments for AsyncTcpSocket>>write:content:then:error:"))
}

fn async_tcp_close(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(Value::Integer(handle)) = args.get(0) {
        let h = handle.to_usize().unwrap_or(0);
        TCP_READERS.lock().unwrap().remove(&h);
        TCP_WRITERS.lock().unwrap().remove(&h);
        TCP_LISTENERS.lock().unwrap().remove(&h);
        return Ok(ReturnValue::Value(Value::Nil));
    }
    Err(anyhow!("Invalid arguments for AsyncTcpSocket>>close:"))
}
