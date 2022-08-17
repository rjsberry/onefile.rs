use qptr::{make_static_shared, Shared};

fn main() {
    let ptr: Shared<dyn std::any::Any> = make_static_shared!(|| -> i32 { 0 }).unwrap();
    let ptr: Shared<i32> = ptr.downcast().unwrap();
    assert_eq!(*ptr, 0);
}
