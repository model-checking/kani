Reading GOTO program from 'test/cargo-rmc/simple-config-toml/target/cbmc.out'
#include <assert.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>

// tag-Unit
// 
struct Unit;

// tag-pair::Pair
// 
struct pair::Pair;

// tag-str
// 
struct str;


// _RNvMNtCs9wJDO6DC35D_18simple_config_toml4pairNtB2_4Pair3new
// file /Users/vecchiot/Documents/rmc/src/test/cargo-rmc/simple-config-toml/src/pair.rs line 6 column 5 function _RNvMNtCs9wJDO6DC35D_18simple_config_toml4pairNtB2_4Pair3new
struct pair::Pair _RNvMNtCs9wJDO6DC35D_18simple_config_toml4pairNtB2_4Pair3new(unsigned long int a, unsigned long int b);
// _RNvMNtCs9wJDO6DC35D_18simple_config_toml4pairNtB2_4Pair3sum
// file /Users/vecchiot/Documents/rmc/src/test/cargo-rmc/simple-config-toml/src/pair.rs line 9 column 5 function _RNvMNtCs9wJDO6DC35D_18simple_config_toml4pairNtB2_4Pair3sum
unsigned long int _RNvMNtCs9wJDO6DC35D_18simple_config_toml4pairNtB2_4Pair3sum(struct pair::Pair *self);
// _ZN4core3num21_$LT$impl$u20$u64$GT$12wrapping_add17h46bd47904fb592e2E
// file /Users/vecchiot/Documents/rmc/library/core/src/num/uint_macros.rs line 1082 column 9 function _ZN4core3num21_$LT$impl$u20$u64$GT$12wrapping_add17h46bd47904fb592e2E
unsigned long int _ZN4core3num21_$LT$impl$u20$u64$GT$12wrapping_add17h46bd47904fb592e2E(unsigned long int self, unsigned long int rhs);
// ceilf
// file <builtin-library-ceilf> function ceilf
float ceilf(float);
// copysignf
// file <builtin-library-copysignf> function copysignf
float copysignf(float, float);
// cosf
// file <builtin-library-cosf> function cosf
float cosf(float);
// exp2f
// file <builtin-library-exp2f> function exp2f
float exp2f(float);
// expf
// file <builtin-library-expf> function expf
float expf(float);
// fabsf
// file <builtin-library-fabsf> function fabsf
float fabsf(float);
// floorf
// file <builtin-library-floorf> function floorf
float floorf(float);
// fmaf
// file <builtin-library-fmaf> function fmaf
float fmaf(float, float, float);
// fmaxf
// file <builtin-library-fmaxf> function fmaxf
float fmaxf(float, float);
// fminf
// file <builtin-library-fminf> function fminf
float fminf(float, float);
// log10f
// file <builtin-library-log10f> function log10f
float log10f(float);
// log2f
// file <builtin-library-log2f> function log2f
float log2f(float);
// logf
// file <builtin-library-logf> function logf
float logf(float);
// memcmp
// file <builtin-library-memcmp> function memcmp
int memcmp(void *, void *, unsigned long int);
// memcpy
// file <builtin-library-memcpy> function memcpy
void * memcpy(void *, void *, unsigned long int);
// memmove
// file <builtin-library-memmove> function memmove
void * memmove(void *, void *, unsigned long int);
// nearbyintf
// file <builtin-library-nearbyintf> function nearbyintf
float nearbyintf(float);
// powf
// file <builtin-library-powf> function powf
float powf(float, float);
// powi
// file <builtin-library-powi> function powi
double powi(double, int);
// powif
// file <builtin-library-powif> function powif
float powif(float, int);
// rintf
// file <builtin-library-rintf> function rintf
float rintf(float);
// roundf
// file <builtin-library-roundf> function roundf
float roundf(float);
// sinf
// file <builtin-library-sinf> function sinf
float sinf(float);
// sqrtf
// file <builtin-library-sqrtf> function sqrtf
float sqrtf(float);
// test_one_plus_two
// file /Users/vecchiot/Documents/rmc/src/test/cargo-rmc/simple-config-toml/src/pair.rs line 29 column 5 function test_one_plus_two
struct Unit test_one_plus_two(void);
// test_sum
// file /Users/vecchiot/Documents/rmc/src/test/cargo-rmc/simple-config-toml/src/lib.rs line 23 column 5 function test_sum
struct Unit test_sum(void);
// truncf
// file <builtin-library-truncf> function truncf
float truncf(float);

struct Unit
{
};

struct pair::Pair
{
  // 0
  unsigned long int 0;
  // 1
  unsigned long int 1;
};

struct str
{
  // data
  signed char *data;
  // len
  unsigned long int len;
};


// VoidUnit
// 
struct Unit VoidUnit;

