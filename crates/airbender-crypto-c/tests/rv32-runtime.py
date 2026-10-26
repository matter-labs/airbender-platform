#!/usr/bin/env python3
"""Link a C-owned startup/libc and execute it on FullUnsigned, including tampered hint tapes.

The fixture (tests/runtime.c) is first built natively without features and run under the C
recording API: it records the hint tape that an RV32 csr-oracle build reads, and the digest of
its results. Each RV32 build must end with that digest. Only csr-oracle builds read the tape,
after their scenario word; the others get an empty one and must send no query. Requires clang,
Python 3, and the pinned Rust toolchain with llvm-tools-preview. Run from any directory. --all
checks the configurations plain, delegation and csr-oracle + delegation.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
TARGET = 'riscv32im-risc0-zkvm-elf'
CONFIGURATIONS = ['', 'delegation', 'csr-oracle delegation']


def run(*args, **kwargs):
    return subprocess.run(args, cwd=ROOT, check=True, text=True, **kwargs)


def target_dir():
    metadata = run('cargo', 'metadata', '--no-deps', '--format-version', '1', capture_output=True).stdout
    return Path(json.loads(metadata)['target_directory'])


def record(out):
    """The digest of the fixture's results and its hint tape, from a native build without features."""
    env = dict(os.environ)
    env.pop('RUSTFLAGS', None)
    link = run('cargo', 'rustc', '-p', 'airbender-crypto-c', '--release', '--crate-type', 'staticlib',
               '--', '--print', 'native-static-libs', capture_output=True, env=env).stderr
    native_libs = re.findall(r'native-static-libs: (.*)$', link, re.M)[-1].split()
    run('clang', '-O2', '-I' + str(ROOT / 'crates/airbender-crypto-c/include'),
        str(ROOT / 'crates/airbender-crypto-c/tests/runtime.c'),
        str(target_dir() / 'release/libairbender_crypto_c.a'), *native_libs, '-o', str(out / 'record'))
    lines = run(str(out / 'record'), capture_output=True).stdout.splitlines()
    digest = re.fullmatch(r'digest ([0-9a-f]{8})', lines[0])[1]
    return digest, lines[1:]


