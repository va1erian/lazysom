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
    prims.insert("EguiContext>>leftPanel:do:".to_string(), gui_left_panel);
    prims.insert("EguiContext>>rightPanel:do:".to_string(), gui_right_panel);
    prims.insert("EguiContext>>bottomPanel:do:".to_string(), gui_bottom_panel);
    prims.insert("EguiContext>>topPanel:do:".to_string(), gui_top_panel);
    prims.insert("EguiContext>>centralPanel:".to_string(), gui_central_panel);
    prims.insert("EguiContext>>applyThemeIsDark:fontSize:fontName:".to_string(), gui_ctx_apply_theme);
    prims.insert("EguiUi>>label:".to_string(), gui_label);
    prims.insert("EguiUi>>button:".to_string(), gui_button);
    prims.insert("EguiUi>>textEdit:".to_string(), gui_text_edit);
    prims.insert("EguiUi>>textEditMultiline:".to_string(), gui_text_edit_multiline);
    prims.insert("EguiUi>>textEditReadonly:".to_string(), gui_text_edit_readonly);
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
            .min_size([50.0, 50.0])
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
        let clicked = if ui.layout().is_horizontal() {
            ui.button(text.borrow().as_str()).clicked()
        } else {
            let avail_w = ui.available_width();
            let btn_h = ui.spacing().interact_size.y;
            ui.add_sized([avail_w, btn_h], eframe::egui::Button::new(text.borrow().as_str())).clicked()
        };
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

pub fn highlight_som(text: &str, is_dark: bool, font_id: eframe::egui::FontId) -> eframe::egui::text::LayoutJob {
    let mut job = eframe::egui::text::LayoutJob::default();
    
    // Choose colors based on theme
    let color_comment = if is_dark { eframe::egui::Color32::from_rgb(117, 113, 94) } else { eframe::egui::Color32::from_rgb(128, 128, 128) };
    let color_string = if is_dark { eframe::egui::Color32::from_rgb(230, 219, 116) } else { eframe::egui::Color32::from_rgb(0, 128, 0) };
    let color_keyword = if is_dark { eframe::egui::Color32::from_rgb(249, 38, 114) } else { eframe::egui::Color32::from_rgb(0, 0, 255) };
    let color_symbol = if is_dark { eframe::egui::Color32::from_rgb(102, 217, 239) } else { eframe::egui::Color32::from_rgb(43, 145, 175) };
    let color_number = if is_dark { eframe::egui::Color32::from_rgb(174, 129, 255) } else { eframe::egui::Color32::from_rgb(163, 21, 21) };
    let color_operator = if is_dark { eframe::egui::Color32::from_rgb(249, 150, 59) } else { eframe::egui::Color32::from_rgb(190, 100, 0) };
    let color_normal = if is_dark { eframe::egui::Color32::from_rgb(248, 248, 242) } else { eframe::egui::Color32::from_rgb(0, 0, 0) };

    let mut chars = text.char_indices().peekable();
    
    while let Some(&(start_idx, c)) = chars.peek() {
        if c == '"' {
            // Comment
            chars.next();
            let mut end_idx = start_idx + 1;
            while let Some(&(_, next_c)) = chars.peek() {
                chars.next();
                end_idx += next_c.len_utf8();
                if next_c == '"' {
                    break;
                }
            }
            job.append(
                &text[start_idx..end_idx],
                0.0,
                eframe::egui::TextFormat {
                    font_id: font_id.clone(),
                    color: color_comment,
                    ..Default::default()
                },
            );
        } else if c == '\'' {
            // String
            chars.next();
            let mut end_idx = start_idx + 1;
            let mut escaped = false;
            while let Some(&(_, next_c)) = chars.peek() {
                chars.next();
                end_idx += next_c.len_utf8();
                if escaped {
                    escaped = false;
                } else if next_c == '\\' {
                    escaped = true;
                } else if next_c == '\'' {
                    break;
                }
            }
            job.append(
                &text[start_idx..end_idx],
                0.0,
                eframe::egui::TextFormat {
                    font_id: font_id.clone(),
                    color: color_string,
                    ..Default::default()
                },
            );
        } else if c == '#' {
            // Symbol (identifier following #)
            chars.next();
            let mut end_idx = start_idx + 1;
            while let Some(&(_, next_c)) = chars.peek() {
                if next_c.is_alphanumeric() || next_c == '_' || next_c == ':' {
                    chars.next();
                    end_idx += next_c.len_utf8();
                } else {
                    break;
                }
            }
            job.append(
                &text[start_idx..end_idx],
                0.0,
                eframe::egui::TextFormat {
                    font_id: font_id.clone(),
                    color: color_symbol,
                    ..Default::default()
                },
            );
        } else if c.is_ascii_digit() {
            // Number
            let mut end_idx = start_idx;
            while let Some(&(_, next_c)) = chars.peek() {
                if next_c.is_ascii_digit() || next_c == '.' {
                    chars.next();
                    end_idx += next_c.len_utf8();
                } else {
                    break;
                }
            }
            job.append(
                &text[start_idx..end_idx],
                0.0,
                eframe::egui::TextFormat {
                    font_id: font_id.clone(),
                    color: color_number,
                    ..Default::default()
                },
            );
        } else if c.is_alphabetic() || c == '_' {
            // Identifier or Keyword
            let mut end_idx = start_idx;
            while let Some(&(_, next_c)) = chars.peek() {
                if next_c.is_alphanumeric() || next_c == '_' || next_c == ':' {
                    chars.next();
                    end_idx += next_c.len_utf8();
                } else {
                    break;
                }
            }
            let word = &text[start_idx..end_idx];
            let is_keyword = match word {
                "self" | "super" | "nil" | "true" | "false" | "system" | "primitive" => true,
                w if w.ends_with(':') => true,
                _ => false,
            };
            let color = if is_keyword { color_keyword } else { color_normal };
            job.append(
                word,
                0.0,
                eframe::egui::TextFormat {
                    font_id: font_id.clone(),
                    color,
                    ..Default::default()
                },
            );
        } else if c == ':' && text.get(start_idx..start_idx+2) == Some(":=") {
            chars.next(); // :
            chars.next(); // =
            job.append(
                ":=",
                0.0,
                eframe::egui::TextFormat {
                    font_id: font_id.clone(),
                    color: color_operator,
                    ..Default::default()
                },
            );
        } else if "~&|*/\\+>=<@%!-^".contains(c) {
            // Operator
            chars.next();
            job.append(
                &text[start_idx..start_idx+c.len_utf8()],
                0.0,
                eframe::egui::TextFormat {
                    font_id: font_id.clone(),
                    color: color_operator,
                    ..Default::default()
                },
            );
        } else {
            // Normal character
            chars.next();
            job.append(
                &text[start_idx..start_idx+c.len_utf8()],
                0.0,
                eframe::egui::TextFormat {
                    font_id: font_id.clone(),
                    color: color_normal,
                    ..Default::default()
                },
            );
        }
    }
    job
}

