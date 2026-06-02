use std::sync::Mutex;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::path::PathBuf;
use crate::universe::Universe;
use crate::interpreter::Interpreter;
use crate::object::{Value, SomObject, som_ref};

#[derive(Debug, Clone)]
pub enum DebugCommand {
    Evaluate(String),
    Resume,
    StepInto,
    Stop,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SerializedVariable {
    pub name: String,
    pub val_type: String,
    pub val_str: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SerializedFrame {
    pub name: String,
    pub class_name: String,
    pub receiver_str: String,
    pub args: Vec<SerializedVariable>,
    pub locals: Vec<SerializedVariable>,
    pub source: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DebugEvent {
    Idle,
    Running,
    Paused {
        stack: Vec<SerializedFrame>,
    },
    Completed {
        result: String,
    },
    Errored {
        message: String,
    },
}

thread_local! {
    pub static IS_BG_THREAD: std::cell::Cell<bool> = std::cell::Cell::new(false);
}

lazy_static::lazy_static! {
    pub static ref VM_RUNNER: Mutex<VmRunner> = Mutex::new(VmRunner::new());
}

#[derive(Debug, Clone)]
pub enum GuiRegistration {
    Snippet { title: String, code: String },
    Class { class_name: String },
}

pub struct VmRunner {
    cmd_tx: Option<Sender<DebugCommand>>,
    event_rx: Option<Receiver<DebugEvent>>,
    status: DebugEvent,
    last_completed_result: String,
    last_errored_message: String,
    current_stack: Vec<SerializedFrame>,
    output_buffer: String,
    gui_registrations: Vec<GuiRegistration>,
}

impl VmRunner {
    pub fn new() -> Self {
        Self {
            cmd_tx: None,
            event_rx: None,
            status: DebugEvent::Idle,
            last_completed_result: String::new(),
            last_errored_message: String::new(),
            current_stack: Vec::new(),
            output_buffer: String::new(),
            gui_registrations: Vec::new(),
        }
    }

    pub fn start_snippet(&mut self, code: String) {
        // If thread already running, terminate it
        self.stop();

        let (cmd_tx, cmd_rx) = channel::<DebugCommand>();
        let (event_tx, event_rx) = channel::<DebugEvent>();

        self.cmd_tx = Some(cmd_tx);
        self.event_rx = Some(event_rx);
        self.status = DebugEvent::Running;
        self.last_completed_result.clear();
        self.last_errored_message.clear();
        self.current_stack.clear();
        self.output_buffer.clear();
        // Keep gui_registrations from previous runs or clear them? Better to clear them on new evaluation run.
        self.gui_registrations.clear();

        let event_tx_clone = event_tx.clone();

        std::thread::Builder::new()
            .name("SomVmRunner".to_string())
            .spawn(move || {
                IS_BG_THREAD.with(|b| b.set(true));
                let classpath = vec![
                    PathBuf::from("SOM/Smalltalk"),
                    PathBuf::from("SOM/TestSuite"),
                    PathBuf::from("Async"),
                    PathBuf::from("Tools"),
                    PathBuf::from("."),
                ];

                let mut universe = Universe::new(classpath);
                universe.dbg_cmd_rx = Some(cmd_rx);
                universe.dbg_event_tx = Some(event_tx_clone.clone());

                if let Err(e) = Self::run_evaluation(&universe, &code, &event_tx_clone) {
                    let _ = event_tx_clone.send(DebugEvent::Errored { message: e.to_string() });
                }
            })
            .expect("Failed to spawn background interpreter thread");
    }

    pub fn append_output(&mut self, text: &str) {
        self.output_buffer.push_str(text);
    }

    pub fn take_output(&mut self) -> String {
        std::mem::take(&mut self.output_buffer)
    }

    pub fn register_gui(&mut self, title: String, code: String) {
        self.gui_registrations.push(GuiRegistration::Snippet { title, code });
    }

    pub fn register_gui_class(&mut self, class_name: String) {
        self.gui_registrations.push(GuiRegistration::Class { class_name });
    }

    pub fn take_gui_registrations(&mut self) -> Vec<GuiRegistration> {
        std::mem::take(&mut self.gui_registrations)
    }

    fn run_evaluation(universe: &Universe, code: &str, event_tx: &Sender<DebugEvent>) -> anyhow::Result<()> {
        universe.load_class("Object")?;
        universe.load_class("Class")?;
        universe.load_class("Metaclass")?;
        universe.load_class("True")?;
        universe.load_class("False")?;
        universe.load_class("Nil")?;
        universe.load_class("String")?;
        universe.load_class("Integer")?;
        universe.load_class("Double")?;
        universe.load_class("Array")?;
        universe.load_class("Block")?;
        universe.load_class("Symbol")?;
        universe.load_class("Method")?;
        universe.load_class("Primitive")?;
        let sys_class = universe.load_class("System")?;

        let system_obj = som_ref(SomObject {
            class: sys_class.clone(),
            fields: Vec::new(),
        });
        universe.set_global("system", Value::Object(system_obj.clone()));
        universe.set_global("nil", Value::Nil);
        universe.set_global("true", Value::Boolean(true));
        universe.set_global("false", Value::Boolean(false));

        let interpreter = Interpreter::new(universe);
        let _ = event_tx.send(DebugEvent::Running);

        match interpreter.evaluate_snippet(code) {
            Ok(v) => {
                let res_str = format!("{:?}", v);
                let _ = event_tx.send(DebugEvent::Completed { result: res_str });
            }
            Err(e) => {
                let _ = event_tx.send(DebugEvent::Errored { message: e.to_string() });
            }
        }
        Ok(())
    }

    pub fn poll_events(&mut self) {
        if let Some(rx) = &self.event_rx {
            while let Ok(event) = rx.try_recv() {
                self.status = event.clone();
                match event {
                    DebugEvent::Completed { result } => {
                        self.last_completed_result = result;
                    }
                    DebugEvent::Errored { message } => {
                        self.last_errored_message = message;
                    }
                    DebugEvent::Paused { stack } => {
                        self.current_stack = stack;
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn send_command(&self, cmd: DebugCommand) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(cmd);
        }
    }

    pub fn resume(&self) {
        self.send_command(DebugCommand::Resume);
    }

    pub fn step_into(&self) {
        self.send_command(DebugCommand::StepInto);
    }

    pub fn stop(&mut self) {
        self.send_command(DebugCommand::Stop);
        self.cmd_tx = None;
        self.event_rx = None;
        self.status = DebugEvent::Idle;
    }

    pub fn get_status_str(&self) -> String {
        match &self.status {
            DebugEvent::Idle => "idle".to_string(),
            DebugEvent::Running => "running".to_string(),
            DebugEvent::Paused { .. } => "paused".to_string(),
            DebugEvent::Completed { .. } => "completed".to_string(),
            DebugEvent::Errored { .. } => "errored".to_string(),
        }
    }

    pub fn get_result(&self) -> String {
        self.last_completed_result.clone()
    }

    pub fn get_error(&self) -> String {
        self.last_errored_message.clone()
    }

    pub fn get_stack(&self) -> Vec<SerializedFrame> {
        self.current_stack.clone()
    }
}
