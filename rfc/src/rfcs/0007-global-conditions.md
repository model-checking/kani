- **Feature Name:** Global Conditions (`global-conditions`)
- **Feature Request Issue:** <https://github.com/model-checking/kani/issues/2525>
- **RFC PR:** <https://github.com/model-checking/kani/pull/2516>
- **Status:** *Under Review*
- **Version:** 0
- **Proof-of-concept:** TBD

## Summary

A new section in Kani's output to summarize the status of properties that depend on other properties. We use the term *global conditions* to refer to such properties. 

## User Impact

The addition of new options that affect the overall verification result depending on certain property attributes demands some consideration.
In particular, the addition of a new option to fail verification if there are uncoverable (i.e., unsatisfiable or unreachable) `cover` properties (requested in [#2299](https://github.com/model-checking/kani/issues/2299)) is posing new challenges to our current architecture and UI.

This concept isn't made explicit in Kani, but exists in some ways.
For example, the `kani::should_panic` attribute is a global condition because it can be described in terms of other properties (checks).
The request in [#2299](https://github.com/model-checking/kani/issues/2299) is essentially another global conditions, and we may expect more to be requested in the future.

In this RFC, we propose a new section in Kani's output focused on reporting global conditions.
The goal is for users to receive useful information about hyperproperties without it becoming overwhelming.
This will help users to understand better options that are enabled through global conditions and ease the addition of such options to Kani.

## User Experience

**The output will refer to properties that depend on other properties as "global conditions"**, which is a simpler term.
The options to enable different global conditions will depend on a case-by-case basis.

The main UI change in this proposal is a new `GLOBAL CONDITIONS` section that **won't be printed if no global conditions have been enabled**.
This section will only appear in Kani's default output after the `RESULTS` section (used for individual checks) and have the format:

```
GLOBAL CONDITIONS:
 - `<name>`: <status> (<reason>)
 - `<name>`: <status> (<reason>)
 [...]
```

where:
  - `<name>` is the name given to the global condition.
  - `<status>` is the status determined for the global condition.
  - `<reason>` is an explanation that depends on the status of the global condition.

For example, let's assume we implement the option requested in [#2299](https://github.com/model-checking/kani/issues/2299).
A concrete example of this output would be:

```
GLOBAL CONDITIONS:
 - `fail_uncoverable`: SUCCESS (all cover statements were satisfied as expected)
```

A `FAILED` status in any enabled global condition will cause verification to fail, pointing out that one or more global conditions were failed as in:

```
VERIFICATION:- FAILURE (one or more global conditions failed)
```

This last UI change will also be implemented for the terse output.

**Global conditions which aren't enabled won't appear in the `GLOBAL CONDITIONS` section**.
Their status will be computed regardless[^status-computation], and we may consider showing this status when the `--verbose` option is passed.

The documentation of global conditions will depend on how they're enabled, which depends on a case-by-case basis.
However, we may consider adding a new subsection `Global conditions` to the `Reference` section that collects all of them so it's easier for users to consult all of them in one place.

## Detailed Design

The only component to be modified is `kani-driver` since that's where verification results are built and determined.
But given the growing complexity of `kani-driver`, we should consider moving the logic related to global conditions onto a new crate.

We don't need new dependencies.
The corner cases will depend on the specific global conditions to be implemented.

## Rationale and alternatives

As mentioned earlier, we're proposing this change to help users understand global conditions and how they're determined.
In many cases, global conditions empower users to write harnesses which weren't possible to write before.
As an example, the `#[kani::should_panic]` attribute allowed users to write harnesses expecting panic-related failures.

On the other hand, this proposal will add a significant amount of code to `kani-driver`.
It's possible to move some code into another crate, but even then some code will be added in `kani-driver`.

Also, we don't really know if more global conditions will be requested in the future.
We may consider discarding this proposal and waiting for the next feature that can be implemented as a global condition to be requested.

### Alternative: Global conditions as regular checks

One option we've considered in the past is to enable global conditions as a regular checks.
While it's technically doable, it doesn't feel appropriate for global conditions to reported through regular checks since generally a higher degree of visibility may be appreciated.

## Open questions

No open questions.

## Future possibilities

A redesign of Kani's output is likely to change the style/architecture to report global conditions.

[^status-computation]: The results for global conditions would be computed during postprocessing based on the results of other checks.
Global conditions' checks aren't part of the SAT, therefore this computation won't impact verification time.
