- **Feature Name:** *Fill me with pretty name and a unique ident. E.g: New Feature (`new_feature`)*
- **Feature Request Issue:** *Link to issue*
- **RFC PR:** *Link to original PR*
- **Status:** *One of the following: [Under Review | Unstable | Stable | Cancelled]*
- **Version:** [0-9]\* *Increment this version whenever you open a new PR to update the RFC (not at every revision).
  Start with 0.*
- **Proof-of-concept:** *Optional field. If you have implemented a proof of concept, add a link here*

-------------------

## Summary

Short description of the feature, i.e.: the elevator pitch. What is this feature about?

## User Impact

Why are we doing this? How will this benefit the final user?

 - If this is an API change, how will that impact current users?
 - For deprecation or breaking changes, how will the transition look like?
 - If this RFC is related to change in the architecture without major user impact, think about the long term
impact for user. I.e.: what future work will this enable.

## User Experience

What is the scope of this RFC? Which use cases do you have in mind? Explain how users will interact with it. Also
please include:

- How would you teach this feature to users? What changes will be required to the user documentation?
- If the RFC is related to architectural changes and there are no visible changes to UX, please state so.

## Detailed Design

This is the technical portion of the RFC. Please provide high level details of the implementation you have in mind:

- What are the main components that will be modified? (E.g.: changes to `kani-compiler`, `kani-driver`, metadata,
  installation...)
- How will they be modified? Any changes to how these components communicate?
- Will this require any new dependency?
- What corner cases do you anticipate?

## Rationale and alternatives

- What are the pros and cons of this design?
- What is the impact of not doing this?
- What other designs have you considered? Why didn't you choose them?

## Open questions

- Is there any part of the design that you expect to resolve through the RFC process?
- What kind of user feedback do you expect to gather before stabilization? How will this impact your design?

## Future possibilities

What are natural extensions and possible improvements that you predict for this feature that is out of the
scope of this RFC? Feel free to brainstorm here.