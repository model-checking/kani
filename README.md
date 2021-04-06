# Rust Model Checker (RMC)
The Rust Model Checker (RMC) aims to be a bit-precise model-checker for Rust.

## Disclaimer
RMC is currently in the initial development phase.
It **does not support all rust language features**.
Some unsupported (or partially supported) features will cause panics when RMC runs.
In other cases, RMC will "successfully" compile the rust code under test, but may inaccuratly give either false positives or false negatives.
If you encounter either false positives, or false negatives, please report them as an issue on this repository.


## Security
See [CONTRIBUTING](CONTRIBUTING.md#security-issue-notifications) for more information.

## Architecture
TODO

## Developer guide
TODO

## License
### Rust compiler
RMC is a fork of the rust compiler, which is primarily primarily distributed under the terms of both the MIT license and the Apache License (Version 2.0), with portions covered by various BSD-like licenses.

See [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT), and
[UPSTREAM-COPYRIGHT](UPSTREAM-COPYRIGHT) for details.

### RMC additions
RMC is Rdistributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT)for details.

## Quickstart

The following will build and test `rmc`:

```
./scripts/rmc-regression.sh
```

## Build frontend

```
cd RustToCBMC
cp config.toml.example config.toml
sed -i "" \
     -e "s/^#debug = .*/debug = true/" \
     -e "s/^#debug-assertions-std = .*/debug-assertions-std = false/" \
     -e "s/^#incremental = .*/incremental = true/" \
     -e "s/^#deny-warnings = .*/deny-warnings = false/" \
     config.toml
./x.py build -i --stage 1 library/std
export RMC_RUSTC=`find \`pwd\`/build -name "rustc" -print | grep stage1`
export PATH=`pwd`/scripts:$PATH
```

Note: You almost certainly want to use the local llvm installation instead
of building llvm from scratch. You do this by setting llvm-config to the
path of the local llvm-config tool in the target section of config.toml
for the target you are building. For example, on MacOS,
```
brew install llvm
echo '' >> config.toml
echo '[target.x86_64-apple-darwin]' >> config.toml
echo 'llvm-config = "/usr/local/opt/llvm/bin/llvm-config"' >> config.toml
```

Note: You almost certainly want full debug information for debugging
under gdb or lldb.  You do this by setting debuginfo-level-rustc to 2.
```
sed -i "" \
     -e "s/^#debuginfo-level-rustc = .*/debuginfo-level-rustc = 2/" \
     config.toml
```

## Install CBMC

CBMC has prebuilt releases [available for major platforms](https://github.com/diffblue/cbmc/releases).
RMC currently works with CBMC versions 5.26 or greater.

### Building CBMC from source

If you want to build CBMC from source, however, do
```
git clone https://github.com/diffblue/cbmc.git
cd cbmc
cmake -S. -Bbuild -DCMAKE_BUILD_TYPE=Release -DWITH_JBMC=OFF
cd build
make -j
export PATH=$(pwd)/bin:$PATH
```