// _RNvMNtCs9wJDO6DC35D_18simple_config_toml4pairNtB2_4Pair3new
// file /Users/vecchiot/Documents/rmc/src/test/cargo-rmc/simple-config-toml/src/pair.rs line 6 column 5 function _RNvMNtCs9wJDO6DC35D_18simple_config_toml4pairNtB2_4Pair3new
struct pair::Pair _RNvMNtCs9wJDO6DC35D_18simple_config_toml4pairNtB2_4Pair3new(unsigned long int a, unsigned long int b)
{
  struct pair::Pair var_0;
  unsigned long int var_3;
  unsigned long int var_4;

bb0:
  ;
  var_3 = a;
  var_4 = b;
  var_0.0 = var_3;
  var_0.1 = var_4;
  return var_0;
}

// _RNvMNtCs9wJDO6DC35D_18simple_config_toml4pairNtB2_4Pair3sum
// file /Users/vecchiot/Documents/rmc/src/test/cargo-rmc/simple-config-toml/src/pair.rs line 9 column 5 function _RNvMNtCs9wJDO6DC35D_18simple_config_toml4pairNtB2_4Pair3sum
unsigned long int _RNvMNtCs9wJDO6DC35D_18simple_config_toml4pairNtB2_4Pair3sum(struct pair::Pair *self)
{
  unsigned long int var_0;
  unsigned long int var_2;
  unsigned long int var_3;

bb0:
  ;
  var_2 = self->0;
  var_3 = self->1;
  var_0=_ZN4core3num21_$LT$impl$u20$u64$GT$12wrapping_add17h46bd47904fb592e2E(var_2, var_3);

bb1:
  ;
  return var_0;
}

// _ZN4core3num21_$LT$impl$u20$u64$GT$12wrapping_add17h46bd47904fb592e2E
// file /Users/vecchiot/Documents/rmc/library/core/src/num/uint_macros.rs line 1082 column 9 function _ZN4core3num21_$LT$impl$u20$u64$GT$12wrapping_add17h46bd47904fb592e2E
unsigned long int _ZN4core3num21_$LT$impl$u20$u64$GT$12wrapping_add17h46bd47904fb592e2E(unsigned long int self, unsigned long int rhs)
{
  unsigned long int var_0;
  unsigned long int var_3;
  unsigned long int var_4;

bb0:
  ;
  var_3 = self;
  var_4 = rhs;
  var_0 = var_3 + var_4;
  return var_0;
}

// __CPROVER__start
// 
void main(void)
{

__CPROVER_HIDE:
  ;
  __CPROVER_initialize();
  return'=test_sum();
  OUTPUT("return'", return');
}

// test_one_plus_two
// file /Users/vecchiot/Documents/rmc/src/test/cargo-rmc/simple-config-toml/src/pair.rs line 29 column 5 function test_one_plus_two
struct Unit test_one_plus_two(void)
{
  struct Unit var_0;
  struct pair::Pair p;
  _Bool var_2;
  _Bool var_3;
  unsigned long int var_4;
  struct pair::Pair *var_5;

bb0:
  ;
  p=_RNvMNtCs9wJDO6DC35D_18simple_config_toml4pairNtB2_4Pair3new(1, 2);

bb1:
  ;
  var_5 = &p;
  var_4=_RNvMNtCs9wJDO6DC35D_18simple_config_toml4pairNtB2_4Pair3sum(var_5);

bb2:
  ;
  var_3 = var_4 == 3;
  var_2 = !(var_3 != 0);
  if(!(var_2 == 0))
  {
  /* assertion failed: p.sum() == 3 */

  bb3:
    ;
    /* assertion failed: p.sum() == 3 */
    assert(0);
  }


bb4:
  ;
  return VoidUnit;
}

// test_sum
// file /Users/vecchiot/Documents/rmc/src/test/cargo-rmc/simple-config-toml/src/lib.rs line 23 column 5 function test_sum
struct Unit test_sum(void)
{
  struct Unit var_0;
  unsigned long int a;
  unsigned long int b;
  struct pair::Pair p;
  unsigned long int var_4;
  unsigned long int var_5;
  _Bool var_6;
  _Bool var_7;
  unsigned long int var_8;
  struct pair::Pair *var_9;
  unsigned long int var_10;
  unsigned long int var_11;
  unsigned long int var_12;

bb0:
  ;
  a = nondet_0();

bb1:
  ;
  b = nondet_0();

bb2:
  ;
  var_4 = a;
  var_5 = b;
  p=_RNvMNtCs9wJDO6DC35D_18simple_config_toml4pairNtB2_4Pair3new(var_4, var_5);

bb3:
  ;
  var_9 = &p;
  var_8=_RNvMNtCs9wJDO6DC35D_18simple_config_toml4pairNtB2_4Pair3sum(var_9);

bb4:
  ;
  var_11 = a;
  var_12 = b;
  var_10=_ZN4core3num21_$LT$impl$u20$u64$GT$12wrapping_add17h46bd47904fb592e2E(var_11, var_12);

bb5:
  ;
  var_7 = var_8 == var_10;
  var_6 = !(var_7 != 0);
  if(!(var_6 == 0))
  {
  /* assertion failed: p.sum() == a.wrapping_add(b) */

  bb6:
    ;
    /* assertion failed: p.sum() == a.wrapping_add(b) */
    assert(0);
  }


bb7:
  ;
  return VoidUnit;
}

