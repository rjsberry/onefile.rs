use qptr::{make_static_unique, Unique};

fn main() {
    let ptr: Unique<dyn std::any::Any> = make_static_unique!(|| -> i32 { 0 }).unwrap();
    let ptr: Unique<i32> = ptr.downcast().unwrap();
    assert_eq!(*ptr, 0);
}
