use eframe::egui;
use crate::universe::Universe;
use crate::interpreter::Interpreter;
use crate::object::Value;
use std::sync::Arc;

pub struct SomGuiApp {
    universe: Arc<Universe>,
    root_object: Value,
    custom_windows: Vec<(String, Value)>,
    /// Max SOM method/block activation depth for interpreters created here. This app's
    /// `update` runs on the main thread (an eframe/winit requirement), which only has the
    /// OS-default stack size, so this must stay at or below
    /// `interpreter::MAIN_THREAD_MAX_DEPTH` unless the caller knows the main thread has a
    /// larger stack.
    max_depth: usize,
}

impl SomGuiApp {
    pub fn new(universe: Arc<Universe>, root_object: Value) -> Self {
        Self::with_max_depth(universe, root_object, crate::interpreter::MAIN_THREAD_MAX_DEPTH)
    }

    pub fn with_max_depth(universe: Arc<Universe>, root_object: Value, max_depth: usize) -> Self {
        Self {
            universe,
            root_object,
            custom_windows: Vec::new(),
            max_depth,
        }
    }
}

impl eframe::App for SomGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let interpreter = Interpreter::with_max_depth(&self.universe, self.max_depth);

        // Poll background VM runner events and repaint if running
        let new_guis = {
            let mut runner = crate::vm_runner::VM_RUNNER.lock().unwrap();
            runner.poll_events();
            if runner.get_status_str() == "running" || runner.get_status_str() == "paused" {
                ctx.request_repaint();
            }
            runner.take_gui_registrations()
        };

        // Process new custom GUI registrations
        for reg in new_guis {
            match reg {
                crate::vm_runner::GuiRegistration::Snippet { title, code } => {
                    match interpreter.evaluate_snippet(&code) {
                        Ok(block_val) => {
                            self.custom_windows.push((title, block_val));
                        }
                        Err(e) => {
                            eprintln!("Error compiling custom GUI snippet '{}': {}", title, e);
                        }
                    }
                }
                crate::vm_runner::GuiRegistration::Class { class_name } => {
                    match self.universe.load_class(&class_name) {
                        Ok(cls) => {
                            match interpreter.dispatch(Value::Class(cls), "new", vec![]) {
                                Ok(instance) => {
                                    self.custom_windows.push((class_name, instance));
                                }
                                Err(e) => {
                                    eprintln!("Error instantiating GUI class '{}': {}", class_name, e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error loading GUI class '{}': {}", class_name, e);
                        }
                    }
                }
            }
        }

        // Reset per-frame counter so scroll area IDs are stable across frames.
        crate::primitives::reset_scroll_id_counter();

        // 1. Render primary IDE frame
        let ctx_ptr = ctx as *const egui::Context as usize;
        if let Err(e) = interpreter.dispatch(
            self.root_object.clone(),
            "renderFrameOn:",
            vec![Value::NativeHandle(ctx_ptr)],
        ) {
            eprintln!("Error rendering GUI frame: {}", e);
        }

        // 2. Render active custom windows
        let mut kept_windows = Vec::new();
        for (title, val) in &self.custom_windows {
            let mut open = true;
            let mut ret_err = None;

            eframe::egui::Window::new(title)
                .open(&mut open)
                .default_size([400.0, 300.0])
                .min_size([50.0, 50.0])
                .resizable(true)
                .show(ctx, |ui| {
                    let ui_ptr = ui as *const eframe::egui::Ui as usize;
                    let ui_val = crate::primitives::helpers::wrap_ui_handle(ui_ptr, &self.universe);
                    match val {
                        Value::Block(b) => {
                            if let Err(e) = interpreter.run_block(b.clone(), vec![ui_val]) {
                                ret_err = Some(e);
                            }
                        }
                        Value::Object(_) => {
                            if let Err(e) = interpreter.dispatch(val.clone(), "renderOn:", vec![ui_val]) {
                                ret_err = Some(e);
                            }
                        }
                        _ => {
                            ret_err = Some(anyhow::anyhow!("Custom GUI registered value is not a Block or Object"));
                        }
                    }
                });

            if let Some(err) = ret_err {
                eprintln!("Error rendering custom GUI window '{}': {}", title, err);
                continue;
            }

            if open {
                kept_windows.push((title.clone(), val.clone()));
            }
        }
        self.custom_windows = kept_windows;
    }

    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The App trait requires `update` or `ui`.
    }
}
