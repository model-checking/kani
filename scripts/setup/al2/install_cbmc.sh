#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# Source kani-dependencies to get CBMC_VERSION
source kani-dependencies

if [ -z "${CBMC_VERSION:-}" ]; then
  echo "$0: Error: CBMC_VERSION is not specified"
  exit 1
fi

# Binaries are not released for AL2, so build from source
WORK_DIR=$(mktemp -d)
git clone \
  --branch cbmc-${CBMC_VERSION} --depth 1 \
  https://github.com/diffblue/cbmc \
  "${WORK_DIR}"

pushd "${WORK_DIR}"

# apply workaround for https://github.com/diffblue/cbmc/issues/8357 until it is
# properly fixed in CBMC
cat > varargs.patch << "EOF"
--- a/src/ansi-c/library/stdio.c
+++ b/src/ansi-c/library/stdio.c
@@ -1135,7 +1135,7 @@ int vfscanf(FILE *restrict stream, const char *restrict format, va_list arg)

   (void)*format;
   while((__CPROVER_size_t)__CPROVER_POINTER_OFFSET(*(void **)&arg) <
-        __CPROVER_OBJECT_SIZE(arg))
+        __CPROVER_OBJECT_SIZE(*(void **)&arg))
   {
     void *a = va_arg(arg, void *);
     __CPROVER_havoc_object(a);
@@ -1233,7 +1233,7 @@ int __stdio_common_vfscanf(

   (void)*format;
   while((__CPROVER_size_t)__CPROVER_POINTER_OFFSET(*(void **)&args) <
-        __CPROVER_OBJECT_SIZE(args))
+        __CPROVER_OBJECT_SIZE(*(void **)&args))
   {
     void *a = va_arg(args, void *);
     __CPROVER_havoc_object(a);
@@ -1312,7 +1312,7 @@ __CPROVER_HIDE:;
   (void)*s;
   (void)*format;
   while((__CPROVER_size_t)__CPROVER_POINTER_OFFSET(*(void **)&arg) <
-        __CPROVER_OBJECT_SIZE(arg))
+        __CPROVER_OBJECT_SIZE(*(void **)&arg))
   {
     void *a = va_arg(arg, void *);
     __CPROVER_havoc_object(a);
@@ -1388,7 +1388,7 @@ int __stdio_common_vsscanf(
   (void)*s;
   (void)*format;
   while((__CPROVER_size_t)__CPROVER_POINTER_OFFSET(*(void **)&args) <
-        __CPROVER_OBJECT_SIZE(args))
+        __CPROVER_OBJECT_SIZE(*(void **)&args))
   {
     void *a = va_arg(args, void *);
     __CPROVER_havoc_object(a);
@@ -1774,12 +1774,12 @@ int vsnprintf(char *str, size_t size, const char *fmt, va_list ap)
   (void)*fmt;

   while((__CPROVER_size_t)__CPROVER_POINTER_OFFSET(*(void **)&ap) <
-        __CPROVER_OBJECT_SIZE(ap))
+        __CPROVER_OBJECT_SIZE(*(void **)&ap))

   {
     (void)va_arg(ap, int);
     __CPROVER_precondition(
-      __CPROVER_POINTER_OBJECT(str) != __CPROVER_POINTER_OBJECT(ap),
+      __CPROVER_POINTER_OBJECT(str) != __CPROVER_POINTER_OBJECT(*(void **)&ap),
       "vsnprintf object overlap");
   }

@@ -1822,12 +1822,12 @@ int __builtin___vsnprintf_chk(
   (void)*fmt;

   while((__CPROVER_size_t)__CPROVER_POINTER_OFFSET(*(void **)&ap) <
-        __CPROVER_OBJECT_SIZE(ap))
+        __CPROVER_OBJECT_SIZE(*(void **)&ap))

   {
     (void)va_arg(ap, int);
     __CPROVER_precondition(
-      __CPROVER_POINTER_OBJECT(str) != __CPROVER_POINTER_OBJECT(ap),
+      __CPROVER_POINTER_OBJECT(str) != __CPROVER_POINTER_OBJECT(*(void **)&ap),
       "vsnprintf object overlap");
   }

EOF
patch -p1 < varargs.patch

cmake3 -S . -Bbuild -DWITH_JBMC=OFF -Dsat_impl="minisat2;cadical" \
  -DCMAKE_C_COMPILER=gcc10-cc -DCMAKE_CXX_COMPILER=gcc10-c++ \
  -DCMAKE_CXX_STANDARD_LIBRARIES=-lstdc++fs \
  -DCMAKE_CXX_FLAGS=-Wno-error=register
cmake3 --build build -- -j$(nproc)
sudo make -C build install

popd
rm -rf "${WORK_DIR}"
