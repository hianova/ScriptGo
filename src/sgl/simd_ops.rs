#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
use core::ptr::{read_unaligned, write_unaligned};

/// # Safety
/// The caller must ensure that `src1_ptr`, `src2_ptr`, and `dest_ptr` point to valid memory buffers of at least `len * 4` bytes.
#[inline(always)]
pub unsafe fn simd_vec_add(len: usize, src1_ptr: *const u8, src2_ptr: *const u8, dest_ptr: *mut u8) {
    let mut i = 0;
    #[cfg(target_arch = "aarch64")]
    unsafe {
        while i + covopt_param!("M_13_18", 4) <= len {
            let v1 = core::arch::aarch64::vld1q_f32(src1_ptr.add(i * covopt_param!("M_14_69", 4)).cast::<f32>());
            let v2 = core::arch::aarch64::vld1q_f32(src2_ptr.add(i * covopt_param!("M_15_69", 4)).cast::<f32>());
            let res = core::arch::aarch64::vaddq_f32(v1, v2);
            core::arch::aarch64::vst1q_f32(dest_ptr.add(i * covopt_param!("M_17_60", 4)).cast::<f32>(), res);
            i += covopt_param!("M_18_17", 4);
        }
    }
    #[cfg(target_arch = "x86_64")]
    #[cfg(target_feature = "avx2")]
    unsafe {
        while i + covopt_param!("M_24_18", 8) <= len {
            let v1 = core::arch::x86_64::_mm256_loadu_ps(src1_ptr.add(i * covopt_param!("M_25_74", 4)).cast::<f32>());
            let v2 = core::arch::x86_64::_mm256_loadu_ps(src2_ptr.add(i * covopt_param!("M_26_74", 4)).cast::<f32>());
            let res = core::arch::x86_64::_mm256_add_ps(v1, v2);
            core::arch::x86_64::_mm256_storeu_ps(dest_ptr.add(i * covopt_param!("M_28_66", 4)).cast::<f32>(), res);
            i += covopt_param!("M_29_17", 8);
        }
    }
    while i < len {
        unsafe {
            let val1 = f32::from_le_bytes(read_unaligned(src1_ptr.add(i * covopt_param!("M_34_74", 4)).cast::<[u8; 4]>()));
            let val2 = f32::from_le_bytes(read_unaligned(src2_ptr.add(i * covopt_param!("M_35_74", 4)).cast::<[u8; 4]>()));
            write_unaligned(dest_ptr.add(i * covopt_param!("M_36_45", 4)).cast::<[u8; 4]>(), (val1 + val2).to_le_bytes());
        }
        i += 1;
    }
}

/// # Safety
/// The caller must ensure that `src1_ptr`, `src2_ptr`, and `dest_ptr` point to valid memory buffers of at least `len * 4` bytes.
#[inline(always)]
pub unsafe fn simd_vec_mul(len: usize, src1_ptr: *const u8, src2_ptr: *const u8, dest_ptr: *mut u8) {
    let mut i = 0;
    #[cfg(target_arch = "aarch64")]
    unsafe {
        while i + covopt_param!("M_49_18", 4) <= len {
            let v1 = core::arch::aarch64::vld1q_f32(src1_ptr.add(i * covopt_param!("M_50_69", 4)).cast::<f32>());
            let v2 = core::arch::aarch64::vld1q_f32(src2_ptr.add(i * covopt_param!("M_51_69", 4)).cast::<f32>());
            let res = core::arch::aarch64::vmulq_f32(v1, v2);
            core::arch::aarch64::vst1q_f32(dest_ptr.add(i * covopt_param!("M_53_60", 4)).cast::<f32>(), res);
            i += covopt_param!("M_54_17", 4);
        }
    }
    #[cfg(target_arch = "x86_64")]
    #[cfg(target_feature = "avx2")]
    unsafe {
        while i + covopt_param!("M_60_18", 8) <= len {
            let v1 = core::arch::x86_64::_mm256_loadu_ps(src1_ptr.add(i * covopt_param!("M_61_74", 4)).cast::<f32>());
            let v2 = core::arch::x86_64::_mm256_loadu_ps(src2_ptr.add(i * covopt_param!("M_62_74", 4)).cast::<f32>());
            let res = core::arch::x86_64::_mm256_mul_ps(v1, v2);
            core::arch::x86_64::_mm256_storeu_ps(dest_ptr.add(i * covopt_param!("M_64_66", 4)).cast::<f32>(), res);
            i += covopt_param!("M_65_17", 8);
        }
    }
    while i < len {
        unsafe {
            let val1 = f32::from_le_bytes(read_unaligned(src1_ptr.add(i * covopt_param!("M_70_74", 4)).cast::<[u8; 4]>()));
            let val2 = f32::from_le_bytes(read_unaligned(src2_ptr.add(i * covopt_param!("M_71_74", 4)).cast::<[u8; 4]>()));
            write_unaligned(dest_ptr.add(i * covopt_param!("M_72_45", 4)).cast::<[u8; 4]>(), (val1 * val2).to_le_bytes());
        }
        i += 1;
    }
}

