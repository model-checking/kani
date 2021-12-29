use crate::spec::Target;

pub fn target() -> Target {
    let mut base = super::windows_msvc_base::opts();
    base.max_atomic_width = Some(64);
    base.features = "+neon,+fp-armv8".to_string();

    Target {
        llvm_target: "aarch64-pc-windows-msvc".to_string(),
        pointer_width: 64,
        data_layout: "e-m:w-p:64:64-i32:32-i64:64-i128:128-n32:64-S128".to_string(),
        arch: "aarch64".to_string(),
        options: base,
    }
}
