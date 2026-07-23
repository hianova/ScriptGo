use core::ptr::{read_unaligned, write_unaligned};

#[inline(always)]
pub unsafe fn simd_vec_add(len: usize, src1_ptr: *const u8, src2_ptr: *const u8, dest_ptr: *mut u8) {
    #[cfg(target_arch = "x86_64")]
    {
        #[cfg(target_feature = "avx2")]
        {
            let mut i = 0;
            while i + 8 <= len {
                let v1 = core::arch::x86_64::_mm256_loadu_ps(src1_ptr.add(i * 4) as *const f32);
                let v2 = core::arch::x86_64::_mm256_loadu_ps(src2_ptr.add(i * 4) as *const f32);
                let res = core::arch::x86_64::_mm256_add_ps(v1, v2);
                core::arch::x86_64::_mm256_storeu_ps(dest_ptr.add(i * 4) as *mut f32, res);
                i += 8;
            }
            while i < len {
                let val1 = f32::from_le_bytes(read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
                let val2 = f32::from_le_bytes(read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
                write_unaligned(dest_ptr.add(i * 4) as *mut [u8; 4], (val1 + val2).to_le_bytes());
                i += 1;
            }
            return;
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        #[cfg(target_feature = "neon")]
        {
            let mut i = 0;
            while i + 4 <= len {
                let v1 = core::arch::aarch64::vld1q_f32(src1_ptr.add(i * 4) as *const f32);
                let v2 = core::arch::aarch64::vld1q_f32(src2_ptr.add(i * 4) as *const f32);
                let res = core::arch::aarch64::vaddq_f32(v1, v2);
                core::arch::aarch64::vst1q_f32(dest_ptr.add(i * 4) as *mut f32, res);
                i += 4;
            }
            while i < len {
                let val1 = f32::from_le_bytes(read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
                let val2 = f32::from_le_bytes(read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
                write_unaligned(dest_ptr.add(i * 4) as *mut [u8; 4], (val1 + val2).to_le_bytes());
                i += 1;
            }
            return;
        }
    }
    // Fallback
    for i in 0..len {
        let val1 = f32::from_le_bytes(read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
        let val2 = f32::from_le_bytes(read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
        write_unaligned(dest_ptr.add(i * 4) as *mut [u8; 4], (val1 + val2).to_le_bytes());
    }
}

#[inline(always)]
pub unsafe fn simd_vec_mul(len: usize, src1_ptr: *const u8, src2_ptr: *const u8, dest_ptr: *mut u8) {
    #[cfg(target_arch = "x86_64")]
    {
        #[cfg(target_feature = "avx2")]
        {
            let mut i = 0;
            while i + 8 <= len {
                let v1 = core::arch::x86_64::_mm256_loadu_ps(src1_ptr.add(i * 4) as *const f32);
                let v2 = core::arch::x86_64::_mm256_loadu_ps(src2_ptr.add(i * 4) as *const f32);
                let res = core::arch::x86_64::_mm256_mul_ps(v1, v2);
                core::arch::x86_64::_mm256_storeu_ps(dest_ptr.add(i * 4) as *mut f32, res);
                i += 8;
            }
            while i < len {
                let val1 = f32::from_le_bytes(read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
                let val2 = f32::from_le_bytes(read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
                write_unaligned(dest_ptr.add(i * 4) as *mut [u8; 4], (val1 * val2).to_le_bytes());
                i += 1;
            }
            return;
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        #[cfg(target_feature = "neon")]
        {
            let mut i = 0;
            while i + 4 <= len {
                let v1 = core::arch::aarch64::vld1q_f32(src1_ptr.add(i * 4) as *const f32);
                let v2 = core::arch::aarch64::vld1q_f32(src2_ptr.add(i * 4) as *const f32);
                let res = core::arch::aarch64::vmulq_f32(v1, v2);
                core::arch::aarch64::vst1q_f32(dest_ptr.add(i * 4) as *mut f32, res);
                i += 4;
            }
            while i < len {
                let val1 = f32::from_le_bytes(read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
                let val2 = f32::from_le_bytes(read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
                write_unaligned(dest_ptr.add(i * 4) as *mut [u8; 4], (val1 * val2).to_le_bytes());
                i += 1;
            }
            return;
        }
    }
    // Fallback
    for i in 0..len {
        let val1 = f32::from_le_bytes(read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
        let val2 = f32::from_le_bytes(read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
        write_unaligned(dest_ptr.add(i * 4) as *mut [u8; 4], (val1 * val2).to_le_bytes());
    }
}

#[inline(always)]
pub unsafe fn simd_vec_dot(len: usize, src1_ptr: *const u8, src2_ptr: *const u8) -> f32 {
    let mut sum = 0.0f32;
    
    #[cfg(target_arch = "x86_64")]
    {
        #[cfg(target_feature = "avx2")]
        {
            let mut i = 0;
            let mut acc = core::arch::x86_64::_mm256_setzero_ps();
            while i + 8 <= len {
                let v1 = core::arch::x86_64::_mm256_loadu_ps(src1_ptr.add(i * 4) as *const f32);
                let v2 = core::arch::x86_64::_mm256_loadu_ps(src2_ptr.add(i * 4) as *const f32);
                let res = core::arch::x86_64::_mm256_mul_ps(v1, v2);
                acc = core::arch::x86_64::_mm256_add_ps(acc, res);
                i += 8;
            }
            let mut temp = [0.0f32; 8];
            core::arch::x86_64::_mm256_storeu_ps(temp.as_mut_ptr(), acc);
            sum += temp.iter().sum::<f32>();
            while i < len {
                let val1 = f32::from_le_bytes(read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
                let val2 = f32::from_le_bytes(read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
                sum += val1 * val2;
                i += 1;
            }
            return sum;
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        #[cfg(target_feature = "neon")]
        {
            let mut i = 0;
            let mut acc = core::arch::aarch64::vdupq_n_f32(0.0);
            while i + 4 <= len {
                let v1 = core::arch::aarch64::vld1q_f32(src1_ptr.add(i * 4) as *const f32);
                let v2 = core::arch::aarch64::vld1q_f32(src2_ptr.add(i * 4) as *const f32);
                acc = core::arch::aarch64::vmlaq_f32(acc, v1, v2);
                i += 4;
            }
            let mut temp = [0.0f32; 4];
            core::arch::aarch64::vst1q_f32(temp.as_mut_ptr(), acc);
            sum += temp.iter().sum::<f32>();
            while i < len {
                let val1 = f32::from_le_bytes(read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
                let val2 = f32::from_le_bytes(read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
                sum += val1 * val2;
                i += 1;
            }
            return sum;
        }
    }
    // Fallback
    for i in 0..len {
        let val1 = f32::from_le_bytes(read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
        let val2 = f32::from_le_bytes(read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
        sum += val1 * val2;
    }
    sum
}
