use core::any::Any;

use qptr::{make_static_shared, make_static_unique, Shared, Unique};

#[test]
fn shared_make_static() {
    let _shared = make_static_shared!(|| -> i32 { 123 }).unwrap();
}

#[test]
#[should_panic]
fn shared_already_claimed() {
    for _ in 0..2 {
        let _shared = make_static_shared!(|| -> i32 { 123 }).unwrap();
    }
}

#[test]
fn unique_make_static() {
    let _unique = make_static_unique!(|| -> i32 { 123 }).unwrap();
}

#[test]
#[should_panic]
fn unique_already_claimed() {
    for _ in 0..2 {
        let _unique = make_static_unique!(|| -> i32 { 123 }).unwrap();
    }
}

#[test]
fn shared_downcast_ok() {
    let shared: Shared<dyn Any> = make_static_shared!(|| -> i32 { 123 }).unwrap();
    let shared: Shared<i32> = shared.downcast().unwrap();
    assert_eq!(*shared, 123);
}

#[test]
fn shared_downcast_err() {
    let shared: Shared<dyn Any> = make_static_shared!(|| -> i32 { 123 }).unwrap();
    assert!(shared.downcast::<u32>().is_err());
}

#[test]
fn unique_downcast_ok() {
    let unique: Unique<dyn Any> = make_static_unique!(|| -> i32 { 123 }).unwrap();
    let unique: Unique<i32> = unique.downcast().unwrap();
    assert_eq!(*unique, 123);
}

#[test]
fn unique_downcast_err() {
    let unique: Unique<dyn Any> = make_static_unique!(|| -> i32 { 123 }).unwrap();
    assert!(unique.downcast::<u32>().is_err());
}
