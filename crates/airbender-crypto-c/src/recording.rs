//! The recording of each thread, for the C recording API: a C caller cannot pass a recorder
//! through its calls. Native builds only.

use crate::crypto::hints::RecordingHintOracle;
use alloc::vec::Vec;
use core::cell::RefCell;

std::thread_local! {
    static ACTIVE_RECORDER: RefCell<Option<RecordingHintOracle>> = const { RefCell::new(None) };
}

/// Starts a recording on this thread; one in progress starts over.
pub(crate) fn begin_recording() {
    ACTIVE_RECORDER.set(Some(RecordingHintOracle::new()));
}

/// Ends the recording of this thread and returns its tape; empty without a recording.
pub(crate) fn end_recording() -> Vec<u32> {
    ACTIVE_RECORDER
        .take()
        .map(RecordingHintOracle::into_tape)
        .unwrap_or_default()
}

/// Runs `f` with this thread's recorder, `None` without a recording.
///
/// The recorder dereferences the requests sent to it, but sending a query is `unsafe`, so safe
/// code in `f` can only reach it through airbender-crypto's hinted operations.
///
/// # Panics
///
/// If `f` calls this function, [`begin_recording`] or [`end_recording`].
pub(crate) fn with_active_recorder<R>(f: impl FnOnce(Option<&mut RecordingHintOracle>) -> R) -> R {
    ACTIVE_RECORDER.with_borrow_mut(|recorder| f(recorder.as_mut()))
}
