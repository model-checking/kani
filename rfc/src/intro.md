# Introduction

Kani is an open-source verification tool that uses automated reasoning to analyze Rust programs. In order to
integrate feedback from developers and users on future changes to Kani, we decided to follow a light-weight
"RFC" (request for comments) process.

## When to create an RFC

You should create an RFC in one of two cases:

1. The change you are proposing would be a "one way door": e.g. a major change to the public API, a new feature that would be difficult to modify once released, etc.
2. The change you are making has a significant design component, and would benefit from a design review.

Bugs and improvements to existing features do not require an RFC.
If you are in doubt, feel free to create  a [feature request](https://github.com/model-checking/kani/issues/new?assignees=&labels=&template=feature_request.md) and discuss the next steps in the new issue.
Your PR reviewer may also request an RFC if your change appears to fall into category 1 or 2.

You do not necessarily need to create an RFC immediately. It is our experience that it is often best to write some "proof of concept" code to test out possible ideas before writing the formal RFC.

## The RFC process

This is the overall workflow for the RFC process:

```toml
    Create RFC ──> Receive Feedback  ──> Accepted?
                        │  ∧                  │ Y
                        ∨  │                  ├───> Implement ───> Test + Feedback ───> Stabilize?
                       Revise                 │ N                                          │ Y
                                              └───> Close PR                               ├───> RFC Stable
                                                                                           │ N
                                                                                           └───> Remove feature
```

1. Create an RFC
   1. Create a tracking issue for your RFC (e.g.: [Issue-1588](https://github.com/model-checking/kani/issues/1588)).
   2. Start from a fork of the Kani repository.
   3. Copy the template file ([`rfc/src/template.md`](./template.md)) to `rfc/src/rfcs/<ID_NUMBER><my-feature>.md`.
   4. Fill in the details according to the template instructions.
     - For the first RFC version, we recommend that you leave the "Software Design" section empty.
     - Focus on the user impact and user experience.
       Include a few usage examples if possible.
   5. Add a link to the new RFC inside [`rfc/src/SUMMARY.md`](https://github.com/model-checking/kani/blob/main/rfc/src/SUMMARY.md)
   6. Submit a pull request.
2. Build consensus and integrate feedback.
   1. RFCs should get approved by at least 2 Kani developers.
   2. Once the RFC has been approved, update the RFC status and merge the PR.
   3. If the RFC is not approved, close the PR without merging.
3. Feature implementation.
   1. Start implementing the new feature in your fork.
   2. Create a new revision of the RFC to add details about the "Software Design".
   3. It is OK to implement the feature incrementally over multiple PRs.
      Just ensure that every pull request has a testable end-to-end flow and that it is properly tested.
   4. In the implementation stage, the feature should only be accessible if the user explicitly passes
      `-Z <FEATURE_ID>` as an argument to Kani.
   5. Document how to use the feature.
   6. Keep the RFC up-to-date with the decisions you make during implementation.
4. Test and Gather Feedback.
   1. Fix major issues related to the new feature.
   2. Gather user feedback and make necessary adjustments.
   3. Resolve RFC open questions.
   4. Add regression tests to cover all expected behaviors and unit tests whenever possible.
5. Stabilization.
   1. Propose to stabilize the feature when feature is well tested and UX has received positive feedback.
   2. Create a new PR that removes the `-Z <FEATURE_ID>` guard and that marks the RFC status as "STABLE".
      1. Make sure the RFC reflects the final implementation and user experience.
   3. In some cases, we might decide not to incorporate a feature
      (E.g.: performance degradation, bad user experience, better alternative).
      In those cases, please update the RFC status to "CANCELLED as per <PR_LINK | ISSUE_LINK>" and remove the code
      that is no longer relevant.
   4. Close the tracking issue.
