use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::{null_mut, NonNull};

/// Largest heap handed to talc in one claim: `isize::MAX` rounded down to a page.
///
/// talc walks a free gap with `base.add(size)` / `acme.sub(size)`, which is undefined
/// behaviour once the gap exceeds `isize::MAX`: 2 GiB on riscv32, which an arena on
/// the 4 GiB machine image (`RamSize::FourGb`) can exceed. The arena is therefore
/// claimed as several adjacent heaps. talc never merges across heaps (each heap's
/// bottom is tagged allocated), so every gap stays within its heap; the only thing
/// given up is a single allocation straddling two heaps, and `Layout` already caps
/// one allocation at `isize::MAX`.
const MAX_HEAP_SIZE: usize = (isize::MAX as usize) & PAGE_MASK;

/// Granularity of heap sizes. talc only needs its own word-sized chunk alignment;
/// a page keeps every heap boundary as aligned as the arena start.
const PAGE: usize = 4096;

/// Clears the in-page offset bits: `x & PAGE_MASK` rounds `x` down to a page.
const PAGE_MASK: usize = !(PAGE - 1);

pub struct TalcAllocator {
    state: UnsafeCell<TalcState>,
}

struct TalcState {
    allocator: Option<talc::Talc<talc::ClaimOnOom>>,
}

unsafe impl Sync for TalcAllocator {}

impl TalcAllocator {
    pub const fn uninit() -> Self {
        Self {
            state: UnsafeCell::new(TalcState { allocator: None }),
        }
    }

    /// # Safety
    ///
    /// Caller must ensure `start` and `end` define a writable heap range.
    pub unsafe fn init(&self, start: *mut usize, end: *mut usize) {
        self.init_with_max_heap_size(start, end, MAX_HEAP_SIZE);
    }

    /// # Safety
    ///
    /// Same as [`Self::init`]. Additionally, `max` must be a multiple of `PAGE` and at
    /// most `MAX_HEAP_SIZE`: heap sizes are rounded up to a page, and a heap larger
    /// than `isize::MAX` is undefined behaviour inside talc.
    unsafe fn init_with_max_heap_size(&self, start: *mut usize, end: *mut usize, max: usize) {
        debug_assert!(max.is_multiple_of(PAGE) && max <= MAX_HEAP_SIZE);
        let state = &mut *self.state.get();
        let end = end as usize;
        let mut base = start as usize;
        if end <= base {
            state.allocator = None;
            return;
        }

        // Equal heaps rather than `max`-sized ones plus a remainder: a remainder
        // can be too small for talc to claim. Rounding up to a page keeps each
        // heap within `max`, and the last one within a few pages of the others.
        let total = end - base;
        let heap_size = total.div_ceil(total.div_ceil(max)).next_multiple_of(PAGE);

        let mut allocator = talc::Talc::new(talc::ClaimOnOom::new(talc::Span::empty()));
        while base < end {
            let size = (end - base).min(heap_size);
            let span = talc::Span::from_base_size(base as *mut u8, size);
            allocator.claim(span).expect("must claim heap span");
            base += size;
        }
        state.allocator = Some(allocator);
    }

    unsafe fn alloc_inner(&self, layout: Layout) -> *mut u8 {
        let state = &mut *self.state.get();
        let Some(allocator) = state.allocator.as_mut() else {
            return null_mut();
        };
        allocator
            .malloc(layout)
            .map_or(null_mut(), |nn| nn.as_ptr())
    }

    unsafe fn dealloc_inner(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() || layout.size() == 0 {
            return;
        }

        let state = &mut *self.state.get();
        if let Some(allocator) = state.allocator.as_mut() {
            allocator.free(NonNull::new_unchecked(ptr), layout);
        }
    }
}

unsafe impl GlobalAlloc for TalcAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_inner(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.dealloc_inner(ptr, layout)
    }

    // Deliberately alloc + copy + free rather than talc's `grow_in_place`, which
    // compares addresses and, near the top of a 32-bit address space, grants a growth
    // whose end wraps past 2^32 (SFBdragon/talc#56).
    unsafe fn realloc(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
        // Safety for `from_size_align_unchecked`: `old_layout` _must_ be the same layout that was
        // used to allocate `ptr`.`

        if ptr.is_null() {
            return self.alloc_inner(Layout::from_size_align_unchecked(
                new_size,
                old_layout.align(),
            ));
        }

        let new_layout = Layout::from_size_align_unchecked(new_size, old_layout.align());
        let new_ptr = self.alloc_inner(new_layout);
        if !new_ptr.is_null() {
            let copy_len = core::cmp::min(old_layout.size(), new_size);
            core::ptr::copy_nonoverlapping(ptr, new_ptr, copy_len);
            self.dealloc_inner(ptr, old_layout);
        }
        new_ptr
    }
}

