#[test]
fn ok_empty_obj() {
    let src = r#"{}"#;
    qjson::validate::<1>(src).unwrap();
}

#[test]
fn ok_empty_obj_with_whitespace() {
    let src = "   {\r\n\t\n} ";
    qjson::validate::<1>(src).unwrap();
}

#[test]
fn err_empty_obj_extra_opening_brace() {
    let src = r#"{{}"#;
    let err = qjson::validate::<1>(src).unwrap_err();
    assert_eq!(err.kind(), qjson::ErrorKind::UnexpectedToken);
    assert_eq!(err.lineno(), 1);
    assert_eq!(err.col(), 2);
}

#[test]
fn err_empty_obj_extra_closing_brace() {
    let src = r#"{}}"#;
    let err = qjson::validate::<1>(src).unwrap_err();
    assert_eq!(err.kind(), qjson::ErrorKind::UnexpectedToken);
    assert_eq!(err.lineno(), 1);
    assert_eq!(err.col(), 3);
}

#[test]
fn ok_empty_arr() {
    let src = r#"[]"#;
    qjson::validate::<1>(src).unwrap();
}

#[test]
fn ok_empty_arr_with_whitespace() {
    let src = "   [\r\n\t\n] ";
    qjson::validate::<1>(src).unwrap();
}

#[test]
fn err_empty_arr_extra_opening_brace() {
    let src = r#"[[]"#;
    let err = qjson::validate::<1>(src).unwrap_err();
    assert_eq!(err.kind(), qjson::ErrorKind::UnexpectedEof);
    assert_eq!(err.lineno(), 1);
    assert_eq!(err.col(), 3);
}

#[test]
fn err_empty_arr_extra_closing_brace() {
    let src = r#"[]]"#;
    let err = qjson::validate::<1>(src).unwrap_err();
    assert_eq!(err.kind(), qjson::ErrorKind::UnexpectedToken);
    assert_eq!(err.lineno(), 1);
    assert_eq!(err.col(), 3);
}

#[test]
fn err_empty_arr_trailing_comma() {
    let src = r#"[],"#;
    let err = qjson::validate::<1>(src).unwrap_err();
    assert_eq!(err.kind(), qjson::ErrorKind::UnexpectedToken);
    assert_eq!(err.lineno(), 1);
    assert_eq!(err.col(), 3);
}

#[test]
fn err_arr_comma_integer() {
    let src = r#"[,1]"#;
    let mut i = None;
    let mut desc = [qjson::Schema::Integer(&mut i)];
    let err = qjson::from_str::<_, 1>(src, &mut desc).unwrap_err();
    assert!(i.is_none());
    assert_eq!(err.kind(), qjson::ErrorKind::UnexpectedToken);
    assert_eq!(err.lineno(), 1);
    assert_eq!(err.col(), 2);
}

#[test]
fn err_arr_only_comma() {
    let src = r#"[,]"#;
    let err = qjson::validate::<1>(src).unwrap_err();
    assert_eq!(err.kind(), qjson::ErrorKind::UnexpectedToken);
    assert_eq!(err.lineno(), 1);
    assert_eq!(err.col(), 2);
}

#[test]
fn ok_empty_str() {
    let src = r#""""#;
    qjson::validate::<0>(src).unwrap();
}

#[test]
fn err_backslash_in_str() {
    let src = r#""\""#;
    let err = qjson::validate::<1>(src).unwrap_err();
    assert_eq!(err.kind(), qjson::ErrorKind::UnterminatedString);
    assert_eq!(err.lineno(), 1);
    assert_eq!(err.col(), 3);
}

#[test]
fn err_arr_integers_no_comma() {
    let src = r#"[1 1]"#;
    let mut i0 = None;
    let mut i1 = None;
    let mut desc = [
        qjson::Schema::Integer(&mut i0),
        qjson::Schema::Integer(&mut i1),
    ];
    let err = qjson::from_str::<_, 1>(src, &mut desc).unwrap_err();
    assert_eq!(i0.unwrap(), 1);
    assert!(i1.is_none());
    assert_eq!(err.kind(), qjson::ErrorKind::MissingComma);
    assert_eq!(err.lineno(), 1);
    assert_eq!(err.col(), 4);
}

