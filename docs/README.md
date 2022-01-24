## Kani documentation development

A good trick when developing Kani on a remote machine is to SSH forward to test documentation changes.

```
ssh -t -L 3000:127.0.0.1:3000 kani-host 'cd kani/docs/ && ./mdbook serve'
```

This command will connect to `kani-host` where it assumes Kani is checked out in `kani/` and the documentation has been built once successfully.
It will automatically detect changes to the docs and rebuild, allowing you to quickly refresh in your local browser when you visit: `http://127.0.0.1:3000/`

## Documentation tests

The code in `src/tutorial/` is tested with `compiletest`.
This means each file should be buildable independently (i.e. you can run `kani` on each `.rs` file).
It also means the necessary `kani-` flag-comments must appear in each file.

To run just these tests, return to the Kani root directory and run:

```
COMPILETEST_FORCE_STAGE0=1 ./x.py test -i --stage 0 kani-docs
```
