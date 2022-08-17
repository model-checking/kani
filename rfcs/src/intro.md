# Introduction

Kani is an open-source verification tool that uses automated reasoning to analyze Rust programs. In order to 
integate feedback from developers and users on future changes to Kani, we decided to follow a light-weight 
"RFC" (request for comments) process.

## When to create an RFC

As a rule of thumb you should create an RFC if you are making changes that impact the user experience. New 
APIs, new features, as well as deprecation, should follow an RFC process. We also ask for substantial architectural 
changes to be made via RFC.

Bugs and smaller improvements to existing features do not require an RFC. If you are in doubt, feel free to create 
a [feature request](https://github.com/model-checking/kani/issues/new?assignees=&labels=&template=feature_request.md)
and discuss the next steps in the new issue.

## The RFC process

This is the overall workflow for the RFC process:

```toml
    Create RFC ──> Receive Feedback  ──> Accepted?
                                            │ Y
                                            ├───> Implement ───> Stabilize?
                                            │ N                      │ Y
                                            └───> Close PR           ├───> RFC Stable
                                                                     │ N
                                                                     └───> Remove feature
```

1. Create an RFC
   1. Start from a fork of the Kani repository.
   2. Copy the template file ([`rfcs/src/template.md`](./template.md)) to `rfcs/src/<my-feature>.md`.
   3. Fill in the details according to the template intructions.
   4. Submit a pull request.
2. Build consensus and integrate feedback.
   1. RFCs should get approved by at least 2 members of the `kani-developers` team.
   2. Once the RFC has been approved, update the RFC status and merge the PR.
   3. If the RFC is not approved, close the PR without merging.
3. Feature implementation.
   1. Start implementing the new feature in your fork.
   2. It is OK to implement it incrementally over multiple PRs. Just ensure that every pull request has a testable 
      end-to-end flow and that it is properly tested.
   3. In the implementation stage, the feature should only be accessible if the user explicitly passes 
      `--enable-unstable` as an argument to Kani.
   4. Document how to use the feature.
4. Stabilization.
   1. After the feature has been implemented, start the stabilization process.
   2. Gather user feedback and make necessary adjustments.
   3. Create a new PR that removes the `--enable-unstable` guard and that marks the RFC status as "STABLE". Also 
      make sure the RFC reflects the final implementation and user experience.
   5. In some cases, we might decide not to stabilize a feature. In those cases, please update the RFC status to 
      "CANCELLED" and remove the code that is no longer relevant.