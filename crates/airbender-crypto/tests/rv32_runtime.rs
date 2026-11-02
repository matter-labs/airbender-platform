//! Host harness for the real C guest, run by `crates/airbender-crypto-c/tests/rv32-runtime.py`.
//!
//! The guest reads a scenario word, then (in `csr-oracle` builds) the hint tape that a native build
//! of the same fixture recorded. Each CSR read takes the next input word, so the tape must be
//! exactly the words the guest reads, and any changed word must make the guest fail or leave every
//! result as it was.
use airbender_host::raw::{NonDeterminismCSRSource, QuasiUARTSource, RamPeek};
use airbender_host::{ExecutionResult, TranspilerRunner, TranspilerRunnerBuilder};
use std::collections::BTreeSet;
use std::ops::Range;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// The query IDs of `airbender_crypto::hints`: field ops, ModExp, wide quotient
const QUERY_IDS: [u32; 3] = [0x4005_0011, 0x4005_0010, 0x4005_0031];
/// The secp256k1 base field modulus, as little-endian words
const SECP256K1_P: [u32; 8] = [
    0xffff_fc2f,
    0xffff_fffe,
    0xffff_ffff,
    0xffff_ffff,
    0xffff_ffff,
    0xffff_ffff,
    0xffff_ffff,
    0xffff_ffff,
];
/// Tapes up to this size are tampered at every word; longer ones at a sample
const TAMPER_ALL_UP_TO: usize = 512;
/// The words drawn at random from a longer tape, besides the ends of every answer
const TAMPER_SAMPLE: usize = 256;

/// A guest query: the input position of its answer, its ID, and its field op (0 for ModExp)
#[derive(Clone, Copy, Debug)]
struct Query {
    start: usize,
    id: u32,
    op: u32,
}

/// A query and the tape positions of its answer
type Answer = (Query, Range<usize>);

#[derive(Default)]
struct Log {
    /// Input words read
    read: usize,
    /// The cycles of the run, if it stopped
    cycles: usize,
    /// The queries, with the input position of their answers
    queries: Vec<Query>,
    /// A query ID was written: the next write is the address of its request
    pending: Option<u32>,
}

/// The input of a run as the CSR source: one word per read (a host panic past the end), with the
/// guest's query writes logged
struct Input<'a> {
    words: &'a [u32],
    log: &'a mut Log,
    /// Prints what the guest writes to the UART (e.g. a panic message), for runs expected to pass
    uart: Option<QuasiUARTSource>,
}

impl Input<'_> {
    fn write(&mut self, peek: impl FnOnce(u32) -> u32, value: u32) {
        if let Some(id) = self.log.pending.take() {
            // the address of the request: the op is its first word, for the field hints
            let op = if id == QUERY_IDS[0] { peek(value) } else { 0 };
            let start = self.log.read;
            self.log.queries.push(Query { start, id, op });
        } else if QUERY_IDS.contains(&value) {
            self.log.pending = Some(value);
        } else if let Some(uart) = self.uart.as_mut() {
            uart.write_with_memory_access_raw(&[], value);
        }
    }
}

impl NonDeterminismCSRSource for Input<'_> {
    fn read(&mut self) -> u32 {
        let word = *self.words.get(self.log.read).expect("must have an answer");
        self.log.read += 1;
        word
    }

    fn write_with_memory_access<R: RamPeek + ?Sized>(&mut self, ram: &R, value: u32) {
        self.write(|address| ram.peek_word(address), value);
    }

    fn write_with_memory_access_raw(&mut self, _ram: &[u32], value: u32) {
        self.write(|_| 0, value);
    }

    fn write_with_memory_access_dyn(&mut self, ram: &dyn RamPeek, value: u32) {
        self.write(|address| ram.peek_word(address), value);
    }
}

fn runner(path: &Path) -> TranspilerRunner {
    TranspilerRunnerBuilder::new(path)
        .with_text_path(path.with_extension("text"))
        .with_ram_bound(1 << 24)
        .with_cycles(1_000_000_000)
        .build()
        .unwrap()
}

/// The output registers and final PC of a run that stopped, or the error of one that failed
#[derive(Debug, PartialEq)]
enum Outcome {
    Stopped { output: [u32; 8], final_pc: u32 },
    Failed(String),
}

