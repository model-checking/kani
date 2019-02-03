# Proptest from the Bottom Up

This tutorial will introduce proptest from the bottom up, starting from the
basic building blocks, in the hopes of making the model as a whole clear.
In particular, we'll start off without using the macros so that the macros
can later be understood in terms of what they expand into rather than
magic. But as a result, the first part is _not_ representative of how
proptest is normally used. If bottom-up isn't your style, you may wish to
skim the first few sections.

Also note that the examples here focus on the usage of proptest itself, and
as such generally have trivial test bodies. In real code, you would
obviously have assertions and so forth in the test bodies.
