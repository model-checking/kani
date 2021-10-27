// compile-flags: -C no-prepopulate-passes
//

// ignore-arm
// ignore-aarch64
// ignore-mips
// ignore-mips64
// ignore-powerpc
// ignore-powerpc64
// ignore-riscv64 see codegen/riscv-abi
// ignore-windows
// See repr-transparent.rs

#![feature(transparent_unions)]

#![crate_type="lib"]


#[derive(Clone, Copy)]
#[repr(C)]
pub struct BigS([u32; 16]);

#[repr(transparent)]
pub struct TsBigS(BigS);

#[repr(transparent)]
pub union TuBigS {
    field: BigS,
}

#[repr(transparent)]
pub enum TeBigS {
    Variant(BigS),
}

// CHECK: define{{.*}}void @test_BigS(%BigS* [[BIGS_RET_ATTRS1:.*]] sret(%BigS) [[BIGS_RET_ATTRS2:.*]], %BigS* [[BIGS_ARG_ATTRS1:.*]] byval(%BigS) [[BIGS_ARG_ATTRS2:.*]])
#[no_mangle]
pub extern "C" fn test_BigS(_: BigS) -> BigS { loop {} }

// CHECK: define{{.*}}void @test_TsBigS(%TsBigS* [[BIGS_RET_ATTRS1]] sret(%TsBigS) [[BIGS_RET_ATTRS2]], %TsBigS* [[BIGS_ARG_ATTRS1]] byval(%TsBigS) [[BIGS_ARG_ATTRS2:.*]])
#[no_mangle]
pub extern "C" fn test_TsBigS(_: TsBigS) -> TsBigS { loop {} }

// CHECK: define{{.*}}void @test_TuBigS(%TuBigS* [[BIGS_RET_ATTRS1]] sret(%TuBigS) [[BIGS_RET_ATTRS2]], %TuBigS* [[BIGS_ARG_ATTRS1]] byval(%TuBigS) [[BIGS_ARG_ATTRS2:.*]])
#[no_mangle]
pub extern "C" fn test_TuBigS(_: TuBigS) -> TuBigS { loop {} }

// CHECK: define{{.*}}void @test_TeBigS(%"TeBigS::Variant"* [[BIGS_RET_ATTRS1]] sret(%"TeBigS::Variant") [[BIGS_RET_ATTRS2]], %"TeBigS::Variant"* [[BIGS_ARG_ATTRS1]] byval(%"TeBigS::Variant") [[BIGS_ARG_ATTRS2]])
#[no_mangle]
pub extern "C" fn test_TeBigS(_: TeBigS) -> TeBigS { loop {} }


#[derive(Clone, Copy)]
#[repr(C)]
pub union BigU {
    foo: [u32; 16],
}

#[repr(transparent)]
pub struct TsBigU(BigU);

#[repr(transparent)]
pub union TuBigU {
    field: BigU,
}

#[repr(transparent)]
pub enum TeBigU {
    Variant(BigU),
}

// CHECK: define{{.*}}void @test_BigU(%BigU* [[BIGU_RET_ATTRS1:.*]] sret(%BigU) [[BIGU_RET_ATTRS2:.*]], %BigU* [[BIGU_ARG_ATTRS1:.*]] byval(%BigU) [[BIGU_ARG_ATTRS2:.*]])
#[no_mangle]
pub extern "C" fn test_BigU(_: BigU) -> BigU { loop {} }

// CHECK: define{{.*}}void @test_TsBigU(%TsBigU* [[BIGU_RET_ATTRS1:.*]] sret(%TsBigU) [[BIGU_RET_ATTRS2:.*]], %TsBigU* [[BIGU_ARG_ATTRS1]] byval(%TsBigU) [[BIGU_ARG_ATTRS2]])
#[no_mangle]
pub extern "C" fn test_TsBigU(_: TsBigU) -> TsBigU { loop {} }

// CHECK: define{{.*}}void @test_TuBigU(%TuBigU* [[BIGU_RET_ATTRS1]] sret(%TuBigU) [[BIGU_RET_ATTRS2:.*]], %TuBigU* [[BIGU_ARG_ATTRS1]] byval(%TuBigU) [[BIGU_ARG_ATTRS2]])
#[no_mangle]
pub extern "C" fn test_TuBigU(_: TuBigU) -> TuBigU { loop {} }

// CHECK: define{{.*}}void @test_TeBigU(%"TeBigU::Variant"* [[BIGU_RET_ATTRS1]] sret(%"TeBigU::Variant") [[BIGU_RET_ATTRS2:.*]], %"TeBigU::Variant"* [[BIGU_ARG_ATTRS1]] byval(%"TeBigU::Variant") [[BIGU_ARG_ATTRS2]])
#[no_mangle]
pub extern "C" fn test_TeBigU(_: TeBigU) -> TeBigU { loop {} }
