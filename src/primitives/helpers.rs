use crate::object::*;
use crate::universe::Universe;
use std::cell::Cell;

thread_local! {
    /// Increments with each gui_scroll_area call within a frame.
    /// Reset at the start of every frame from gui.rs so IDs are
    /// unique per call but stable across frames for the same layout.
    pub static SCROLL_ID_COUNTER: Cell<u64> = const { Cell::new(0) };
}

pub fn reset_scroll_id_counter() {
    SCROLL_ID_COUNTER.with(|c| c.set(0));
}

pub fn extract_f32(val: &Value) -> f32 {
    match val {
        Value::Integer(i) => {
            use num_traits::ToPrimitive;
            i.to_f32().unwrap_or(0.0)
        }
        Value::Double(d) => *d as f32,
        _ => 0.0,
    }
}

pub fn extract_u8(val: &Value) -> u8 {
    match val {
        Value::Integer(i) => {
            use num_traits::ToPrimitive;
            i.to_u8().unwrap_or(255)
        }
        Value::Double(d) => *d as u8,
        _ => 255,
    }
}

pub fn extract_handle(val: &Value) -> Option<usize> {
    match val {
        Value::NativeHandle(h) => Some(*h),
        Value::Object(obj) => {
            let obj_ref = obj.borrow();
            if let Some(Value::NativeHandle(h)) = obj_ref.fields.get(0) {
                Some(*h)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn wrap_ui_handle(ui_ptr: usize, universe: &Universe) -> Value {
    match universe.load_class("EguiUi") {
        Ok(ui_class) => {
            let instance = som_ref(SomObject {
                class: ui_class.clone(),
                fields: vec![Value::NativeHandle(ui_ptr)],
            });
            Value::Object(instance)
        }
        Err(_) => Value::NativeHandle(ui_ptr),
    }
}