fn gui_text_edit_multiline(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ui_ptr), Some(Value::String(text_ref))) = (extract_handle(self_val), args.get(0)) {
        let ui = unsafe { &mut *(ui_ptr as *mut eframe::egui::Ui) };
        let mut s = text_ref.borrow().clone();
        
        let is_dark = ui.style().visuals.dark_mode;
        
        let mut text_edit = eframe::egui::TextEdit::multiline(&mut s)
            .font(eframe::egui::TextStyle::Monospace)
            .desired_width(f32::INFINITY)
            .desired_rows(15);
            
        let layouter = &mut |ui: &eframe::egui::Ui, string: &dyn eframe::egui::TextBuffer, _wrap_width: f32| {
            let font_id = eframe::egui::TextStyle::Monospace.resolve(ui.style());
            let job = highlight_som(string.as_str(), is_dark, font_id);
            ui.painter().layout_job(job)
        };
        
        text_edit = text_edit.layouter(layouter);
        
        ui.add(text_edit);
        *text_ref.borrow_mut() = s;
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn gui_text_edit_readonly(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ui_ptr), Some(Value::String(text_ref))) = (extract_handle(self_val), args.get(0)) {
        let ui = unsafe { &mut *(ui_ptr as *mut eframe::egui::Ui) };
        let text = text_ref.borrow().clone();
        // Render as a non-interactive multiline text area that fills all available width
        // and expands vertically to show content without a fixed row cap.
        let mut text_copy = text;
        ui.add(
            eframe::egui::TextEdit::multiline(&mut text_copy)
                .font(eframe::egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY)
                .interactive(false)
        );
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
        // Use available height so the scroll area expands to fill its parent.
        // Fall back to 200px if available height is too small (e.g. inside a group).
        let available = ui.available_height();
        let max_h = if available > 50.0 { available } else { 200.0 };
        let mut ret = Ok(ReturnValue::Value(Value::Nil));
        eframe::egui::ScrollArea::vertical()
            .id_salt(id)
            .min_scrolled_height(200.0)
            .max_height(max_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
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

#[allow(deprecated)]
fn gui_left_panel(self_val: &Value, args: Vec<Value>, universe: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ctx_ptr), Some(Value::String(title)), Some(Value::Block(block))) = (extract_handle(self_val), args.get(0), args.get(1)) {
        let ctx = unsafe { &*(ctx_ptr as *const eframe::egui::Context) };
        let mut ret = Ok(ReturnValue::Value(Value::Nil));
        let panel_id = eframe::egui::Id::new(&*title.borrow());
        eframe::egui::SidePanel::left(panel_id)
            .default_width(300.0)
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

#[allow(deprecated)]
fn gui_right_panel(self_val: &Value, args: Vec<Value>, universe: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ctx_ptr), Some(Value::String(title)), Some(Value::Block(block))) = (extract_handle(self_val), args.get(0), args.get(1)) {
        let ctx = unsafe { &*(ctx_ptr as *const eframe::egui::Context) };
        let mut ret = Ok(ReturnValue::Value(Value::Nil));
        let panel_id = eframe::egui::Id::new(&*title.borrow());
        eframe::egui::SidePanel::right(panel_id)
            .default_width(300.0)
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

#[allow(deprecated)]
fn gui_bottom_panel(self_val: &Value, args: Vec<Value>, universe: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ctx_ptr), Some(Value::String(title)), Some(Value::Block(block))) = (extract_handle(self_val), args.get(0), args.get(1)) {
        let ctx = unsafe { &*(ctx_ptr as *const eframe::egui::Context) };
        let mut ret = Ok(ReturnValue::Value(Value::Nil));
        let panel_id = eframe::egui::Id::new(&*title.borrow());
        eframe::egui::TopBottomPanel::bottom(panel_id)
            .default_height(200.0)
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

#[allow(deprecated)]
fn gui_top_panel(self_val: &Value, args: Vec<Value>, universe: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ctx_ptr), Some(Value::String(title)), Some(Value::Block(block))) = (extract_handle(self_val), args.get(0), args.get(1)) {
        let ctx = unsafe { &*(ctx_ptr as *const eframe::egui::Context) };
        let mut ret = Ok(ReturnValue::Value(Value::Nil));
        let panel_id = eframe::egui::Id::new(&*title.borrow());
        eframe::egui::TopBottomPanel::top(panel_id)
            .resizable(false)
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

#[allow(deprecated)]
fn gui_central_panel(self_val: &Value, args: Vec<Value>, universe: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ctx_ptr), Some(Value::Block(block))) = (extract_handle(self_val), args.get(0)) {
        let ctx = unsafe { &*(ctx_ptr as *const eframe::egui::Context) };
        let mut ret = Ok(ReturnValue::Value(Value::Nil));
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
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

fn gui_ctx_apply_theme(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Some(ctx_ptr), Some(Value::Boolean(is_dark)), Some(Value::Integer(size_val)), Some(Value::String(font_name))) = 
        (extract_handle(self_val), args.get(0), args.get(1), args.get(2)) {
        let ctx = unsafe { &*(ctx_ptr as *const eframe::egui::Context) };
        
        if *is_dark {
            ctx.set_visuals(eframe::egui::Visuals::dark());
        } else {
            ctx.set_visuals(eframe::egui::Visuals::light());
        }

        let size = size_val.to_f32().unwrap_or(14.0);
        let font_family = match font_name.borrow().as_str() {
            "Monospace" => eframe::egui::FontFamily::Monospace,
            _ => eframe::egui::FontFamily::Proportional,
        };

        let mut style = (*ctx.global_style()).clone();
        
        for (style_type, font_id) in style.text_styles.iter_mut() {
            font_id.size = size;
            if style_type == &eframe::egui::TextStyle::Monospace {
                font_id.family = eframe::egui::FontFamily::Monospace;
            } else {
                font_id.family = font_family.clone();
            }
        }
        
        ctx.set_global_style(style);
    }
    Ok(ReturnValue::Value(Value::Nil))
}
