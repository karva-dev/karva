# https://just.systems

test *args:
    rm -rf target/wheels
    uvx maturin build
    @if command -v cargo-nextest > /dev/null 2>&1; then \
        cargo nextest run {{args}}; \
    else \
        cargo test {{args}}; \
    fi

coverage *args:
    #!/usr/bin/env bash
    set -euo pipefail

    # Find llvm-cov and llvm-profdata from PATH, rustup sysroot, or Homebrew
    find_llvm_tool() {
        local tool="$1"
        if command -v "$tool" > /dev/null 2>&1; then echo "$tool"; return; fi
        local sysroot_bin="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | grep host | awk '{print $2}')/bin/$tool"
        if [ -x "$sysroot_bin" ]; then echo "$sysroot_bin"; return; fi
        local brew="/opt/homebrew/opt/llvm/bin/$tool"
        if [ -x "$brew" ]; then echo "$brew"; return; fi
        echo "error: could not find $tool" >&2; exit 1
    }
    LLVM_COV=$(find_llvm_tool llvm-cov)
    LLVM_PROFDATA=$(find_llvm_tool llvm-profdata)
    export LLVM_COV LLVM_PROFDATA

    rm -rf target/wheels
    uvx maturin build
    find target/llvm-cov-target -name '*.profraw' -delete 2>/dev/null || true
    RUSTFLAGS="-C instrument-coverage -C llvm-args=--instrprof-atomic-counter-update-all" cargo build --target-dir target/llvm-cov-target -p karva_worker
    __KARVA_COVERAGE=1 cargo llvm-cov nextest --no-report {{args}}
    find target/llvm-cov-target -name '*.profraw' > target/llvm-cov-target/profraw-files.txt
    "$LLVM_PROFDATA" merge -failure-mode=warn -f target/llvm-cov-target/profraw-files.txt -o target/llvm-cov-target/merged.profdata
    "$LLVM_COV" report target/llvm-cov-target/debug/karva -object target/llvm-cov-target/debug/karva-worker -instr-profile=target/llvm-cov-target/merged.profdata -ignore-filename-regex='(\.cargo|rustc-|/rustlib/|\.claude/)'
    "$LLVM_COV" show target/llvm-cov-target/debug/karva -object target/llvm-cov-target/debug/karva-worker -instr-profile=target/llvm-cov-target/merged.profdata -ignore-filename-regex='(\.cargo|rustc-|/rustlib/|\.claude/)' --format=html -output-dir=target/coverage-html
    "$LLVM_COV" export target/llvm-cov-target/debug/karva -object target/llvm-cov-target/debug/karva-worker -instr-profile=target/llvm-cov-target/merged.profdata -ignore-filename-regex='(\.cargo|rustc-|/rustlib/|\.claude/)' --format=lcov > target/coverage.lcov
    find . -maxdepth 1 -name 'default_*.profraw' -delete 2>/dev/null || true
