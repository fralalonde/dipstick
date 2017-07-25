/// PCG32 random number generation for fast sampling
// TODO use https://github.com/codahale/pcg instead?
use std::cell::RefCell;
use time;

fn seed() -> u64 {
    let seed = 5573589319906701683_u64;
    let seed = seed.wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
        .wrapping_add(time::precise_time_ns());
    seed.wrapping_mul(6364136223846793005).wrapping_add(
        1442695040888963407,
    )
}

/// quickly return a random int
fn pcg32_random() -> u32 {
    thread_local! {
        static PCG32_STATE: RefCell<u64> = RefCell::new(seed());
    }

    PCG32_STATE.with(|state| {
        let oldstate: u64 = *state.borrow();
        // XXX could generate the increment from the thread ID
        *state.borrow_mut() = oldstate.wrapping_mul(6364136223846793005).wrapping_add(
            1442695040888963407,
        );
        ((((oldstate >> 18) ^ oldstate) >> 27) as u32).rotate_right((oldstate >> 59) as u32)
    })
}

/// Convert a floating point sampling rate to an integer so that a fast integer RNG can be used
/// Float rate range is between 1.0 (send 100% of the samples) and 0.0 (_no_ samples taken)
/// .    | float rate | int rate | percentage
/// ---- | ---------- | -------- | ----
/// all  | 1.0        | 0x0      | 100%
/// none | 0.0        | 0xFFFFFFFF | 0%
pub fn to_int_rate(float_rate: f64) -> u32 {
    assert!(float_rate <= 1.0 && float_rate >= 0.0);
    ((1.0 - float_rate) * ::std::u32::MAX as f64) as u32
}

/// randomly select samples based on an int rate
pub fn accept_sample(int_rate: u32) -> bool {
    pcg32_random() > int_rate
}
