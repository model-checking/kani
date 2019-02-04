# Differences between QuickCheck and Proptest

QuickCheck and Proptest are similar in many ways: both generate random
inputs for a function to check certain properties, and automatically shrink
inputs to minimal failing cases.

The one big difference is that QuickCheck generates and shrinks values
based on type alone, whereas Proptest uses explicit `Strategy` objects. The
QuickCheck approach has a lot of disadvantages in comparison:

- QuickCheck can only define one generator and shrinker per type. If you need a
  custom generation strategy, you need to wrap it in a newtype and implement
  traits on that by hand. In Proptest, you can define arbitrarily many
  different strategies for the same type, and there are plenty built-in.

- For the same reason, QuickCheck has a single "size" configuration that tries
  to define the range of values generated. If you need an integer between 0 and
  100 and another between 0 and 1000, you probably need to do another newtype.
  In Proptest, you can directly just express that you want a `0..100` integer
  and a `0..1000` integer.

- Types in QuickCheck are not easily composable. Defining `Arbitrary` and
  `Shrink` for a new struct which is simply produced by the composition of its
  fields requires implementing both by hand, including a bidirectional mapping
  between the struct and a tuple of its fields. In Proptest, you can make a
  tuple of the desired components and then `prop_map` it into the desired form.
  Shrinking happens automatically in terms of the input types.

- Because constraints on values cannot be expressed in QuickCheck, generation
  and shrinking may lead to a lot of input rejections. Strategies in Proptest
  are aware of simple constraints and do not generate or shrink to values that
  violate them.

The author of Hypothesis also has an [article on this
topic](http://hypothesis.works/articles/integrated-shrinking/).

Of course, there's also some relative downsides that fall out of what
Proptest does differently:

- Generating complex values in Proptest can be up to an order of magnitude
  slower than in QuickCheck. This is because QuickCheck performs stateless
  shrinking based on the output value, whereas Proptest must hold on to all the
  intermediate states and relationships in order for its richer shrinking model
  to work.
