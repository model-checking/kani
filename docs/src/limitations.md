# Limitations

Like other tools, Kani comes with some limitations. In some cases, these
limitations are inherent because of the techniques it's based on, or the
undecidability of the properties that Kani seeks to prove. In other
cases, it's just a matter of time and effort to remove these limitations (e.g.,
specific unsupported Rust language features).

In this chapter, we do the following to document these limitations:
 * Discuss the effect of [Rust undefined behaviour](./undefined-behaviour.md).
 * Summarize the [current support for Rust features](./rust-feature-support.md).
 * Explain the need for [overrides](./overrides.md) and list all overriden
   symbols.
