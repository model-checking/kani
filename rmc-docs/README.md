## RMC documentation development

A good trick when developing RMC on a remote machine is to SSH forward to test documentation changes.

```
ssh -t -L 3000:127.0.0.1:3000 rmc-host 'cd rmc/rmc-docs/ && ./mdbook serve'
```

This command will connect to `rmc-host` where it assumes RMC is checked out in `rmc/` and the documentation has been built once successfully.
It will automatically detect changes to the docs and rebuild, allowing you to quickly refresh in your local browser when you visit: `http://127.0.0.1:3000/`