fn run(runner: &TranspilerRunner, words: &[u32], log: &mut Log, verbose: bool) -> Outcome {
    let input = Input {
        words,
        log: &mut *log,
        uart: verbose.then(QuasiUARTSource::default),
    };
    let result = catch_unwind(AssertUnwindSafe(|| runner.run_with_source(input)));
    match result {
        Ok(Ok(ExecutionResult {
            receipt,
            cycles_executed,
            reached_end: true,
            final_pc,
            ..
        })) => {
            log.cycles = cycles_executed;
            Outcome::Stopped {
                output: receipt.output,
                final_pc,
            }
        }
        Ok(Ok(result)) => Outcome::Failed(format!("did not stop: {result:?}")),
        Ok(Err(error)) => Outcome::Failed(format!("host error: {error}")),
        Err(panic) => Outcome::Failed(
            panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|text| text.to_string()))
                .unwrap_or_default(),
        ),
    }
}

/// A guest failure: its panic (an illegal instruction) or the end of the input
fn trapped(outcome: &Outcome) -> bool {
    matches!(outcome, Outcome::Failed(text)
        if text.contains("Illegal instruction") || text.contains("must have an answer"))
}

/// A deterministic generator for the sample (xorshift64*)
struct Sampler(u64);

impl Sampler {
    fn below(&mut self, bound: usize) -> usize {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as usize % bound
    }
}

/// The tape positions to tamper with: every word of a short tape; otherwise the first and last
/// word of every answer, and `TAMPER_SAMPLE` more drawn with a fixed seed
fn tamper_positions(tape_len: usize, answers: &[Answer]) -> Vec<usize> {
    if tape_len <= TAMPER_ALL_UP_TO {
        return (0..tape_len).collect();
    }
    let mut positions = BTreeSet::new();
    for (_, words) in answers {
        positions.insert(words.start);
        positions.insert(words.end - 1);
    }
    let mut sampler = Sampler(0x5eed_0f7a_9e00);
    let target = positions.len() + TAMPER_SAMPLE.min(tape_len - positions.len());
    while positions.len() < target {
        positions.insert(sampler.below(tape_len));
    }
    positions.into_iter().collect()
}

/// `-x` modulo the secp256k1 prime, for the little-endian words of an `x` below it
fn negate_secp256k1(words: &mut [u32]) {
    if words.iter().all(|word| *word == 0) {
        return;
    }
    let mut borrow = 0;
    for (word, p) in words.iter_mut().zip(SECP256K1_P) {
        let difference = u64::from(p)
            .wrapping_sub(u64::from(*word))
            .wrapping_sub(borrow);
        *word = difference as u32;
        borrow = difference >> 63;
    }
}

