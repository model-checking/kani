-- THIS FILE WAS AUTOMATICALLY GENERATED BY AENEAS
-- [test]
import Base
open Primitives
set_option linter.dupNamespace false
set_option linter.hashCommand false
set_option linter.unusedVariables false

namespace test

/- [test::select]:
   Source: 'test.rs', lines 8:1-8:42 -/
def select (s : Bool) (x : I32) (y : I32) : Result I32 :=
  if s
  then Result.ok x
  else Result.ok y

/- [test::main]:
   Source: 'test.rs', lines 13:1-13:10 -/
def main : Result Unit :=
  do
  let _ ← select true 3#i32 7#i32
  Result.ok ()

end test