/// # Safety
/// The caller must ensure that `src1_ptr` and `src2_ptr` point to valid memory buffers of at least `len * 4` bytes.
#[inline(always)]
pub unsafe fn simd_vec_dot(len: usize, src1_ptr: *const u8, src2_ptr: *const u8) -> f32 {
    let mut sum = 0.0f32;
    let mut i = 0;

    #[cfg(target_arch = "aarch64")]
    unsafe {
        let mut acc = core::arch::aarch64::vdupq_n_f32(0.0);
        while i + covopt_param!("M_88_18", 4) <= len {
            let v1 = core::arch::aarch64::vld1q_f32(src1_ptr.add(i * covopt_param!("M_89_69", 4)).cast::<f32>());
            let v2 = core::arch::aarch64::vld1q_f32(src2_ptr.add(i * covopt_param!("M_90_69", 4)).cast::<f32>());
            acc = core::arch::aarch64::vmlaq_f32(acc, v1, v2);
            i += covopt_param!("M_92_17", 4);
        }
        let mut temp = [0.0f32; 4];
        core::arch::aarch64::vst1q_f32(temp.as_mut_ptr(), acc);
        sum += temp.iter().sum::<f32>();
    }

    #[cfg(target_arch = "x86_64")]
    #[cfg(target_feature = "avx2")]
    unsafe {
        let mut acc = core::arch::x86_64::_mm256_setzero_ps();
        while i + covopt_param!("M_103_18", 8) <= len {
            let v1 = core::arch::x86_64::_mm256_loadu_ps(src1_ptr.add(i * covopt_param!("M_104_74", 4)).cast::<f32>());
            let v2 = core::arch::x86_64::_mm256_loadu_ps(src2_ptr.add(i * covopt_param!("M_105_74", 4)).cast::<f32>());
            let res = core::arch::x86_64::_mm256_mul_ps(v1, v2);
            acc = core::arch::x86_64::_mm256_add_ps(acc, res);
            i += covopt_param!("M_108_17", 8);
        }
        let mut temp = [0.0f32; 8];
        core::arch::x86_64::_mm256_storeu_ps(temp.as_mut_ptr(), acc);
        sum += temp.iter().sum::<f32>();
    }

    while i < len {
        unsafe {
            let val1 = f32::from_le_bytes(read_unaligned(src1_ptr.add(i * covopt_param!("M_117_74", 4)).cast::<[u8; 4]>()));
            let val2 = f32::from_le_bytes(read_unaligned(src2_ptr.add(i * covopt_param!("M_118_74", 4)).cast::<[u8; 4]>()));
            sum += val1 * val2;
        }
        i += 1;
    }

    sum
}
