//! UART writer compatible with the existing quasi-uart protocol.

use crate::sys;
use core::fmt;

/// Minimal UART writer compatible with the existing quasi-uart protocol.
#[derive(Default)]
pub struct QuasiUart {
    buffer: [u8; 4],
    len: usize,
}

impl QuasiUart {
    const HELLO_MARKER: u32 = u32::MAX;

    #[inline(never)]
    pub const fn new() -> Self {
        Self {
            buffer: [0u8; 4],
            len: 0,
        }
    }

    #[inline(never)]
    pub fn write_entry_sequence(&mut self, message_len: usize) {
        sys::write_word(Self::HELLO_MARKER);
        sys::write_word(message_len.div_ceil(4) as u32 + 1);
        sys::write_word(message_len as u32);
    }

    #[inline(never)]
    fn write_byte(&mut self, byte: u8) {
        self.buffer[self.len] = byte;
        self.len += 1;
        if self.len == 4 {
            self.len = 0;
            let word = u32::from_le_bytes(self.buffer);
            sys::write_word(word);
        }
    }

    fn flush(&mut self) {
        if self.len == 0 {
            self.buffer.fill(0);
            return;
        }
        for i in self.len..4 {
            self.buffer[i] = 0u8;
        }
        self.len = 0;
        sys::write_word(u32::from_le_bytes(self.buffer));
    }
}

impl fmt::Write for QuasiUart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_entry_sequence(s.len());
        for c in s.bytes() {
            self.write_byte(c);
        }
        self.flush();
        Ok(())
    }
}
