use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::{null_mut, NonNull};

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
        let state = &mut *self.state.get();
        let base = start.cast::<u8>();
        let size = (end as usize).saturating_sub(start as usize);
        if size == 0 {
            state.allocator = None;
            return;
        }

        let mut allocator = talc::Talc::new(talc::ClaimOnOom::new(talc::Span::empty()));
        let span = talc::Span::from_base_size(base, size);
        allocator.claim(span).expect("must claim initial heap span");
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
            self.dealloc_inner(ptr, layout);
        }
        new_ptr
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: TalcAllocator = TalcAllocator::uninit();

#[allow(clippy::not_unsafe_ptr_arg_deref)] // TODO: consider making it unsafe?
pub fn init(start: *mut usize, end: *mut usize) {
    unsafe { GLOBAL_ALLOCATOR.init(start, end) };
}
