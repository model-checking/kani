use crate::spec::{SanitizerSet, Target, TargetOptions};

pub fn target() -> Target {
    Target {
        llvm_target: "aarch64-fuchsia".to_string(),
        pointer_width: 64,
        data_layout: "e-m:e-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128".to_string(),
        arch: "aarch64".to_string(),
        options: TargetOptions {
            max_atomic_width: Some(128),
            supported_sanitizers: SanitizerSet::ADDRESS | SanitizerSet::CFI,
            ..super::fuchsia_base::opts()
        },
    }
}
