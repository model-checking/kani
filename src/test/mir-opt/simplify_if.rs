#[inline(never)]
fn noop() {}

// EMIT_MIR simplify_if.main.SimplifyConstCondition-after-const-prop.diff
fn main() {
    if false {
        noop();
    }
}
