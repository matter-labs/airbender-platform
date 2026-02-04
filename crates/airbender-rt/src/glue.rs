use crate::uart::QuasiUart;
use std::alloc::{Allocator, Layout};
use std::fmt::Write as _;

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn sys_halt() {
    crate::sys::exit_error();
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn sys_rand(_recv_buf: *mut u32, _words: usize) {}

/// # Safety
///
/// This function is called by the standard library with trusted pointers.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe fn sys_panic(msg_ptr: *const u8, len: usize) -> ! {
    let msg = core::str::from_raw_parts(msg_ptr, len);
    let mut uart = QuasiUart::new();
    let _ = uart.write_str("PANIC: ");
    let _ = uart.write_str(msg);
    sys_halt();
    core::hint::unreachable_unchecked()
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn sys_log(_msg_ptr: *const u8, _len: usize) {}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn sys_read(_fd: u32, _recv_buf: *mut u8, _nrequested: usize) -> usize {
    0
}

/// # Safety
///
/// This function is called by the standard library with trusted pointers.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe fn sys_write(fd: u32, write_buf: *const u8, nbytes: usize) {
    if fd != 1 && fd != 2 {
        return;
    }
    let msg = core::str::from_raw_parts(write_buf, nbytes);
    let mut uart = QuasiUart::new();
    let _ = uart.write_str(msg);
}

/// # Safety
///
/// This function is called by the standard library with trusted pointers.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe fn sys_getenv(
    _recv_buf: *mut u32,
    _words: usize,
    _varname: *const u8,
    _varname_len: usize,
) -> usize {
    0
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn sys_argc() -> usize {
    0
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn sys_argv(_out_words: *mut u32, _out_nwords: usize, _arg_index: usize) -> usize {
    0
}

const WORD_SIZE: usize = core::mem::size_of::<u32>();

#[inline(never)]
#[unsafe(no_mangle)]
pub fn sys_alloc_words(nwords: usize) -> *mut u32 {
    std::alloc::Global
        .allocate(Layout::from_size_align(nwords * WORD_SIZE, WORD_SIZE).expect("Layout failed"))
        .expect("Allocation failed")
        .as_ptr() as *mut u32
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn sys_alloc_aligned(nwords: usize, align: usize) -> *mut u8 {
    std::alloc::Global
        .allocate(Layout::from_size_align(nwords * WORD_SIZE, align).expect("Layout failed"))
        .expect("Allocation failed")
        .as_ptr() as *mut u8
}
