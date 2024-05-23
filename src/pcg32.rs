//! PCG32 random number generation for fast sampling
//! Kept here for low dependency count.

#![cfg_attr(feature = "tool_lints", allow(clippy::unreadable_literal))]
#![allow(clippy::unreadable_literal)]

use std::{cell::RefCell, time::{SystemTime, UNIX_EPOCH}};

fn seed() -> u64 {
    let seed = 5573589319906701683_u64;
    let seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
        .wrapping_add(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64);
    seed.wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

/// quickly return a random int
fn pcg32_random() -> u32 {
    thread_local! {
        static PCG32_STATE: RefCell<u64> = RefCell::new(seed());
    }

    PCG32_STATE.with(|state| {
        let old_state: u64 = *state.borrow();
        // XXX could generate the increment from the thread ID
        *state.borrow_mut() = old_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((((old_state >> 18) ^ old_state) >> 27) as u32).rotate_right((old_state >> 59) as u32)
    })
}

/// Convert a floating point sampling rate to an integer so that a fast integer RNG can be used
/// Float rate range is between 1.0 (send 100% of the samples) and 0.0 (_no_ samples taken)
/// .    | float rate | int rate | percentage
/// ---- | ---------- | -------- | ----
/// all  | 1.0        | 0x0      | 100%
/// none | 0.0        | 0xFFFFFFFF | 0%
pub fn to_int_rate(float_rate: f64) -> u32 {
    assert!((0.0..=1.0).contains(&float_rate));
    ((1.0 - float_rate) * f64::from(u32::MAX)) as u32
}

/// randomly select samples based on an int rate
pub fn accept_sample(int_rate: u32) -> bool {
    pcg32_random() > int_rate
}
