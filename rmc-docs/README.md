## RMC documentation development

A good trick when developing RMC on a remote machine is to SSH forward to test documentation changes.

```
ssh -t -L 3000:127.0.0.1:3000 rmc-host 'cd rmc/rmc-docs/ && ./mdbook serve'
```

This command will connect to `rmc-host` where it assumes RMC is checked out in `rmc/` and the documentation has been built once successfully.
It will automatically detect changes to the docs and rebuild, allowing you to quickly refresh in your local browser when you visit: `http://127.0.0.1:3000/`

## Documentation tests

The code in `src/tutorial/` is tested with `compiletest`.
This means each file should be buildable independently (i.e. you can run `rmc` on each `.rs` file).
It also means the necessary `rmc-` flag-comments must appear in each file.

To run just these tests, return to the RMC root directory and run:

```
./x.py test -i --stage 1 rmc-docs
```
