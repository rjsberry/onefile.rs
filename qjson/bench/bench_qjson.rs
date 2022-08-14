use criterion::{black_box, criterion_group, criterion_main, Criterion};
use qjson::Schema as QSchema;

const DATA: &str = r#"{
    "employees": [
        {
            "name": "John Smith",
            "id": 1000,
            "phones": ["+44 1234 5678"],
            "bonus": 0.1
        },
        {
            "name": "Jane Doe",
            "id": 1001,
            "phones": ["+44 8765 4321", "+44 4321 5678"],
            "bonus": 0.2
        }
    ]
}"#;

pub fn bench_qjson(c: &mut Criterion) {
    #[derive(Default)]
    struct Employees<'a> {
        employees: [Employee<'a>; 2],
    }

    #[derive(Default)]
    struct Employee<'a> {
        name: Option<&'a str>,
        id: Option<i64>,
        phones: [Option<&'a str>; 2],
        bonus: Option<f64>,
    }

    c.bench_function("qjson(employees)", |b| {
        b.iter(|| {
            let mut es = Employees::default();

            let Employees {
                employees:
                    [Employee {
                        name: e0_name,
                        id: e0_id,
                        phones: [e0_phone0, e0_phone1],
                        bonus: e0_bonus,
                    }, Employee {
                        name: e1_name,
                        id: e1_id,
                        phones: [e1_phone0, e1_phone1],
                        bonus: e1_bonus,
                    }],
            } = &mut es;

            let mut e0_phones_desc = [QSchema::Str(e0_phone0), QSchema::Str(e0_phone1)];
            let mut e0_desc = [
                ("name", QSchema::Str(e0_name)),
                ("id", QSchema::Integer(e0_id)),
                ("phones", QSchema::Array(&mut e0_phones_desc)),
                ("bonus", QSchema::Float(e0_bonus)),
            ];

            let mut e1_phones_desc = [QSchema::Str(e1_phone0), QSchema::Str(e1_phone1)];
            let mut e1_desc = [
                ("name", QSchema::Str(e1_name)),
                ("id", QSchema::Integer(e1_id)),
                ("phones", QSchema::Array(&mut e1_phones_desc)),
                ("bonus", QSchema::Float(e1_bonus)),
            ];

            let mut es_desc = [QSchema::Object(&mut e0_desc), QSchema::Object(&mut e1_desc)];
            let mut desc = [("employees", QSchema::Array(&mut es_desc))];

            black_box(qjson::from_str::<_, 2>(black_box(DATA), &mut desc)).unwrap();
        });
    });
}

pub fn bench_serde_json(c: &mut Criterion) {
    #[derive(serde_derive::Deserialize)]
    #[allow(dead_code)]
    struct Employees<'a> {
        #[serde(borrow)]
        employees: Vec<Employee<'a>>,
    }

    #[derive(serde_derive::Deserialize)]
    #[allow(dead_code)]
    struct Employee<'a> {
        name: &'a str,
        id: i64,
        phones: Vec<&'a str>,
        bonus: f64,
    }

    c.bench_function("serde_json(employees)", |b| {
        b.iter(|| {
            let _es: Employees<'_> = black_box(serde_json::from_str(black_box(DATA)).unwrap());
        });
    });
}

criterion_group!(benches, bench_qjson, bench_serde_json);
criterion_main!(benches);
