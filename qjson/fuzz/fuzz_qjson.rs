fn main() {
    afl::fuzz!(|data: &[u8]| {
        if let Ok(s) = core::str::from_utf8(data) {
            let _ = qjson::validate::<128>(s);
        }
    });
}
