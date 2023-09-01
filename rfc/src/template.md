- **Feature Name:** *Fill me with pretty name and a unique ident [^unstable_feature] (E.g: New Feature (`new_feature`)*
- **Feature Request Issue:** *Link to issue*
- **RFC PR:** *Link to original PR*
- **Status:** *One of the following: [Under Review | Unstable | Stable | Cancelled]*
- **Version:** [0-9]\* *Increment this version whenever you open a new PR to update the RFC (not at every revision).
  Start with 0.*
- **Proof-of-concept:** *Optional field. If you have implemented a proof of concept, add a link here*

-------------------

## Summary

Short (1-2 sentences) description of the feature. What is this feature about?

## User Impact

Imagine this as your elevator pitch directed to users as well as Kani developers.
Why are we doing this?
Why should users care about this feature?
How will this benefit them?
What is the downside?

If this RFC is related to change in the architecture without major user impact,
think about the long term impact for user.
I.e.: what future work will this enable.
   - If you are unsure you need an RFC, please create a feature request issue and discuss the need with other Kani developers.

## User Experience

This should be a description on how users will interact with the feature.
Users should be able to read this section and understand how to use the feature.
**Do not include implementation details in this section, neither discuss the rationale behind the chosen UX.**

Please include:
  - High level user flow description.
  - Any new major functions or attributes that will be added to Kani library.
  - New command line options or subcommands (no need to mention the unstable flag).
  - List failure scenarios and how are they presented (e.g., compilation errors, verification failures, and possible failed user iterations).
  - Substantial changes to existing functionality or Kani output.

If the RFC is related to architectural changes and there are no visible changes to UX, please state so.
No further explanation is needed.

## Software Design

This is the beginning of the technical portion of the RFC.
From now on, your main audience is Kani developers, so it's OK to assume readers know Kani architecture.

Please provide a high level description your design.

- What are the main components that will be modified? (E.g.: changes to `kani-compiler`, `kani-driver`, metadata, proc-macros, installation...)
- Will there be changes to the components interface?
- Any changes to how these components communicate?
- What corner cases do you anticipate?

**We recommend you to leave this empty for the first version of your RFC**.

## Rationale and alternatives

This is the section where you discuss the decisions you made.

- What are the pros and cons of the UX? What would be the alternatives?
- What is the impact of not doing this?
- Any pros / cons on how you designed this?

## Open questions

List of open questions + an optional link to an issue that capture the work required to address the open question.
Capture the details of each open question in their respective issue, not here.

Example:
- Is there any use case that isn't handled yet?
- Is there any part of the UX that still needs some improvement?

Make sure all open questions are addressed before stabilization.

## Out of scope / Future Improvements

*Optional Section*: List of extensions and possible improvements that you predict for this feature that is out of
the scope of this RFC.

Feel free to add as many items as you want, but please refrain from adding too much detail.
If you want to capture your thoughts or start a discussion, please create a feature request.
You are welcome to add a link to the new issue here.

[^unstable_feature]: This unique ident should be used to enable features proposed in the RFC using `-Z <ident>` until the feature has been stabilized.
