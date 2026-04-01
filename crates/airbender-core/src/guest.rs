//! Guest-side output commitment traits shared between host and guest crates.

/// Values that can be committed to the public output registers (`x10..x17`).
pub trait Commit {
    fn commit_words(&self) -> [u32; 8];
}

impl Commit for () {
    fn commit_words(&self) -> [u32; 8] {
        [0u32; 8]
    }
}

impl Commit for u32 {
    fn commit_words(&self) -> [u32; 8] {
        let mut words = [0u32; 8];
        words[0] = *self;
        words
    }
}

impl Commit for u64 {
    fn commit_words(&self) -> [u32; 8] {
        let mut words = [0u32; 8];
        words[0] = *self as u32;
        words[1] = (*self >> 32) as u32;
        words
    }
}

impl Commit for i64 {
    fn commit_words(&self) -> [u32; 8] {
        (*self as u64).commit_words()
    }
}

impl Commit for bool {
    fn commit_words(&self) -> [u32; 8] {
        let mut words = [0u32; 8];
        words[0] = u32::from(*self);
        words
    }
}

impl Commit for [u32; 8] {
    fn commit_words(&self) -> [u32; 8] {
        *self
    }
}

impl<T: Commit, E: core::fmt::Debug> Commit for Result<T, E> {
    fn commit_words(&self) -> [u32; 8] {
        match self {
            Ok(val) => val.commit_words(),
            Err(e) => panic!("committed a Result::Err: {:?}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_words_u64_layout() {
        let value: u64 = 0x11223344_55667788;
        let words = <u64 as Commit>::commit_words(&value);
        assert_eq!(words[0], 0x55667788);
        assert_eq!(words[1], 0x11223344);
        assert_eq!(words[2], 0);
    }

    #[test]
    fn commit_words_bool_layout() {
        let words = <bool as Commit>::commit_words(&true);
        assert_eq!(words[0], 1);
        let words = <bool as Commit>::commit_words(&false);
        assert_eq!(words[0], 0);
    }

    #[test]
    fn commit_words_result_ok() {
        let result: Result<u32, &str> = Ok(42);
        let words = result.commit_words();
        assert_eq!(words[0], 42);
        assert_eq!(words[1], 0);
    }

    #[test]
    #[should_panic(expected = "committed a Result::Err")]
    fn commit_words_result_err_panics() {
        let result: Result<u32, &str> = Err("something went wrong");
        result.commit_words();
    }
}
