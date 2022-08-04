OUTDIR := 'target'

alias ba := build-all
alias b  := build
alias ta := test-all
alias t  := test
alias d  := doc
alias c  := clean

# Build all libs
@build-all: (_iterate 'build')

# Build a specific lib
build lib: (_lib_exists lib)
    rustc --crate-type=lib --crate-name {{lib}} {{lib}}.rs --out-dir={{OUTDIR}}

# Test all libs
@test-all: (_iterate 'test')

# Test a specific lib
test lib: (build lib)
    rustc -L{{OUTDIR}} --extern {{lib}} --test {{lib}}_test.rs --out-dir={{OUTDIR}}
    target/{{lib}}_test --quiet

# Show doucmentation for a specific lib
doc lib: (_lib_exists lib)
    rustdoc {{lib}}.rs --out-dir {{OUTDIR}}/doc
    open {{OUTDIR}}/doc/{{lib}}/index.html

# Remove build artifacts
clean:
    rm -rf {{OUTDIR}}

# Emit `rust-analyzer` configuration in `rust-project.json` format
@emit_rust_project_json:
    rustup component list | grep rust-src >/dev/null || rustup component add rust-src
    echo "{"
    echo "    "\""sysroot_src"\"": "\""$(rustc --print=sysroot)/lib/rustlib/src/rust/library"\"","
    echo "    "\""crates"\"": ["
    echo "        {"
    echo "            "\""root_module"\"": "\""$(rustc --print=sysroot)/lib/rustlib/src/rust/library/std/src/lib.rs"\"","
    echo "            "\""edition"\"": "\""2021"\"","
    echo "            "\""deps"\"": []"
    echo "        },"
    echo "        {"
    echo "            "\""root_module"\"": "\""ini.rs"\"","
    echo "            "\""edition"\"": "\""2021"\"","
    echo "            "\""deps"\"": ["\""libstd"\""]"
    echo "        }"
    echo "    ]"
    echo "}"
    


# Check if a lib exists
@_lib_exists lib:
    [[ -f {{lib}}.rs ]]

# Run a recipe for every lib
@_iterate recipe:
    for lib in *.rs; do if [[ $lib != *"_test.rs" ]]; then just {{recipe}} ${lib%.rs}; fi; done
