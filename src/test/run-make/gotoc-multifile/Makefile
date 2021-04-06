# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT
-include ../../run-make-fulldeps/tools.mk

all:
	$(RUSTC) foo.rs
	$(RUSTC) -Z codegen-backend=gotoc foo.rs
	symtab2gb $(TMPDIR)/foo.json --out $(TMPDIR)/foo.o
	$(RUSTC) -Z codegen-backend=gotoc bar.rs
	symtab2gb $(TMPDIR)/bar.json --out $(TMPDIR)/bar.o
	goto-cc $(TMPDIR)/foo.o $(TMPDIR)/bar.o -o $(TMPDIR)/mix

