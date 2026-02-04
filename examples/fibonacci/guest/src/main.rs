#![no_std]
#![no_main]

use airbender::guest::read;

/// Reads `n` from the host and returns the nth Fibonacci number.
#[airbender::main]
fn main() -> u32 {
    let n: u32 = read().expect("failed to read input");
    fib(n)
}

fn fib(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        _ => {
            let mut a = 0u32;
            let mut b = 1u32;
            let mut i = 2u32;
            while i <= n {
                let next = a.wrapping_add(b);
                a = b;
                b = next;
                i += 1;
            }
            b
        }
    }
}
