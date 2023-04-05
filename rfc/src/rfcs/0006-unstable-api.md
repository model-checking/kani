- **Feature Name:** Unstable APIs (`unstable-api`)
- **RFC Tracking Issue**: <https://github.com/model-checking/kani/issues/2279>
- **RFC PR:** <https://github.com/model-checking/kani/pull/2281>
- **Status:** Under Review
- **Version:** 0

-------------------

## Summary

Provide a standard option for users to enable experimental APIs and features in Kani,
and ensure that those APIs are off by default.

## User Impact

Add an opt-in model for users to try experimental APIs.
The goal is to enable users to try features that aren't stable yet,
which allow us to get valuable feedback during the development of new features and APIs.

The opt-in model empowers the users to control when some instability is acceptable,
which makes Kani UX more consistent and safe.

Currently, each new unstable feature will introduce a new switch, some of them will look like `--enable-<feature>`,
while others will be a plain switch which allows further feature configuration `--<feature-config>=[value]`.
For example, we today have the following unstable switches `--enable-stubbing`, `--concrete-playback`, `--gen-c`.
In all cases, users are still required to provide the additional `--enable-unstable` option.
Some unstable features are included in the `--help` section, and only a few mention the requirement
to include `--enable-unstable`. There is no way to list all unstable features.
The the transition to stable switches is also ad-hoc.

In order to reduce friction, we will also standardize how users opt-in to any Kani unstable feature.
We will use similar syntax to the one used by the Rust compiler and Cargo.
As part of this work, we will also deprecate and remove `--enable-unstable` option.

Note that although Kani is still on v0, which means that everything is somewhat unstable,
this allow us to set different bars when it comes to what kind of changes is expected,
as well as what kind of support we will provide for a feature.

## User Experience

Users will have to invoke Kani with:
```
-Z <feature_identifier>
```
in order to enable any unstable feature in Kani, including unstable APIs in the Kani library.
For unstable command line options, we will add `-Z unstable-options`, similar to the Rust compiler.
E.g.:
```
-Z unstable-options --concrete-playback=print
```

Users will also be able to enable unstable features in their `Cargo.toml` in the `unstable` table
under `kani` table. E.g:
```toml
[package.metadata.kani.unstable]
unstable-options = true

[workspace.metadata.kani]
flags = { concrete-playback = true }
unstable = { unstable-options = true }
```

In order to mark an API as unstable, we will add the following attribute to the APIs marked as unstable:

```rust
#[kani::unstable(feature="<IDENTIFIER>", issue="<TRACKING_ISSUE_NUMBER>", reason="<OPTIONAL_DESCRIPTION>")]
pub fn unstable_api() {}
```

This is similar to the interface used by [the standard library](https://rustc-dev-guide.rust-lang.org/stability.html#unstable).

If the user tries to use an unstable feature in Kani without explicitly enabling it,
Kani will trigger an error. For unstable APIs, the error will be triggered during the crate
compilation.

## Detailed Design

We will add the `-Z` option to both `kani-driver` and `kani-compiler`.
Kani driver will pass the information to the compiler.

For unstable APIs, the compiler will check if any reachable function uses an unstable feature that was not enabled.
If that is the case, the compiler will trigger a compilation error.

We will also change the compiler to only generate code for harnesses that match the harness filter.
The filter is already passed to the compiler, but it is currently only used for stubbing.

### API Stabilization

Once an API has been stabilized, we will remove the `unstable` attributes from the given API.
If the user tries to enable a feature that was already stabilized,
Kani will print a warning stating that the feature has been stabilized.

### API Removal

If we decide to remove an API that is marked as unstable, we should follow a regular deprecation
path (using `#[deprecated]` attribute), and keep the `unstable` flag + attributes, until we are
ready to remove the feature completely.

## Rational and Alternatives

For this RFC, the suggestion is to only enable experimental features globally for simplicity of use and implementation.

For now, we will trigger a compilation error if an unstable API is reachable from a user crate
unless if the user opts in for the unstable feature.

We could allow users to specify experimental features on a per-harness basis,
but it could be tricky to make it clear to the user which harness may be affected by which feature.
The extra granularity would also be painful when we decide a feature is no longer experimental,
whether it is stabilized or removed.
In those cases, users would have to edit each harness that enables the affected feature.

## Open questions

- Should we also add a `stable` attribute that documents when an API was stabilized?

## Future possibilities

- Delay the error due to the usage of a unstable API, and only fail at runtime if the API is reachable.
- Allow users to enable unstable features on a per-harness basis.