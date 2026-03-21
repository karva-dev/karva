# https://just.systems

test *args:
    rm -rf target/wheels
    maturin build
    @if command -v cargo-nextest > /dev/null 2>&1; then \
        cargo nextest run {{args}}; \
    else \
        cargo test {{args}}; \
    fi

coverage *args:
    rm -rf target/wheels
    maturin build
    RUSTFLAGS="-C instrument-coverage -C llvm-args=--instrprof-atomic-counter-update-all" cargo build --target-dir target/llvm-cov-target -p karva_worker
    __KARVA_COVERAGE=1 cargo llvm-cov nextest -p karva --no-report {{args}}
    {{LLVM_PROFDATA}} merge -failure-mode=warn target/llvm-cov-target/*.profraw -o target/llvm-cov-target/merged.profdata
    {{LLVM_COV}} report target/llvm-cov-target/debug/karva -object target/llvm-cov-target/debug/karva-worker -instr-profile=target/llvm-cov-target/merged.profdata -ignore-filename-regex='(\.cargo|rustc-|/rustlib/|\.claude/)'
    {{LLVM_COV}} show target/llvm-cov-target/debug/karva -object target/llvm-cov-target/debug/karva-worker -instr-profile=target/llvm-cov-target/merged.profdata -ignore-filename-regex='(\.cargo|rustc-|/rustlib/|\.claude/)' --format=html -output-dir=target/coverage-html
    rm -f default_*.profraw

# Set LLVM_COV and LLVM_PROFDATA for Homebrew LLVM installations
export LLVM_COV := env("LLVM_COV", "/opt/homebrew/opt/llvm/bin/llvm-cov")
export LLVM_PROFDATA := env("LLVM_PROFDATA", "/opt/homebrew/opt/llvm/bin/llvm-profdata")
