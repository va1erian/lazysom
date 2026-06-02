use crate::interpreter::{Interpreter, ReturnValue};
use crate::object::*;
use crate::universe::Universe;
use crate::primitives::helpers::*;
use anyhow::Result;
use num_traits::ToPrimitive;
use std::collections::HashMap;

pub fn register(prims: &mut HashMap<String, fn(&Value, Vec<Value>, &Universe, &Interpreter) -> Result<ReturnValue>>) {
    prims.insert("EguiContext>>isKeyDown:".to_string(), gui_ctx_is_key_down);
    prims.insert("EguiContext>>requestRepaint".to_string(), gui_ctx_request_repaint);
    prims.insert("EguiPainter>>fillRectX:y:w:h:r:g:b:a:".to_string(), gui_painter_fill_rect);
    prims.insert("EguiPainter>>fillCircleX:y:radius:r:g:b:a:".to_string(), gui_painter_fill_circle);
    prims.insert("EguiPainter>>drawLineX1:y1:x2:y2:width:r:g:b:a:".to_string(), gui_painter_draw_line);
    prims.insert("EguiContext>>window:do:".to_string(), gui_window_do);
    prims.insert("EguiUi>>label:".to_string(), gui_label);
    prims.insert("EguiUi>>button:".to_string(), gui_button);
    prims.insert("EguiUi>>textEdit:".to_string(), gui_text_edit);
    prims.insert("EguiUi>>textEditMultiline:".to_string(), gui_text_edit_multiline);
    prims.insert("EguiUi>>primHorizontal:".to_string(), gui_horizontal);
    prims.insert("EguiUi>>primVertical:".to_string(), gui_vertical);
    prims.insert("EguiUi>>primScrollArea:".to_string(), gui_scroll_area);
    prims.insert("EguiUi>>primColumns:do:".to_string(), gui_columns);
    prims.insert("EguiUi>>heading:".to_string(), gui_heading);
    prims.insert("EguiUi>>separator".to_string(), gui_separator);
    prims.insert("EguiUi>>primGroup:".to_string(), gui_group);
    prims.insert("EguiUi>>primComboBox:selected:options:do:".to_string(), gui_combo_box);
}