#[test]
#[ignore = "requires an RV32 C image; run crypto-c/tests/rv32-runtime.py"]
fn c_runtime_image() {
    let path = PathBuf::from(std::env::var_os("CRYPTO_C_IMAGE").expect("CRYPTO_C_IMAGE"));
    // The native fixture's tape, one hex word per line (empty without `csr-oracle`), and the
    // digest of its results
    let tape: Vec<u32> =
        std::fs::read_to_string(std::env::var_os("CRYPTO_C_TAPE").expect("CRYPTO_C_TAPE"))
            .unwrap()
            .lines()
            .map(|word| u32::from_str_radix(word, 16).unwrap())
            .collect();
    let digest = u32::from_str_radix(
        &std::env::var("CRYPTO_C_DIGEST").expect("CRYPTO_C_DIGEST"),
        16,
    )
    .unwrap();
    let mode = std::env::var("CRYPTO_C_FEATURES").unwrap_or_default();
    let reads_hints = mode
        .split_whitespace()
        .any(|feature| feature == "csr-oracle");
    assert_eq!(
        !tape.is_empty(),
        reads_hints,
        "a tape for csr-oracle builds only"
    );
    let runner = runner(&path);
    let input = |scenario: u32, tape: &[u32]| [&[scenario], tape].concat();

    // The honest run: every result as natively, and every tape word read, no more
    let mut log = Log::default();
    let honest = run(&runner, &input(0, &tape), &mut log, true);
    let Outcome::Stopped { output, final_pc } = honest else {
        panic!("C runtime fixture failed: {honest:?}");
    };
    assert_eq!(output[0], 0, "C runtime fixture");
    assert_eq!(output[1], digest, "the results differ from the native ones");
    assert_eq!(log.read, 1 + tape.len(), "the tape is not read exactly");
    assert!(log.pending.is_none());
    // Builds without csr-oracle send no query at all
    assert_eq!(!log.queries.is_empty(), reads_hints);
    let honest_cycles = log.cycles;
    let answers: Vec<Answer> = log
        .queries
        .iter()
        .enumerate()
        .map(|(i, query)| {
            let end = log
                .queries
                .get(i + 1)
                .map_or(1 + tape.len(), |next| next.start);
            (*query, query.start - 1..end - 1)
        })
        .collect();
    if let Some((first, _)) = answers.first() {
        assert_eq!(first.start, 1, "tape words read before the first query");
    }
    assert!(
        answers.iter().all(|(_, words)| !words.is_empty()),
        "a query took no answer"
    );
    let accepted = |outcome: &Outcome| *outcome == Outcome::Stopped { output, final_pc };

    // A word past the tape is left unread
    let mut log = Log::default();
    let extended = run(
        &runner,
        &input(0, &[&tape[..], &[0x5eed_5eed]].concat()),
        &mut log,
        true,
    );
    assert!(accepted(&extended), "{extended:?}");
    assert_eq!(log.read, 1 + tape.len());

    // Duplicate heap initialization traps
    let outcome = run(&runner, &input(1, &tape), &mut Log::default(), false);
    assert!(
        trapped(&outcome),
        "duplicate heap initialization: {outcome:?}"
    );

    let (mut tampered, mut unchanged) = (0, 0);
    if reads_hints {
        // A truncated tape traps, at its last word or halfway
        for len in [tape.len() - 1, tape.len() / 2] {
            let outcome = run(&runner, &input(0, &tape[..len]), &mut Log::default(), false);
            assert!(trapped(&outcome), "a tape of {len} words: {outcome:?}");
        }
        // The other root in every secp256k1 square-root answer (op 0: the root's Montgomery
        // words, then the flag) is valid too, and must leave every result unchanged
        let mut other_roots = tape.clone();
        let mut roots = 0;
        for (query, words) in &answers {
            if query.id == QUERY_IDS[0] && query.op == 0 && words.len() == 9 {
                negate_secp256k1(&mut other_roots[words.start..words.start + 8]);
                roots += 1;
            }
        }
        assert!(roots > 0, "the fixture takes square roots");
        let outcome = run(&runner, &input(0, &other_roots), &mut Log::default(), true);
        assert!(accepted(&outcome), "the other roots: {outcome:?}");
        // An all-zero tape traps
        let zeroes = vec![0; tape.len()];
        let outcome = run(&runner, &input(0, &zeroes), &mut Log::default(), false);
        assert!(trapped(&outcome), "an all-zero tape: {outcome:?}");

        // Tampered words (XOR 1): the guest fails, or every result is as it was
        let positions = tamper_positions(tape.len(), &answers);
        tampered = positions.len();
        let unchanged_count = AtomicUsize::new(0);
        let unexpected = Mutex::new(Vec::new());
        let next = AtomicUsize::new(0);
        let threads = std::thread::available_parallelism()
            .map_or(1, |n| n.get())
            .min(8);
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        std::thread::scope(|scope| {
            for _ in 0..threads {
                scope.spawn(|| {
                    let runner = self::runner(&path);
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(&position) = positions.get(i) else {
                            break;
                        };
                        let mut words = input(0, &tape);
                        words[1 + position] ^= 1;
                        let outcome = run(&runner, &words, &mut Log::default(), false);
                        if accepted(&outcome) {
                            unchanged_count.fetch_add(1, Ordering::Relaxed);
                        } else if !trapped(&outcome) {
                            unexpected.lock().unwrap().push((position, outcome));
                        }
                    }
                });
            }
        });
        std::panic::set_hook(hook);
        let unexpected = unexpected.into_inner().unwrap();
        assert!(
            unexpected.is_empty(),
            "tampered words that neither trapped nor left the results unchanged: {unexpected:?}"
        );
        unchanged = unchanged_count.into_inner();
    }
    println!(
        "FullUnsigned C runtime: features={mode:?}, cycles={honest_cycles}, tape words={}, \
         queries={}, tampered positions={tampered} ({unchanged} left the results unchanged)",
        tape.len(),
        answers.len(),
    );
}
