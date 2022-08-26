macro_rules! test_ok {
    (
        $name:ident,
        $ini:literal,
        [$(($section:literal, $key:literal, $value:literal)),+ $(,)?] $(,)?
    ) => {
        #[test]
        fn $name() {
            let mut params = qini::parse($ini)
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            params.reverse();

            for (section, key, value) in [$(($section, $key, $value)),+] {
                let param = params.pop().unwrap();
                assert_eq!(param.section, section);
                assert_eq!(param.key, key);
                assert_eq!(param.value, value);
            }

            assert!(params.is_empty());
        }
    }
}

macro_rules! test_err {
    (
        $name:ident,
        $ini:literal,
        $lineno:literal,
        $kind:expr $(,)?
    ) => {
        #[test]
        fn $name() {
            let err = qini::parse($ini)
                .collect::<Result<Vec<_>, _>>()
                .unwrap_err();

            assert_eq!(err.lineno(), $lineno);
            assert_eq!(err.kind(), $kind);
        }
    };
}

test_ok! {
    key_value_equals_delimiter,
    "foo = 1",
    [("", "foo", "1")]
}

test_ok! {
    key_value_colon_delimiter,
    "foo : 1",
    [("", "foo", "1")]
}

test_ok! {
    no_sections_lf,
    "foo = 1\nbar = 2\n",
    [("", "foo", "1"), ("", "bar", "2")],
}

test_ok! {
    no_sections_crlf,
    "foo = 1\r\nbar = 2\r\n",
    [("", "foo", "1"), ("", "bar", "2")],
}

test_ok! {
    one_sections,
    "[foo]\nfoo = 1\nbar = 2",
    [("foo", "foo", "1"), ("foo", "bar", "2")],
}

test_ok! {
    multiple_sections,
    r#"
    [foo]
    foo = 1
    bar = 2

    [bar]
    foo = 3
    bar = 4
    "#,
    [
        ("foo", "foo", "1"),
        ("foo", "bar", "2"),
        ("bar", "foo", "3"),
        ("bar", "bar", "4"),
    ],
}

test_ok! {
    global_vars_with_sections,
    r#"
    foo = 1
    bar = 2

    [foo]
    foo = 3
    bar = 4

    [bar]
    foo = 5
    bar = 6
    "#,
    [
        ("", "foo", "1"),
        ("", "bar", "2"),
        ("foo", "foo", "3"),
        ("foo", "bar", "4"),
        ("bar", "foo", "5"),
        ("bar", "bar", "6"),
    ],
}

test_ok! {
    empty_value,
    "foo =",
    [("", "foo", "")],
}

test_ok! {
    comment_semicolon,
    "; comment\nfoo = 1",
    [("", "foo", "1")],
}

test_ok! {
    comment_octothorpe,
    "# comment\nfoo = 1",
    [("", "foo", "1")],
}

test_ok! {
    inline_comments_semicolon_part_of_value,
    "foo = 1 ; comment",
    [("", "foo", "1 ; comment")],
}

test_ok! {
    inline_comments_octothorpe_part_of_value,
    "foo = 1 # comment",
    [("", "foo", "1 # comment")],
}

test_ok! {
    repeated_keys,
    "foo = 1\nfoo = 2",
    [("", "foo", "1"), ("", "foo", "2")],
}

test_ok! {
    multiple_key_value_delimiters_equals_equals,
    "foo = 1 = 2",
    [("", "foo", "1 = 2")],
}

test_ok! {
    multiple_key_value_delimiters_colon_colon,
    "foo : 1 : 2",
    [("", "foo", "1 : 2")],
}

test_ok! {
    mixed_key_value_delimiters_equals_colon,
    "foo = 1 : 2",
    [("", "foo", "1 : 2")],
}

test_ok! {
    mixed_key_value_delimiters_colon_equals,
    "foo : 1 = 2",
    [("", "foo", "1 = 2")],
}

test_ok! {
    subsections,
    "[foo.bar]\nbaz = 1",
    [("foo.bar", "baz", "1")],
}

test_err! {
    empty_section,
    "[]",
    1,
    qini::ErrorKind::InvalidSection,
}

test_err! {
    empty_key,
    "= 1",
    1,
    qini::ErrorKind::InvalidKey,
}

test_err! {
    missing_key_value_delimiter,
    "foo",
    1,
    qini::ErrorKind::UnexpectedEol,
}

test_err! {
    comment_after_section,
    "[foo] ; disallowed",
    1,
    qini::ErrorKind::UnexpectedEol,
}

test_err! {
    invalid_symbol_in_section,
    "[bad section name]",
    1,
    qini::ErrorKind::InvalidSection,
}

test_err! {
    invalid_symbol_in_key,
    "bad key name = value",
    1,
    qini::ErrorKind::InvalidKey,
}