fn gui_ctx_is_key_down(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ctx_ptr), Some(Value::String(key_str))) = (extract_handle(self_val), args.get(0)) {
        let ctx = unsafe { &*(ctx_ptr as *const eframe::egui::Context) };
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
    if let Some(ctx_ptr) = extract_handle(self_val) {
        let ctx = unsafe { &*(ctx_ptr as *const eframe::egui::Context) };
        ctx.request_repaint();
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn gui_painter_fill_rect(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(ui_ptr) = extract_handle(self_val) {
        let ui = unsafe { &mut *(ui_ptr as *mut eframe::egui::Ui) };
        let x = extract_f32(&args[0]);
        let y = extract_f32(&args[1]);
        let w = extract_f32(&args[2]);
        let h = extract_f32(&args[3]);
        let r = extract_u8(&args[4]);
        let g = extract_u8(&args[5]);
        let b = extract_u8(&args[6]);
        let a = extract_u8(&args[7]);
        let rect = eframe::egui::Rect::from_min_size(eframe::egui::pos2(x, y), eframe::egui::vec2(w, h));
        let color = eframe::egui::Color32::from_rgba_unmultiplied(r, g, b, a);
        ui.painter().rect_filled(rect, 0.0, color);
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn gui_painter_fill_circle(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(ui_ptr) = extract_handle(self_val) {
        let ui = unsafe { &mut *(ui_ptr as *mut eframe::egui::Ui) };
        let x = extract_f32(&args[0]);
        let y = extract_f32(&args[1]);
        let radius = extract_f32(&args[2]);
        let r = extract_u8(&args[3]);
        let g = extract_u8(&args[4]);
        let b = extract_u8(&args[5]);
        let a = extract_u8(&args[6]);
        let color = eframe::egui::Color32::from_rgba_unmultiplied(r, g, b, a);
        ui.painter().circle_filled(eframe::egui::pos2(x, y), radius, color);
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn gui_painter_draw_line(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(ui_ptr) = extract_handle(self_val) {
        let ui = unsafe { &mut *(ui_ptr as *mut eframe::egui::Ui) };
        let x1 = extract_f32(&args[0]);
        let y1 = extract_f32(&args[1]);
        let x2 = extract_f32(&args[2]);
        let y2 = extract_f32(&args[3]);
        let width = extract_f32(&args[4]);
        let r = extract_u8(&args[5]);
        let g = extract_u8(&args[6]);
        let b = extract_u8(&args[7]);
        let a = extract_u8(&args[8]);
        let color = eframe::egui::Color32::from_rgba_unmultiplied(r, g, b, a);
        let stroke = eframe::egui::Stroke::new(width, color);
        ui.painter().line_segment([eframe::egui::pos2(x1, y1), eframe::egui::pos2(x2, y2)], stroke);
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn gui_window_do(self_val: &Value, args: Vec<Value>, universe: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ctx_ptr), Some(Value::String(title)), Some(Value::Block(block))) = (extract_handle(self_val), args.get(0), args.get(1)) {
        let ctx = unsafe { &*(ctx_ptr as *const eframe::egui::Context) };
        let mut ret = Ok(ReturnValue::Value(Value::Nil));
        eframe::egui::Window::new(title.borrow().as_str())
            .default_size([600.0, 400.0])
            .min_size([300.0, 200.0])
            .max_size([1000.0, 800.0])
            .resizable(true)
            .show(ctx, |ui| {
                let ui_ptr = ui as *const eframe::egui::Ui as usize;
                match interpreter.run_block(block.clone(), vec![wrap_ui_handle(ui_ptr, universe)]) {
                    Ok(res) => ret = Ok(res),
                    Err(e) => ret = Err(e),
                }
            });
        return ret;
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn gui_label(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ui_ptr), Some(Value::String(text))) = (extract_handle(self_val), args.get(0)) {
        let ui = unsafe { &mut *(ui_ptr as *mut eframe::egui::Ui) };
        ui.label(text.borrow().as_str());
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn gui_button(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ui_ptr), Some(Value::String(text))) = (extract_handle(self_val), args.get(0)) {
        let ui = unsafe { &mut *(ui_ptr as *mut eframe::egui::Ui) };
        let avail_w = ui.available_width();
        let btn_h = ui.spacing().interact_size.y;
        let clicked = ui.add_sized([avail_w, btn_h], eframe::egui::Button::new(text.borrow().as_str())).clicked();
        return Ok(ReturnValue::Value(Value::Boolean(clicked)));
    }
    Ok(ReturnValue::Value(Value::Boolean(false)))
}

fn gui_text_edit(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ui_ptr), Some(Value::String(text_ref))) = (extract_handle(self_val), args.get(0)) {
        let ui = unsafe { &mut *(ui_ptr as *mut eframe::egui::Ui) };
        let mut s = text_ref.borrow().clone();
        ui.text_edit_singleline(&mut s);
        *text_ref.borrow_mut() = s;
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn gui_text_edit_multiline(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ui_ptr), Some(Value::String(text_ref))) = (extract_handle(self_val), args.get(0)) {
        let ui = unsafe { &mut *(ui_ptr as *mut eframe::egui::Ui) };
        let mut s = text_ref.borrow().clone();
        ui.add_sized(ui.available_size(), eframe::egui::TextEdit::multiline(&mut s));
        *text_ref.borrow_mut() = s;
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn gui_horizontal(self_val: &Value, args: Vec<Value>, _universe: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ui_ptr), Some(Value::Block(block))) = (extract_handle(self_val), args.get(0)) {
        let ui = unsafe { &mut *(ui_ptr as *mut eframe::egui::Ui) };
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

fn gui_vertical(self_val: &Value, args: Vec<Value>, _universe: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ui_ptr), Some(Value::Block(block))) = (extract_handle(self_val), args.get(0)) {
        let ui = unsafe { &mut *(ui_ptr as *mut eframe::egui::Ui) };
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

fn gui_scroll_area(self_val: &Value, args: Vec<Value>, _universe: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ui_ptr), Some(Value::Block(block))) = (extract_handle(self_val), args.get(0)) {
        let ui = unsafe { &mut *(ui_ptr as *mut eframe::egui::Ui) };
        let counter = SCROLL_ID_COUNTER.with(|c| { let v = c.get(); c.set(v + 1); v });
        let id = eframe::egui::Id::new(("som_scroll", counter));
        let max_h = ui.available_height() / 2.0;
        let mut ret = Ok(ReturnValue::Value(Value::Nil));
        eframe::egui::ScrollArea::vertical().id_salt(id).max_height(max_h).auto_shrink([false, false]).show(ui, |ui| {
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

fn gui_columns(self_val: &Value, args: Vec<Value>, universe: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ui_ptr), Some(Value::Integer(count)), Some(Value::Block(block))) = (extract_handle(self_val), args.get(0), args.get(1)) {
        let ui = unsafe { &mut *(ui_ptr as *mut eframe::egui::Ui) };
        let count_usize = count.to_usize().unwrap_or(2);
        let mut ret = Ok(ReturnValue::Value(Value::Nil));
        ui.columns(count_usize, |cols| {
            let mut col_objs = Vec::new();
            for col in cols {
                let col_ptr = col as *const eframe::egui::Ui as usize;
                col_objs.push(wrap_ui_handle(col_ptr, universe));
            }
            let array = Value::Array(som_ref(col_objs));
            match interpreter.run_block(block.clone(), vec![array]) {
                Ok(res) => ret = Ok(res),
                Err(e) => ret = Err(e),
            }
        });
        return ret;
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn gui_heading(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ui_ptr), Some(Value::String(text))) = (extract_handle(self_val), args.get(0)) {
        let ui = unsafe { &mut *(ui_ptr as *mut eframe::egui::Ui) };
        ui.heading(text.borrow().as_str());
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn gui_separator(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(ui_ptr) = extract_handle(self_val) {
        let ui = unsafe { &mut *(ui_ptr as *mut eframe::egui::Ui) };
        ui.separator();
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn gui_group(self_val: &Value, args: Vec<Value>, universe: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ui_ptr), Some(Value::Block(block))) = (extract_handle(self_val), args.get(0)) {
        let ui = unsafe { &mut *(ui_ptr as *mut eframe::egui::Ui) };
        let mut ret = Ok(ReturnValue::Value(Value::Nil));
        ui.group(|ui| {
            let inner_ui_ptr = ui as *const eframe::egui::Ui as usize;
            match interpreter.run_block(block.clone(), vec![wrap_ui_handle(inner_ui_ptr, universe)]) {
                Ok(res) => ret = Ok(res),
                Err(e) => ret = Err(e),
            }
        });
        return ret;
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn gui_combo_box(self_val: &Value, args: Vec<Value>, _universe: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ui_ptr), Some(Value::String(label)), Some(Value::String(selected)), Some(Value::Array(options)), Some(Value::Block(block))) = 
        (extract_handle(self_val), args.get(0), args.get(1), args.get(2), args.get(3)) {
        let ui = unsafe { &mut *(ui_ptr as *mut eframe::egui::Ui) };
        let mut current_selected = selected.borrow().clone();
        let mut changed_to = None;

        eframe::egui::ComboBox::from_label(label.borrow().as_str())
            .selected_text(&current_selected)
            .show_ui(ui, |ui| {
                for (idx, opt) in options.borrow().iter().enumerate() {
                    let opt_text = match opt {
                        Value::String(opt_str) => opt_str.borrow().clone(),
                        Value::Symbol(opt_sym) => opt_sym.clone(),
                        _ => continue,
                    };
                    if ui.selectable_value(&mut current_selected, opt_text.clone(), opt_text).clicked() {
                        changed_to = Some(idx);
                    }
                }
            });

        if let Some(idx) = changed_to {
            let val = options.borrow()[idx].clone();
            let _ = interpreter.run_block(block.clone(), vec![Value::Integer(num_bigint::BigInt::from(idx)), val]);
        }
    }
    Ok(ReturnValue::Value(Value::Nil))
}