#[cfg(target_arch = "riscv32")]
#[global_allocator]
static GLOBAL_ALLOCATOR: TalcAllocator = TalcAllocator::uninit();

/// # Safety
///
/// Caller must ensure `start` and `end` define a valid, exclusively-owned heap region.
#[cfg(target_arch = "riscv32")]
pub unsafe fn init(start: *mut usize, end: *mut usize) {
    GLOBAL_ALLOCATOR.init(start, end);
}

/// # Safety
///
/// No-op on non-riscv32 targets; kept for API compatibility.
#[cfg(not(target_arch = "riscv32"))]
pub unsafe fn init(_start: *mut usize, _end: *mut usize) {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{vec, vec::Vec};

    const ARENA: usize = 1 << 20;
    const HEAP: usize = 256 << 10;

    /// A 1 MiB host buffer claimed as 256 KiB heaps, so the split is exercised on a
    /// 64-bit host where `MAX_HEAP_SIZE` is out of reach.
    fn split_allocator(arena: &mut [u64]) -> (TalcAllocator, usize, usize) {
        let allocator = TalcAllocator::uninit();
        let start = arena.as_mut_ptr() as usize;
        let end = start + core::mem::size_of_val(arena);
        unsafe { allocator.init_with_max_heap_size(start as *mut usize, end as *mut usize, HEAP) };
        (allocator, start, end)
    }

    #[test]
    fn allocations_across_heaps_stay_in_the_arena_and_do_not_overlap() {
        let mut arena = vec![0u64; ARENA / 8];
        let (allocator, start, end) = split_allocator(&mut arena);
        let layout = Layout::from_size_align(16 << 10, 8).unwrap();

        let mut blocks = Vec::new();
        loop {
            let ptr = unsafe { allocator.alloc(layout) };
            if ptr.is_null() {
                break;
            }
            let addr = ptr as usize;
            assert!(addr >= start && addr + layout.size() <= end);
            blocks.push(addr);
        }
        // Each 256 KiB heap loses a little to tags (and the first to talc's
        // metadata), so a few fewer than 64 fit, spread over all four heaps.
        assert!(blocks.len() >= 56, "only {} blocks fit", blocks.len());
        assert!(blocks.iter().any(|&a| a >= start + 3 * HEAP));

        blocks.sort_unstable();
        assert!(blocks.windows(2).all(|w| w[0] + layout.size() <= w[1]));

        for &addr in &blocks {
            unsafe { allocator.dealloc(addr as *mut u8, layout) };
        }
        let again = unsafe { allocator.alloc(layout) };
        assert!(!again.is_null());
    }

    #[test]
    fn an_arena_just_past_a_multiple_of_the_heap_limit_is_claimed_in_full() {
        // Four heaps' worth plus one word: split greedily, the last heap would be
        // eight bytes, which talc cannot claim.
        let mut arena = vec![0u64; ARENA / 8 + 1];
        let (allocator, start, end) = split_allocator(&mut arena);

        let layout = Layout::from_size_align(HEAP / 2, 8).unwrap();
        let mut blocks = Vec::new();
        loop {
            let ptr = unsafe { allocator.alloc(layout) };
            if ptr.is_null() {
                break;
            }
            assert!(ptr as usize >= start && ptr as usize + layout.size() <= end);
            blocks.push(ptr);
        }
        // Five heaps of 192-208 KiB each hold one 128 KiB block apiece.
        assert_eq!(blocks.len(), 5);
    }

    #[test]
    fn an_allocation_larger_than_one_heap_is_refused() {
        let mut arena = vec![0u64; ARENA / 8];
        let (allocator, _, _) = split_allocator(&mut arena);

        let fits = Layout::from_size_align(HEAP / 2, 8).unwrap();
        let straddles = Layout::from_size_align(HEAP + (16 << 10), 8).unwrap();
        assert!(!unsafe { allocator.alloc(fits) }.is_null());
        assert!(unsafe { allocator.alloc(straddles) }.is_null());
    }
}
