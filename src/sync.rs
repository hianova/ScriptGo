use core::sync::atomic::{AtomicUsize, Ordering, fence};
use core::hint::spin_loop;
use core::cell::UnsafeCell;
use no_std_tool::sync::{SpinMutex, Backoff};

/// A simple Sequence Lock for Wait-Free Readers.
/// Requires the protected data to be `Copy`.
pub struct SeqLock<T: Copy> {
    seq: AtomicUsize,
    writer_lock: SpinMutex<()>,
    data: UnsafeCell<T>,
}

unsafe impl<T: Copy + Send> Send for SeqLock<T> {}
unsafe impl<T: Copy + Send> Sync for SeqLock<T> {}

impl<T: Copy> SeqLock<T> {
    /// Creates a new SeqLock.
    pub const fn new(val: T) -> Self {
        Self {
            seq: AtomicUsize::new(0),
            writer_lock: SpinMutex::new(()),
            data: UnsafeCell::new(val),
        }
    }

    /// Read the data wait-free using sequence validation.
    #[inline(always)]
    pub fn read(&self) -> T {
        loop {
            // Read the sequence number before reading data
            let seq1 = self.seq.load(Ordering::Acquire);
            
            // If it's odd, a writer is currently writing. Spin and retry.
            if seq1 & 1 != 0 {
                spin_loop();
                continue;
            }

            fence(Ordering::Acquire);

            // SAFETY: Data race is possible here, but since T is Copy, 
            // and we validate the sequence number afterwards, any torn or 
            // invalid read will be discarded if the sequence number changed.
            // Using read_volatile to prevent the compiler from reordering or optimizing out the read.
            let data = unsafe { core::ptr::read_volatile(self.data.get()) };

            // Ensure the reads are not reordered after the second sequence check
            fence(Ordering::Acquire);

            let seq2 = self.seq.load(Ordering::Acquire);
            
            // If sequence hasn't changed, our read was valid.
            if seq1 == seq2 {
                return data;
            }
        }
    }

    /// Exclusively write the data.
    pub fn write(&self, new_data: T) {
        // Serialize multiple writers using no_std_tool's SpinMutex and Backoff retry loop
        let mut backoff = Backoff::new();
        let _guard = loop {
            match self.writer_lock.lock() {
                Ok(guard) => break guard,
                Err(_) => {
                    backoff.snooze();
                }
            }
        };

        let seq = self.seq.load(Ordering::Relaxed);
        
        // Increment sequence to odd (writing state)
        self.seq.store(seq.wrapping_add(1), Ordering::Release);

        // Ensure the sequence is visible before data write
        fence(Ordering::Release);

        // SAFETY: We hold the writer lock, so we have exclusive write access.
        unsafe {
            core::ptr::write_volatile(self.data.get(), new_data);
        }

        // Ensure data write is visible before sequence update
        fence(Ordering::Release);

        // Increment sequence to even (stable state)
        self.seq.store(seq.wrapping_add(2), Ordering::Release);
    }
}
