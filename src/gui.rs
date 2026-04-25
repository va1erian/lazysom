use eframe::egui;
use crate::universe::Universe;
use crate::interpreter::Interpreter;
use crate::object::Value;
use std::sync::Arc;

pub struct SomGuiApp {
    universe: Arc<Universe>,
    root_object: Value,
}

impl SomGuiApp {
    pub fn new(universe: Arc<Universe>, root_object: Value) -> Self {
        Self {
            universe,
            root_object,
        }
    }
}

impl eframe::App for SomGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let interpreter = Interpreter::new(&self.universe);
        let ctx_ptr = ctx as *const egui::Context as usize;

        if let Err(e) = interpreter.dispatch(
            self.root_object.clone(),
            "renderFrameOn:",
            vec![Value::NativeHandle(ctx_ptr)],
        ) {
            eprintln!("Error rendering GUI frame: {}", e);
        }
    }

    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The App trait requires `update` or `ui`. In 0.34.1, `ui` is added as a requirement.
    }
}