#[test]
fn err_shallow_depth() {
    let src = r#"{"a":{}}"#;
    let err = qjson::validate::<1>(src).unwrap_err();
    assert_eq!(err.kind(), qjson::ErrorKind::MaxDepthExceeded);
    assert_eq!(err.lineno(), 1);
    assert_eq!(err.col(), 6);
}

#[test]
fn err_deep_depth() {
    let src = r#"{"a":{"b":{"c":{"d":{"e":{"f":{"g":{"h":{"i":{"j":{"k":{}}}}}}}}}}}}"#;
    let err = qjson::validate::<10>(src).unwrap_err();
    assert_eq!(err.kind(), qjson::ErrorKind::MaxDepthExceeded);
    assert_eq!(err.lineno(), 1);
    assert_eq!(err.col(), 51);
}

#[test]
fn ok_small_positive_int() {
    #[derive(Default)]
    struct S {
        i: Option<i64>,
    }

    let mut s = S::default();
    let src = r#"{"i":1}"#;
    let mut desc = [("i", qjson::Schema::Integer(&mut s.i))];
    qjson::from_str::<_, 1>(src, &mut desc).unwrap();
    assert_eq!(s.i.unwrap(), 1);
}

#[test]
fn ok_small_negative_int() {
    #[derive(Default)]
    struct S {
        i: Option<i64>,
    }

    let mut s = S::default();
    let src = r#"{"i":-1}"#;
    let mut desc = [("i", qjson::Schema::Integer(&mut s.i))];
    qjson::from_str::<_, 1>(src, &mut desc).unwrap();
    assert_eq!(s.i.unwrap(), -1);
}

#[test]
fn ok_large_positive_int() {
    #[derive(Default)]
    struct S {
        i: Option<i64>,
    }

    let mut s = S::default();
    let src = r#"{"i":12345678}"#;
    let mut desc = [("i", qjson::Schema::Integer(&mut s.i))];
    qjson::from_str::<_, 1>(src, &mut desc).unwrap();
    assert_eq!(s.i.unwrap(), 12345678);
}

#[test]
fn ok_large_negative_int() {
    #[derive(Default)]
    struct S {
        i: Option<i64>,
    }

    let src = r#"{"i":-12345678}"#;
    let mut s = S::default();
    let mut desc = [("i", qjson::Schema::Integer(&mut s.i))];
    qjson::from_str::<_, 1>(src, &mut desc).unwrap();
    assert_eq!(s.i.unwrap(), -12345678);
}

#[test]
fn ok_small_positive_float() {
    #[derive(Default)]
    struct S {
        f: Option<f64>,
    }

    let mut s = S::default();
    let src = r#"{"f":1.0}"#;
    let mut desc = [("f", qjson::Schema::Float(&mut s.f))];
    qjson::from_str::<_, 1>(src, &mut desc).unwrap();
    assert!((s.f.unwrap() - 1.0).abs() < 1e-9);
}

#[test]
fn ok_small_negative_float() {
    #[derive(Default)]
    struct S {
        f: Option<f64>,
    }

    let mut s = S::default();
    let src = r#"{"f":-1.0}"#;
    let mut desc = [("f", qjson::Schema::Float(&mut s.f))];
    qjson::from_str::<_, 1>(src, &mut desc).unwrap();
    assert!((s.f.unwrap() + 1.0).abs() < 1e-9);
}

#[test]
fn ok_array_of_integers() {
    #[derive(Default)]
    struct S {
        arr: [Option<i64>; 3],
    }

    let mut s = S::default();
    let src = r#"{"arr":[1,-1]}"#;

    let S { arr: [a0, a1, a2] } = &mut s;
    let mut arr_desc = [
        qjson::Schema::Integer(a0),
        qjson::Schema::Integer(a1),
        qjson::Schema::Integer(a2),
    ];
    let mut desc = [("arr", qjson::Schema::Array(&mut arr_desc))];

    qjson::from_str::<_, 1>(src, &mut desc).unwrap();
    assert_eq!(s.arr, [Some(1), Some(-1), None]);
}

