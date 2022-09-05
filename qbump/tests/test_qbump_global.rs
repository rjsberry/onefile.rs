use qbump::GlobalBump;

#[global_allocator]
static BUMP: GlobalBump<{ 128 * 1024 }> = unsafe { GlobalBump::new() };

#[test]
#[rustfmt::skip]
fn global_bump_dyn() {
    trait V { fn v(&self) -> i32; }
    struct W(i32);
    impl V for W { fn v(&self) -> i32 { self.0 } }

    let v: Box<dyn V> = Box::new(W(123));
    assert_eq!(v.v(), 123);
}
