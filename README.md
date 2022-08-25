# _quick.rs_

![ci_status](https://github.com/rjsberry/quick.rs/actions/workflows/ci.yml/badge.svg)

Public domain single file Rust libraries.

| library   | description                                          | dependencies          | optional dependencies |
|-----------|------------------------------------------------------|-----------------------|-----------------------|
| [`qbump*`] | bump allocation backed by static buffers             | `libcore` `liballoc` | `atomic-polyfill`     |
| [`qcell`] | thread-safe lock-free interior mutability primitives | `libcore`             | `atomic-polyfill`     |
| [`qini`]  | .ini parser                                          | `libcore`             |                       |
| [`qjson`] | json deserializer                                    | `libcore`             |                       |
| [`qptr`]  | allocation-free smart pointers and trait objects     | `libcore`             |                       |

>  \*nightly

[`qbump*`]: qbump/qbump.rs
[`qcell`]: qcell/qcell.rs
[`qini`]: qini/qini.rs
[`qjson`]: qjson/qjson.rs
[`qptr`]: qptr/qptr.rs
