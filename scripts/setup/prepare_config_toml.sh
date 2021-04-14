# Use config.toml template and set debugging options
cp config.toml.example config.toml \
  && sed -i"" \
    -e "s:^#llvm-config = <none> (path):llvm-config = \"/usr/bin/llvm-config-11\":" \
    -e "s/^#debug = false/debug = true/" \
    -e "s/^#debug-assertions-std = false/debug-assertions-std = false/" \
    -e "s/^#deny-warnings = true/deny-warnings = false/" \
    config.toml
