# Testing

Testing in Kani is carried out in multiple ways. There are at least
two very good reasons to do it:
 1. **Software regression**: A regression is a type of bug
    that appears after a change is introduced where a feature that
    was previously working has unexpectedly stopped working.

    Regression testing allows one to prevent a software regression
    from happening by running a comprehensive set of working tests
    before any change is committed to the project.
 2. **Software metrics**: A metric is a measure of software
    characteristics which are quantitative and countable. Metrics are
    particularly valuable for project management purposes.

We recommend reading our section on [Regression Testing](#regression-testing)
if you are interested in Kani development. At present, we obtain metrics based
on the [book runner](./bookrunner.md).