// frontend/enhance_llm.rs
use anyhow::Result;
use serde_json::json;

use crate::frontend::json_handler::JsonHandler;
use crate::harness_runner::HarnessResult;     // 你已有的类型
use kani_metadata::HarnessMetadata;            // 你已有的类型
use crate::call_cbmc::VerificationStatus;      // 用于状态判断

/// Post-process the run into an LLM-friendly summary section.
/// This does NOT change verification logic; it only augments the JSON.
pub fn enhance_llm(
    handler: &mut JsonHandler,
    results: &[HarnessResult<'_>],
    harnesses: &[&HarnessMetadata],
) -> Result<()> {
    // 统计口径示例：成功/失败数量 + 失败列表
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut failing_names = Vec::new();

    for r in results {
        match r.result.status {
            VerificationStatus::Success => passed += 1,
            VerificationStatus::Failure => {
                failed += 1;
                failing_names.push(r.harness.pretty_name.clone());
            }
        }
    }

    // 也可以按 harness 顺序写更丰富的自然语言摘要，这里先放一个简单版本
    let summary_text = if failed == 0 {
        format!("All {} harnesses verified successfully.", passed)
    } else {
        format!(
            "{} passed, {} failed. Failing: {}",
            passed,
            failed,
            failing_names.join(", ")
        )
    };

    // 把一个简单的结构写进顶层 "llm" 节点（你也可以换成 nested 结构）
    handler.add_item(
        "llm",
        json!({
            "summary": {
                "total": harnesses.len(),
                "passed": passed,
                "failed": failed,
            },
            "failing_harnesses": failing_names,
            "note": summary_text
        }),
    );

    Ok(())
}
