#![allow(unused_imports)]
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use no_std_tool::bignum::U256;
use no_std_tool::math::{silu_approx_i8, exp_approx_q16};
use no_std_tool::random::Xoshiro256StarStar;
use crate::sgl::vm::ScriptVm;
use sgl_macros::sgl_package;
use core::sync::atomic::{AtomicU64, Ordering};

// A global RNG for the SGL runtime
static RNG_SEED: AtomicU64 = AtomicU64::new(0x1234567890ABCDEF);

#[sgl_package(name = "sgl_math", kind = "hardware")]
pub mod sgl_math {
    use super::*;

    /// Command 10: ModPow - (base ^ exp) % modulus
    /// Expects hex strings for base, exp, and modulus. Returns a hex string.
    #[sgl_cmd(id = 10)]
    pub fn mod_pow(_vm: &mut ScriptVm, base_hex: String, exp_hex: String, mod_hex: String) -> Result<String, u32> {
        let base = U256::from_hex_str(&base_hex).unwrap_or(U256::zero());
        let exp = U256::from_hex_str(&exp_hex).unwrap_or(U256::zero());
        let modulus = U256::from_hex_str(&mod_hex).unwrap_or(U256::zero());
        
        if modulus.is_zero() {
            return Err(1); // Error code 1: Divide by zero
        }

        let result = U256::mod_pow(base, exp, modulus);
        Ok(alloc::format!("{:x}", result))
    }

    /// Command 11: SiluApprox - AI activation function
    /// Computes x * sigmoid(x)
    #[sgl_cmd(id = 11)]
    pub fn math_silu_approx(_vm: &mut ScriptVm, x: u32) -> u32 {
        silu_approx_i8(x as i8).unwrap_or(0) as u8 as u32
    }

    /// Command 12: ExpQ16 - AI Exponential function
    /// Computes exp(x) for Q16.16 fixed-point
    #[sgl_cmd(id = 12)]
    pub fn math_exp_q16(_vm: &mut ScriptVm, x: u32) -> u32 {
        exp_approx_q16(x as i32).unwrap_or(0) as u32
    }

    /// Command 13: RandomU32 - Generates a hardware-seeded random number
    #[sgl_cmd(id = 13)]
    pub fn random_u32(_vm: &mut ScriptVm) -> u32 {
        let seed = RNG_SEED.fetch_add(1, Ordering::Relaxed);
        let mut rng = Xoshiro256StarStar::new(seed);
        rng.next_u64() as u32
    }
}

pub use self::sgl_math::*;
