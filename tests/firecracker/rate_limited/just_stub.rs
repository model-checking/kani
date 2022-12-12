use std::time::Instant;

#[kani::proof]
pub fn test() {
    let t1 = instant_now_stub();
    let t2 = instant_now_stub();
    assert!(gt(&t2,&t1));
    //assert!(gt(&t1,&t2));
}

// Instant is 16 bytes on linux, 8 on mac
struct InstantStub {
    nanos : u64,
    padding: [u8;std::mem::size_of::<Instant>() - std::mem::size_of::<u64>()],
}
pub fn instant_now_stub() -> Instant {
    unsafe {
        static mut LAST : u64 = 0;
        let next = kani::any();
        // Constraint 1: instants are monotonically increasing, cause time works that way.
        kani::assume(next > LAST);
        LAST = next;
        unsafe {std::mem::transmute(InstantStub {nanos: next as u64, padding: kani::any()})}
    }
}

pub fn gt(left : &Instant, right: &Instant) -> bool {
    unsafe {
        let left : InstantStub = std::mem::transmute(*left);
        let right : InstantStub = std::mem::transmute(*right);
        let ln = left.nanos;
        let rn = right.nanos;
        left.nanos > right.nanos
    }
}
