use gc::{Gc, GcCell};

fn main() {
    let s = Gc::new(GcCell::new(String::from("hello")));
    let ptr = &*s as *const _ as usize;
    println!("{}", ptr);
}