def check(features, digest, tape):
    env = dict(os.environ, FEATURES=features)
    run('sh', 'crates/airbender-crypto-c/build-rv32.sh', env=env)
    sysroot = Path(run('rustc', '--print', 'sysroot', capture_output=True).stdout.strip())
    host = re.search(r'^host: (.+)$', run('rustc', '-vV', capture_output=True).stdout, re.M)[1]
    llvm = sysroot / 'lib/rustlib' / host / 'bin'
    archive = target_dir() / TARGET / 'release/libairbender_crypto_c.a'
    symbols = run(str(llvm / 'llvm-nm'), '--defined-only', str(archive), capture_output=True).stdout
    assert not re.search(r' T (_start|memcpy|memmove|memset)$', symbols, re.M), 'archive owns startup/libc'
    with tempfile.TemporaryDirectory(prefix='crypto-c-rv32-') as directory:
        out = Path(directory)
        reads_hints = 'csr-oracle' in features.split()
        (out / 'tape.hex').write_text(''.join(word + '\n' for word in tape) if reads_hints else '')
        run('clang', '--target=riscv32-none-elf', '-march=rv32im', '-mabi=ilp32',
            '-ffreestanding', '-fno-builtin', '-O2', '-S', '-emit-llvm',
            '-I' + str(ROOT / 'crates/airbender-crypto-c/include'),
            str(ROOT / 'crates/airbender-crypto-c/tests/runtime.c'), '-o', str(out / 'runtime.ll'))
        ir = (out / 'runtime.ll').read_text()
        ir = re.sub(r'"target-features"="[^"]*"', '"target-features"="+m"', ir)
        (out / 'runtime.ll').write_text(ir)
        # Startup is owned by the consumer. Copy constants/data out of ROM before
        # any delegation (its operands must be in RAM), and provide the stack. The
        # fixture's status goes to a0 and the digest of its results to a1. The heap
        # is 1 MiB: the hinted bn254 pairing builds the 16 KB lines of each G2 point.
        (out / 'start.ll').write_text(r'''
module asm ".section .text.start,\22ax\22"
module asm ".globl _start"
module asm "_start:"
module asm "li sp, 0xf00000"
module asm "call consumer_main"
module asm "j ."
@_sdata = external global i8
@_edata = external global i8
@_sidata = external global i8
@_sbss = external global i8
@_ebss = external global i8
declare ptr @memcpy(ptr, ptr, i32)
declare ptr @memset(ptr, i32, i32)
declare i32 @airbender_crypto_runtime_smoke(ptr, ptr, i32, ptr)
define void @consumer_main() nounwind {
  %digest = alloca i32, align 4
  %size = sub i32 ptrtoint(ptr @_edata to i32), ptrtoint(ptr @_sdata to i32)
  %copy = call ptr @memcpy(ptr @_sdata, ptr @_sidata, i32 %size)
  %bsssize = sub i32 ptrtoint(ptr @_ebss to i32), ptrtoint(ptr @_sbss to i32)
  %zero = call ptr @memset(ptr @_sbss, i32 0, i32 %bsssize)
  %scenario = call i32 asm sideeffect "csrrw $0, 0x7c0, zero", "=r"()
  %result = call i32 @airbender_crypto_runtime_smoke(ptr inttoptr(i32 8388608 to ptr), ptr inttoptr(i32 9437184 to ptr), i32 %scenario, ptr %digest)
  %value = load i32, ptr %digest, align 4
  call void asm sideeffect "", "{a0},{a1}"(i32 %result, i32 %value)
  br label %exit
exit:
  br label %exit
}
''')
        for name in ['runtime', 'start']:
            run(str(llvm / 'llc'), '-mtriple=riscv32-unknown-none-elf', '-mattr=+m',
                '-filetype=obj', str(out / (name + '.ll')), '-o', str(out / (name + '.o')))
        (out / 'link.ld').write_text('''ENTRY(_start)
SECTIONS {
 . = 0; .text : { *(.text.start) *(.text .text.*) }
 _sidata = ALIGN(., 32);
 .data 0x500000 : AT(_sidata) { _sdata = .; *(.rodata .rodata.* .srodata .srodata.* .data .data.* .sdata .sdata.*) _edata = .; }
 .bss (NOLOAD) : { _sbss = .; *(.bss .bss.* .sbss .sbss.*) _ebss = .; }
}
''')
        run(str(llvm / 'rust-lld'), '-flavor', 'gnu', '-m', 'elf32lriscv',
            '--gc-sections', '--no-relax', '-T', str(out / 'link.ld'),
            str(out / 'start.o'), str(out / 'runtime.o'), str(archive), '-o', str(out / 'guest.elf'))
        for extension, sections in [('bin', []), ('text', ['--only-section=.text'])]:
            run(str(llvm / 'llvm-objcopy'), '-O', 'binary', *sections,
                str(out / 'guest.elf'), str(out / ('guest.' + extension)))
        env.update(CRYPTO_C_IMAGE=str(out / 'guest.bin'), CRYPTO_C_TAPE=str(out / 'tape.hex'),
                   CRYPTO_C_DIGEST=digest, CRYPTO_C_FEATURES=features)
        selection = ['--release', '--test', 'rv32_runtime', 'c_runtime_image']
        run('cargo', 'test', '-p', 'airbender-crypto', *selection, '--', '--ignored', '--nocapture', env=env)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--all', action='store_true')
    parser.add_argument('--features', default=os.environ.get('FEATURES', 'delegation'))
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='crypto-c-native-') as directory:
        digest, tape = record(Path(directory))
    print(f'native fixture: digest {digest}, {len(tape)} tape words')
    for selection in CONFIGURATIONS if args.all else [args.features]:
        check(selection, digest, tape)
