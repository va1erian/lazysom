pub mod helpers;
pub mod core;
pub mod numbers;
pub mod collections;
pub mod system;
pub mod gui;
pub mod async_io;
pub mod debugger;

use crate::interpreter::{Interpreter, ReturnValue};
use crate::object::Value;
use crate::universe::Universe;
use anyhow::Result;
use std::collections::HashMap;

pub use helpers::reset_scroll_id_counter;

pub fn get_primitives() -> HashMap<String, fn(&Value, Vec<Value>, &Universe, &Interpreter) -> Result<ReturnValue>> {
    let mut prims = HashMap::new();

    core::register(&mut prims);
    numbers::register(&mut prims);
    collections::register(&mut prims);
    system::register(&mut prims);
    gui::register(&mut prims);
    async_io::register(&mut prims);
    debugger::register(&mut prims);

    prims
}
