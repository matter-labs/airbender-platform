use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::null_mut;

/// Simple bump allocator for guest programs.
pub struct BumpAllocator {
    state: UnsafeCell<BumpState>,
}

#[derive(Clone, Copy)]
struct BumpState {
    start: usize,
    end: usize,
    current: usize,
    initialized: bool,
}

unsafe impl Sync for BumpAllocator {}

impl BumpAllocator {
    pub const fn uninit() -> Self {
        Self {
            state: UnsafeCell::new(BumpState {
                start: 0,
                end: 0,
                current: 0,
                initialized: false,
            }),
        }
    }

    /// # Safety
    ///
    /// Caller must ensure `start` and `end` define a writable heap range.
    pub unsafe fn init(&self, start: *mut usize, end: *mut usize) {
        let state = &mut *self.state.get();
        state.start = start as usize;
        state.end = end as usize;
        state.current = state.start;
        state.initialized = true;
    }

    unsafe fn alloc_inner(&self, layout: Layout) -> *mut u8 {
        let state = &mut *self.state.get();
        if !state.initialized {
            return null_mut();
        }

        let align = layout.align();
        let size = layout.size();
        let aligned = (state.current + align - 1) & !(align - 1);
        let next = aligned.saturating_add(size);
        if next > state.end {
            return null_mut();
        }

        state.current = next;
        aligned as *mut u8
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_inner(layout)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // no-op: bump allocator does not free
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ptr.is_null() {
            return self
                .alloc_inner(Layout::from_size_align(new_size, layout.align()).unwrap_or(layout));
        }

        let new_layout = Layout::from_size_align(new_size, layout.align()).unwrap_or(layout);
        let new_ptr = self.alloc_inner(new_layout);
        if !new_ptr.is_null() {
            let copy_len = core::cmp::min(layout.size(), new_size);
            core::ptr::copy_nonoverlapping(ptr, new_ptr, copy_len);
        }
        new_ptr
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: BumpAllocator = BumpAllocator::uninit();

#[allow(clippy::not_unsafe_ptr_arg_deref)] // TODO: consider making it unsafe?
pub fn init(start: *mut usize, end: *mut usize) {
    unsafe { GLOBAL_ALLOCATOR.init(start, end) };
}