#[test]
fn ok_array_of_different_types() {
    #[derive(Default)]
    struct S {
        a0: Option<i64>,
        a1: Option<f64>,
    }

    let mut s = S::default();
    let src = r#"{"arr":[1,1.0]}"#;

    let S { a0, a1 } = &mut s;
    let mut arr_desc = [qjson::Schema::Integer(a0), qjson::Schema::Float(a1)];
    let mut desc = [("arr", qjson::Schema::Array(&mut arr_desc))];

    qjson::from_str::<_, 1>(src, &mut desc).unwrap();
    assert_eq!(s.a0.unwrap(), 1);
    assert!((s.a1.unwrap() - 1.0).abs() < 1e-9);
}

#[test]
fn ok_array_of_objects() {
    #[derive(Default)]
    struct S<'src> {
        obj0_name: Option<&'src str>,
        obj0_val: Option<i64>,
        obj1_name: Option<&'src str>,
        obj1_val: Option<i64>,
    }

    let src = r#"{"arr":[{"name":"foo","val":1},{"name":"bar","val":2}]}"#;

    let mut s = S::default();
    let S {
        obj0_name,
        obj0_val,
        obj1_name,
        obj1_val,
    } = &mut s;

    let mut obj0_desc = [
        ("name", qjson::Schema::Str(obj0_name)),
        ("val", qjson::Schema::Integer(obj0_val)),
    ];
    let mut obj1_desc = [
        ("name", qjson::Schema::Str(obj1_name)),
        ("val", qjson::Schema::Integer(obj1_val)),
    ];
    let mut arr_desc = [
        qjson::Schema::Object(&mut obj0_desc),
        qjson::Schema::Object(&mut obj1_desc),
    ];
    let mut desc = [("arr", qjson::Schema::Array(&mut arr_desc))];

    qjson::from_str::<_, 2>(src, &mut desc).unwrap();
    assert_eq!(s.obj0_name.unwrap(), "foo");
    assert_eq!(s.obj0_val.unwrap(), 1);
    assert_eq!(s.obj1_name.unwrap(), "bar");
    assert_eq!(s.obj1_val.unwrap(), 2);
}

#[test]
fn err_nan() {
    let mut f = None;
    let src = r#"{"f":nan}"#;
    let mut desc = [("f", qjson::Schema::Float(&mut f))];
    let err = qjson::from_str::<_, 1>(src, &mut desc).unwrap_err();
    assert!(f.is_none());
    assert_eq!(err.kind(), qjson::ErrorKind::UnknownIdentifier);
    assert_eq!(err.lineno(), 1);
    assert_eq!(err.col(), 7);
}

#[test]
fn err_neg_nan() {
    let mut f = None;
    let src = r#"{"f":-nan}"#;
    let mut desc = [("f", qjson::Schema::Float(&mut f))];
    let err = qjson::from_str::<_, 1>(src, &mut desc).unwrap_err();
    assert!(f.is_none());
    assert_eq!(err.kind(), qjson::ErrorKind::InvalidNumber);
    assert_eq!(err.lineno(), 1);
    assert_eq!(err.col(), 6);
}

#[test]
fn err_inf() {
    let mut f = None;
    let src = r#"{"f":inf}"#;
    let mut desc = [("f", qjson::Schema::Float(&mut f))];
    let err = qjson::from_str::<_, 1>(src, &mut desc).unwrap_err();
    assert!(f.is_none());
    assert_eq!(err.kind(), qjson::ErrorKind::UnknownStartOfToken);
    assert_eq!(err.lineno(), 1);
    assert_eq!(err.col(), 6);
}

#[test]
fn err_neg_inf() {
    let mut f = None;
    let src = r#"{"f":-inf}"#;
    let mut desc = [("f", qjson::Schema::Float(&mut f))];
    let err = qjson::from_str::<_, 1>(src, &mut desc).unwrap_err();
    assert!(f.is_none());
    assert_eq!(err.kind(), qjson::ErrorKind::InvalidNumber);
    assert_eq!(err.lineno(), 1);
    assert_eq!(err.col(), 6);
}
