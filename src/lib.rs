#![no_std]

extern crate alloc;

pub mod instruction;
pub mod vm;
pub mod assembler;
pub mod sync;

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod covopt_tests {
    use std::env;
    use super::*;

    #[test]
    fn covopt_benchmark_test() {
        let n_str = env::var("COVOPT_N").unwrap_or_else(|_| alloc::string::String::from("100"));
        let n: usize = n_str.parse().unwrap();
        
        // Simulating O(N) behavior based on n
        let mut _sum = 0;
        for i in 0..n { // TARGET LINE
            _sum += i;
        }
    }
}
