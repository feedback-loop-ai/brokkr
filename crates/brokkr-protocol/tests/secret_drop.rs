//! A dropped `Secret` hands its buffer back to the allocator holding
//! zeroes, not its value (decision 0012; #419).
//!
//! The seam is this binary's allocator. It passes every call to the
//! system allocator, and when the one buffer a test watches is freed it
//! copies that buffer's first bytes aside before freeing it. It reads the
//! buffer while the buffer is still allocated, never after: freed memory
//! is never read. The watch is claimed by compare-and-swap, so a later
//! allocation at the same address cannot overwrite what was seen.

use brokkr_protocol::secret::Secret;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering::SeqCst};

/// How many leading bytes of the watched buffer are kept.
const SEEN: usize = 16;

static WATCHED: AtomicUsize = AtomicUsize::new(0);
static FREED: [AtomicU8; SEEN] = [const { AtomicU8::new(0) }; SEEN];
static FREED_LEN: AtomicUsize = AtomicUsize::new(0);

struct Witness;

// SAFETY: every allocation is the system allocator's, unchanged.
unsafe impl GlobalAlloc for Witness {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract for `alloc`, passed through.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if WATCHED
            .compare_exchange(ptr as usize, 0, SeqCst, SeqCst)
            .is_ok()
        {
            let len = layout.size().min(SEEN);
            for (offset, slot) in FREED.iter().take(len).enumerate() {
                // SAFETY: `ptr` is allocated for `layout.size()` bytes
                // until the `System.dealloc` below.
                slot.store(unsafe { ptr.add(offset).read_volatile() }, SeqCst);
            }
            FREED_LEN.store(len, SeqCst);
        }
        // SAFETY: the caller's contract for `dealloc`, passed through.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Witness = Witness;

/// What `value`'s buffer held when `release` handed it back.
fn freed_by(value: &[u8], release: impl FnOnce(Vec<u8>)) -> Vec<u8> {
    let bytes = value.to_vec();
    WATCHED.store(bytes.as_ptr() as usize, SeqCst);
    release(bytes);
    assert_eq!(WATCHED.load(SeqCst), 0, "the watched buffer was not freed");
    FREED[..FREED_LEN.load(SeqCst)]
        .iter()
        .map(|byte| byte.load(SeqCst))
        .collect()
}

/// The control first: a plain vector frees its value as it held it, so
/// the witness sees the buffer. Then the `Secret`, which must not.
#[test]
fn a_dropped_secret_frees_zeroes_not_its_value() {
    assert_eq!(freed_by(b"hunter22", drop), b"hunter22");
    assert_eq!(
        freed_by(b"hunter22", |bytes| drop(Secret::new(bytes))),
        [0; 8]
    );
}
