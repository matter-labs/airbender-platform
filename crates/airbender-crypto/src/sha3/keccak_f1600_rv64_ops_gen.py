#!/usr/bin/env python3
"""Generator of `keccak_f1600_rv64_ops.S`: Keccak-f[1600] for RV64 with binbender's Keccak opcodes.

    python3 keccak_f1600_rv64_ops_gen.py           # rewrite keccak_f1600_rv64_ops.S
    python3 keccak_f1600_rv64_ops_gen.py --check   # emulate every entry against FIPS 202
    python3 keccak_f1600_rv64_ops_gen.py --counts  # executed instructions and memory accesses

The entries, their ABI, the frame and the lane-complemented representation in memory are those
of `keccak_f1600_rv64.S` (see its header); only the rounds differ. The opcodes are binbender's
(github.com/matter-labs/binbender, crates/machine/src/isa.rs, `Opcode::{Xri, Rax1, Bcax, Eor3}`):

    XRI  rd, rs1, rs2, k   rd = (rs1 ^ rs2) >>> k          custom-1 (0x2b) f3 0, k in bits 30:25
    RAX1 rd, rs1, rs2      rd = rs1 ^ (rs2 <<< 1)          custom-1 (0x2b) f3 6, f7 0
    BCAX rd, rs1, rs2, rs3 rd = rs3 ^ (rs1 & ~rs2)         custom-3 (0x7b) f3 1, f2 0 (R4)
    EOR3 rd, rs1, rs2, rs3 rd = rs1 ^ rs2 ^ rs3            custom-3 (0x7b) f3 2, f2 0 (R4)

Design:

* The 25 lanes live in 25 of the 30 general registers (all but `zero` and `sp`) for the whole
  permutation; five are temporaries. No lane is spilled.
* Lanes are renamed instead of moved: an instruction writes its result over an operand that
  dies, and the generator tracks which register holds which lane. Rho and pi are then 24 XRI in
  place and a relabelling; chi writes two outputs of a row into temporaries and the rest in place.
  The rounds therefore start from the canonical registers (those of `keccak_f1600_rv64.S`) and
  end elsewhere; the exits store from wherever the lanes are, and only the absorb loop moves
  them back before its next block.
* Theta: the five parities take two EOR3 each and D[x] = C[x-1] ^ rotl(C[x+1], 1) one RAX1. D0 is
  applied to its column at once (five xors, one of which lane 0, which does not rotate, needs
  anyway), which frees the register needed for the last parity; the other four D stay for rho,
  where XRI applies D and the rotation in one instruction.
* Chi: out[x] = B[x] ^ (~B[x+1] & B[x+2]) is BCAX(B[x+2], B[x+1], B[x]).
* Iota: an immediate xor if the constant fits in 12 bits, else a PC-relative load and a xor.
* The lanes stored complemented are complemented on load and on store; inside they are plain.
"""
import argparse
import os
import random
import sys

M64 = (1 << 64) - 1
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "keccak_f1600_rv64_ops.S")
SYMBOL = "airbender_keccak_f1600_rv64"


def rol(v, n):
    n %= 64
    return ((v << n) | (v >> (64 - n))) & M64 if n else v


def rho_offsets():
    r = [0] * 25
    x, y = 1, 0
    for t in range(24):
        r[x + 5 * y] = ((t + 1) * (t + 2) // 2) % 64
        x, y = y, (2 * x + 3 * y) % 5
    return r


def round_constants():
    out, lfsr = [], 1
    for _ in range(24):
        c = 0
        for j in range(7):
            if lfsr & 1:
                c |= 1 << ((1 << j) - 1)
            lfsr = (lfsr << 1) ^ (0x171 if lfsr & 0x80 else 0)
        out.append(c)
    return out


RHO = rho_offsets()
RC = round_constants()


def pi_dest(lane):
    x, y = lane % 5, lane // 5
    return y + 5 * ((2 * x + 3 * y) % 5)


def keccak_f1600(state):
    a = list(state)
    for rc in RC:
        c = [a[x] ^ a[x + 5] ^ a[x + 10] ^ a[x + 15] ^ a[x + 20] for x in range(5)]
        d = [c[(x - 1) % 5] ^ rol(c[(x + 1) % 5], 1) for x in range(5)]
        a = [a[i] ^ d[i % 5] for i in range(25)]
        b = [0] * 25
        for i in range(25):
            b[pi_dest(i)] = rol(a[i], RHO[i])
        a = [b[i] ^ (~b[(i % 5 + 1) % 5 + 5 * (i // 5)] & b[(i % 5 + 2) % 5 + 5 * (i // 5)] & M64)
             for i in range(25)]
        a[0] ^= rc
    return a


FIPS_ZERO = [  # Keccak-f[1600] of the zero state, from the Keccak team's KeccakF-1600 test vectors
    0xF1258F7940E1DDE7, 0x84D5CCF933C0478A, 0xD598261EA65AA9EE, 0xBD1547306F80494D,
    0x8B284E056253D057, 0xFF97A42D7F8E6FD4, 0x90FEE5A0A44647C4, 0x8C5BDA0CD6192E76,
    0xAD30A6F71B19059C, 0x30935AB7D08FFC64, 0xEB5AA93F2317D635, 0xA9A6E6260D712103,
    0x81A57C16DBCF555F, 0x43B831CD0347C826, 0x01F22F1A11A5569F, 0x05E5635A21D9AE61,
    0x64BEFEF28CC970F2, 0x613670957BC46611, 0xB87C5A554FD00ECB, 0x8C3EE88A1CCF32C8,
    0x940C7922AE3A2614, 0x1841F924A2C509E4, 0x16F53526E70465C2, 0x75F644E97F30A13B,
    0xEAF1FF7B5CECA249,
]

# ---------------------------------------------------------------------------------------------
# Registers, the frame and the representation, as in keccak_f1600_rv64.S
# ---------------------------------------------------------------------------------------------

ABI = ("zero ra sp gp tp t0 t1 t2 s0 s1 a0 a1 a2 a3 a4 a5 a6 a7 "
       "s2 s3 s4 s5 s6 s7 s8 s9 s10 s11 t3 t4 t5 t6").split()
XNUM = {name: i for i, name in enumerate(ABI)}
CANON = ("t0 t1 t2 t3 t4 t5 t6 a5 a6 a7 s0 s1 s2 s3 s4 "
         "s5 s6 s7 s8 s9 s10 s11 ra gp tp").split()
TEMPS = "a0 a1 a2 a3 a4".split()
SAVED = ["ra", "gp", "tp"] + ["s%d" % i for i in range(12)]
FRAME = 144  # 0 state or digest pointer, 8 blocks left (-1: digest), 16 data pointer, 24.. saved
COMPLEMENTED = (3, 7, 8, 12, 17, 23)
RATE = 17
SHORT = 9


def complement_of(lane):
    return M64 if lane in COMPLEMENTED else 0


# ---------------------------------------------------------------------------------------------
# Instructions: a list of tuples, rendered to text and emulated from the same list
# ---------------------------------------------------------------------------------------------


class Code:
    def __init__(self):
        self.ins = []
        self.pc_labels = 0

    def add(self, *op, c=""):
        self.ins.append(op + (c,))

    def render(self):
        out = []
        for op in self.ins:
            name, args, c = op[0], op[1:-1], op[-1]
            if name == "label":
                out.append("%s:" % args[0])
                continue
            if name == "raw":
                out.append(args[0])
                continue
            if name == "phase":
                out.append("    # -- %s --" % args[0])
                continue
            if name == "rc":
                rd, index = args
                self.pc_labels += 1
                label = ".L%s_ops_pc%d" % (SYMBOL, self.pc_labels)
                out.append(label + ":")
                out.append("    auipc %s, %%pcrel_hi(.L%s_ops_rc+%d)" % (rd, SYMBOL, 8 * index))
                text = "ld %s, %%pcrel_lo(%s)(%s)" % (rd, label, rd)
                c = "RC[%d] = 0x%016x" % (index, RC[index])
            elif name in ("ld", "sd"):
                text = "%s %s, %d(%s)" % (name, args[0], args[2], args[1])
            elif name in ("xor", "or"):
                text = "%s %s, %s, %s" % (name, *args)
            elif name in ("xori", "addi", "slli"):
                text = "%s %s, %s, %d" % (name, *args)
            elif name == "xri":
                rd, a, b, k = args
                text = ".insn r 0x2b, 0, %d, x%d, x%d, x%d" % (k, XNUM[rd], XNUM[a], XNUM[b])
                c = c or "xri %s, %s, %s, %d" % (rd, a, b, k)
            elif name == "rax1":
                rd, a, b = args
                text = ".insn r 0x2b, 6, 0, x%d, x%d, x%d" % (XNUM[rd], XNUM[a], XNUM[b])
                c = c or "rax1 %s, %s, %s" % (rd, a, b)
            elif name in ("bcax", "eor3"):
                rd, a, b, d = args
                f3 = 1 if name == "bcax" else 2
                text = ".insn r4 0x7b, %d, 0, x%d, x%d, x%d, x%d" % (
                    f3, XNUM[rd], XNUM[a], XNUM[b], XNUM[d])
                c = c or "%s %s, %s, %s, %s" % (name, rd, a, b, d)
            elif name in ("beqz", "bnez", "bltz"):
                text = "%s %s, %s" % (name, args[0], args[1])
            elif name == "j":
                text = "jal zero, %s" % args[0]
            elif name == "ret":
                text = "jalr zero, 0(ra)"
            else:
                raise ValueError(name)
            out.append("    %-40s # %s" % (text, c) if c else "    " + text)
        return out


class Lanes:
    """Which register holds which lane, and which registers are free."""

    def __init__(self, regs=None):
        self.reg = list(regs or CANON)
        self.free = [r for r in TEMPS if r not in self.reg] if regs is None else [
            r for r in ABI if r not in ("zero", "sp") and r not in self.reg]

    def take(self):
        return self.free.pop(0)

    def give(self, r):
        assert r not in self.free and r not in ("zero", "sp")
        self.free.append(r)


def emit_round(e, L, index, digest=False):
    """One round over lanes `L` (renamed in place). With `digest`, only lanes 0-3 are computed
    and the result is their registers; the other lanes are dead."""
    R = L.reg
    e.add("phase", "theta")

    def parity(x):
        t = L.take()
        e.add("eor3", t, R[x], R[x + 5], R[x + 10], c="C%d = A%d ^ A%d ^ A%d" % (x, x, x + 5, x + 10))
        e.add("eor3", t, t, R[x + 15], R[x + 20], c="C%d ^= A%d ^ A%d" % (x, x + 15, x + 20))
        return t

    # C0 is read off column 0 before D0 changes it
    C4, C1, C0 = parity(4), parity(1), parity(0)
    D0 = L.take()
    e.add("rax1", D0, C4, C1, c="D0 = C4 ^ rotl(C1, 1)")
    for y in range(5):
        e.add("xor", R[5 * y], R[5 * y], D0, c="A%d ^= D0" % (5 * y))
    L.give(D0)
    C2 = parity(2)
    e.add("rax1", C4, C2, C4, c="D3 = C2 ^ rotl(C4, 1), over C4")
    D3 = C4
    e.add("rax1", C2, C0, C2, c="D1 = C0 ^ rotl(C2, 1), over C2")
    D1 = C2
    C3 = parity(3)
    e.add("rax1", C0, C3, C0, c="D4 = C3 ^ rotl(C0, 1), over C0")
    D4 = C0
    e.add("rax1", C1, C1, C3, c="D2 = C1 ^ rotl(C3, 1), over C1")
    D2 = C1
    L.give(C3)
    D = ["zero", D1, D2, D3, D4]

    e.add("phase", "rho pi")
    B = [None] * 25
    needed = range(25)
    if digest:  # row 0 of B: lanes 0, 6, 12, 18, 24
        needed = [i for i in range(25) if pi_dest(i) < 5]
    for src in needed:
        dst = pi_dest(src)
        if src != 0:  # lane 0 neither moves nor rotates
            e.add("xri", R[src], R[src], D[src % 5], 64 - RHO[src],
                  c="B%d = rotl(A%d ^ D%d, %d)" % (dst, src, src % 5, RHO[src]))
        B[dst] = R[src]
    for t in (D1, D2, D3, D4):
        L.give(t)
    if digest:
        for src in range(25):
            if pi_dest(src) >= 5:
                L.give(R[src])
    L.reg = B

    e.add("phase", "chi")
    R = L.reg
    rows = [0] if digest else range(5)
    for y in rows:
        b = [R[5 * y + x] for x in range(5)]
        out = [None] * 5
        for x in (0, 1):
            out[x] = L.take()
        for x in range(5) if not digest else range(4):
            if x >= 2:
                out[x] = b[x]
            e.add("bcax", out[x], b[(x + 2) % 5], b[(x + 1) % 5], b[x],
                  c="A%d = B%d ^ (~B%d & B%d)" % (5 * y + x, 5 * y + x, 5 * y + (x + 1) % 5,
                                                 5 * y + (x + 2) % 5))
        if digest:
            L.give(b[4])
        L.give(b[0])
        L.give(b[1])
        for x in range(5 if not digest else 4):
            R[5 * y + x] = out[x]

    e.add("phase", "iota")
    rc = RC[index]
    if rc < 2048:
        e.add("xori", R[0], R[0], rc, c="A0 ^= 0x%x" % rc)
    else:
        t = L.take()
        e.add("rc", t, index)
        e.add("xor", R[0], R[0], t, c="A0 ^= RC[%d]" % index)
        L.give(t)
    assert len(L.free) == (5 if not digest else 26), L.free


def fixup(e, L):
    """Moves the lanes back into the canonical registers, through a free register."""
    where = {r: lane for lane, r in enumerate(L.reg)}
    want = {lane: CANON[lane] for lane in range(25)}
    moves = 0
    pending = {lane for lane in range(25) if L.reg[lane] != want[lane]}
    while pending:
        # a lane whose canonical register is free (held by no lane) moves first
        ready = [l for l in pending if want[l] not in where]
        if ready:
            lane = min(ready)
            src = L.reg[lane]
            e.add("addi", want[lane], src, 0, c="A%d" % lane)
            del where[src]
            where[want[lane]] = lane
            L.reg[lane] = want[lane]
            pending.discard(lane)
        else:  # a cycle: park one lane in a free register
            lane = min(pending)
            spare = next(r for r in TEMPS if r not in where)
            src = L.reg[lane]
            e.add("addi", spare, src, 0, c="A%d, parked" % lane)
            del where[src]
            where[spare] = lane
            L.reg[lane] = spare
        moves += 1
    L.free = [r for r in TEMPS]
    return moves


def frame_in(e, entry_args):
    e.add("addi", "sp", "sp", -FRAME)
    for reg, off, c in entry_args:
        e.add("sd", reg, "sp", off, c=c)
    for i, r in enumerate(SAVED):
        e.add("sd", r, "sp", 24 + 8 * i)


def frame_out(e):
    for i, r in enumerate(SAVED):
        e.add("ld", r, "sp", 24 + 8 * i)
    e.add("addi", "sp", "sp", FRAME)
    e.add("ret")


def load_state(e, base):
    """The 25 lanes from `base` (a temporary, loaded last), complemented ones made plain."""
    order = [l for l in range(25) if CANON[l] != base] + [l for l in range(25) if CANON[l] == base]
    for lane in order:
        e.add("ld", CANON[lane], base, 8 * lane, c="A%d" % lane)
    for lane in COMPLEMENTED:
        e.add("xori", CANON[lane], CANON[lane], -1, c="A%d is stored complemented" % lane)


def absorb_block(e, first):
    """The next block of data into the canonical lanes; the counter and pointer advance."""
    e.add("ld", "a1", "sp", 16, c="data pointer")
    e.add("ld", "a0", "sp", 8, c="blocks left")
    for lane in range(RATE):
        if first:
            e.add("ld", CANON[lane], "a1", 8 * lane, c="A%d = data[%d]" % (lane, lane))
        else:
            e.add("ld", "a2", "a1", 8 * lane)
            e.add("xor", CANON[lane], CANON[lane], "a2", c="A%d ^= data[%d]" % (lane, lane))
    if first:
        for lane in range(RATE, 25):
            e.add("addi", CANON[lane], "zero", 0, c="A%d = 0" % lane)
    e.add("addi", "a1", "a1", 8 * RATE)
    e.add("addi", "a0", "a0", -1)
    e.add("sd", "a1", "sp", 16)
    e.add("sd", "a0", "sp", 8)


def emit():
    e = Code()
    lab = lambda name: ".L%s_ops_%s" % (SYMBOL, name)
    rounds = "%s_rounds" % SYMBOL

    def entry(name, comment):
        e.add("raw", "")
        e.add("raw", "    .globl %s" % name)
        e.add("raw", "    .type %s, @function" % name)
        e.add("raw", "    .p2align 2")
        e.add("label", name)
        e.add("phase", comment)

    def end(name):
        e.add("raw", "    .size %s, . - %s" % (name, name))

    # state <- Keccak-f(state)
    entry(SYMBOL, "entry")
    frame_in(e, [("zero", 8, "0: the state leaves the registers"), ("a0", 0, "state pointer")])
    load_state(e, "a0")
    e.add("j", rounds)
    end(SYMBOL)

    # absorb `blocks` blocks into the state
    name = SYMBOL + "_absorb"
    entry(name, "entry")
    frame_in(e, [("a2", 8, "blocks left"), ("a1", 16, "data pointer"), ("a0", 0, "state pointer")])
    load_state(e, "a0")
    e.add("label", lab("block"))
    e.add("phase", "block")
    absorb_block(e, first=False)
    e.add("j", rounds)
    end(name)

    # absorb from the zero state
    name = SYMBOL + "_absorb_first"
    entry(name, "entry")
    frame_in(e, [("a2", 8, "blocks left"), ("a1", 16, "data pointer"), ("a0", 0, "state pointer")])
    absorb_block(e, first=True)
    e.add("j", rounds)
    end(name)

    # digest of the last permutation of a state
    name = SYMBOL + "_final"
    entry(name, "entry")
    e.add("addi", "a2", "zero", -1)
    frame_in(e, [("a2", 8, "-1: the digest leaves the registers"), ("a1", 0, "digest pointer")])
    load_state(e, "a0")
    e.add("j", rounds)
    end(name)

    # a whole hash of at most 64 bytes from its nine plain lanes
    name = SYMBOL + "_final64"
    entry(name, "entry")
    e.add("addi", "a2", "zero", -1)
    frame_in(e, [("a2", 8, "-1: the digest leaves the registers"), ("a1", 0, "digest pointer")])
    for lane in range(SHORT):
        e.add("ld", CANON[lane], "a0", 8 * lane, c="A%d" % lane)
    for lane in range(SHORT, 25):
        if lane != RATE - 1:
            e.add("addi", CANON[lane], "zero", 0, c="A%d = 0" % lane)
    e.add("addi", CANON[16], "zero", -1, c="A16: the last padding bit")
    e.add("slli", CANON[16], CANON[16], 63)
    e.add("j", rounds)
    end(name)

    # the rounds and the three ways out
    e.add("raw", "")
    e.add("raw", "# The 24 rounds and the three ways out, shared by the entries.")
    e.add("raw", "    .type %s, @function" % rounds)
    e.add("raw", "    .p2align 2")
    e.add("label", rounds)
    L = Lanes()
    for r in range(23):
        e.add("raw", "    # round %d" % r)
        emit_round(e, L, r)
    t = L.free[0]
    e.add("ld", t, "sp", 8, c="blocks left, or -1 for a digest")
    e.add("bltz", t, lab("digest"))
    e.add("raw", "    # round 23")
    full = Lanes(L.reg)
    full.free = list(L.free)
    emit_round(e, full, 23)
    t = full.free[0]
    e.add("ld", t, "sp", 8, c="blocks left")
    e.add("beqz", t, lab("store"))
    moves = fixup(e, _copy(full))
    e.add("j", lab("block"))
    e.add("label", lab("store"))
    e.add("phase", "store the state")
    e.add("ld", t, "sp", 0, c="state pointer")
    for lane in COMPLEMENTED:
        e.add("xori", full.reg[lane], full.reg[lane], -1, c="A%d is stored complemented" % lane)
    for lane in range(25):
        e.add("sd", full.reg[lane], t, 8 * lane, c="A%d" % lane)
    frame_out(e)
    e.add("label", lab("digest"))
    e.add("raw", "    # round 23, lanes 0-3 only")
    dig = _copy(L)
    emit_round(e, dig, 23, digest=True)
    t = dig.free[0]
    e.add("ld", t, "sp", 0, c="digest pointer")
    for lane in range(4):
        e.add("sd", dig.reg[lane], t, 8 * lane, c="A%d" % lane)
    frame_out(e)
    e.add("raw", "    .size %s, . - %s" % (rounds, rounds))
    return e, moves


def _copy(L):
    c = Lanes(L.reg)
    c.reg = list(L.reg)
    c.free = list(L.free)
    return c


HEADER = """\
# Keccak-f[1600] for RV64 with binbender's Keccak opcodes (XRI, RAX1, BCAX, EOR3).
# GENERATED by keccak_f1600_rv64_ops_gen.py: do not edit, regenerate.
#
# The entries, their contracts, the frame and the lane-complemented representation in memory
# are those of keccak_f1600_rv64.S (see its header). The lanes are plain in the registers.
#
# REGISTERS. On entry to the rounds the lanes are in the registers of keccak_f1600_rv64.S;
# each instruction writes over an operand that dies and the lanes are renamed, never spilled,
# with five temporaries at most. The exits store from wherever the lanes are; the absorb loop
# moves them back before its next block.
#
# SAFETY: the contract of keccak_f1600_rv64.S. The routines follow the C calling convention:
# they clobber only caller-saved registers, use 144 bytes of stack below the entry `sp`, and
# read and write nothing else but the state, data, digest and round constants. They must not
# be interrupted by anything that expects `gp` or `tp` to hold their usual values (this guest
# has no interrupts and no threads).
"""


def text():
    e, _ = emit()
    lines = [HEADER, "    .option push", "    .option norvc", "    .option norelax", ""]
    lines.append('    .section .rodata.%s_ops_rc,"a",@progbits' % SYMBOL)
    lines.append("    .balign 8")
    lines.append(".L%s_ops_rc:" % SYMBOL)
    lines += ["    .dword 0x%016x  # round %d" % (c, i) for i, c in enumerate(RC)]
    lines += ["", '    .section .text.%s,"ax",@progbits' % SYMBOL]
    lines += e.render()
    lines += ["", "    .option pop", ""]
    return "\n".join(lines)


# ---------------------------------------------------------------------------------------------
# Emulation of the instruction list
# ---------------------------------------------------------------------------------------------

RC_BASE = 0x200000
RET = 0x7FFF0000


def run(e, entry, regs, mem):
    """Runs from label `entry`; returns (executed instructions, loads, stores, writes)."""
    ins = e.ins
    labels = {op[1]: i for i, op in enumerate(ins) if op[0] == "label"}
    pc = labels[entry]
    x = dict(regs)
    x["zero"] = 0
    counts = {"ins": 0, "ld": 0, "sd": 0, "spill": 0}
    writes = set()
    while True:
        op = ins[pc]
        name, a = op[0], op[1:-1]
        pc += 1
        if name in ("label", "raw", "phase"):
            continue
        counts["ins"] += 1
        g = lambda r: 0 if r == "zero" else x[r]

        def s(r, v):
            if r != "zero":
                x[r] = v & M64

        if name == "ld":
            addr = (g(a[1]) + a[2]) & M64
            assert addr % 8 == 0
            s(a[0], mem[addr])
            counts["ld"] += 1
        elif name == "sd":
            addr = (g(a[1]) + a[2]) & M64
            assert addr % 8 == 0
            mem[addr] = g(a[0])
            writes.add(addr)
            counts["sd"] += 1
        elif name == "rc":
            counts["ins"] += 1  # auipc and ld
            counts["ld"] += 1
            s(a[0], RC[a[1]])
        elif name == "xor":
            s(a[0], g(a[1]) ^ g(a[2]))
        elif name == "xori":
            s(a[0], g(a[1]) ^ (a[2] & M64))
        elif name == "addi":
            s(a[0], g(a[1]) + a[2])
        elif name == "slli":
            s(a[0], g(a[1]) << a[2])
        elif name == "xri":
            v = g(a[1]) ^ g(a[2])
            s(a[0], rol(v, 64 - a[3]))
        elif name == "rax1":
            s(a[0], g(a[1]) ^ rol(g(a[2]), 1))
        elif name == "bcax":
            s(a[0], g(a[3]) ^ (g(a[1]) & ~g(a[2]) & M64))
        elif name == "eor3":
            s(a[0], g(a[1]) ^ g(a[2]) ^ g(a[3]))
        elif name in ("beqz", "bnez", "bltz"):
            v = g(a[0])
            sv = v - (1 << 64) if v >> 63 else v
            if (name == "beqz" and v == 0) or (name == "bnez" and v) or (name == "bltz" and sv < 0):
                pc = labels[a[1]]
        elif name == "j":
            pc = labels[a[0]]
        elif name == "ret":
            assert x["ra"] == RET
            return counts, x, writes
        else:
            raise ValueError(name)


def call(e, entry, args, mem):
    rng = random.Random(len(mem))
    regs = {r: rng.getrandbits(64) for r in ABI if r not in ("zero",)}
    regs["sp"] = 0x7F000000
    regs["ra"] = RET
    for r, v in zip(("a0", "a1", "a2"), args):
        regs[r] = v & M64
    before = dict(regs)
    counts, after, writes = run(e, entry, regs, mem)
    for r in SAVED + ["sp"]:
        assert after[r] == before[r], r
    stack = {a for a in writes if 0x7F000000 - FRAME <= a < 0x7F000000}
    return counts, writes - stack


def check(trials=200):
    e, moves = emit()
    assert keccak_f1600([0] * 25) == FIPS_ZERO
    rng = random.Random(1)
    states = [[0] * 25, [M64] * 25] + [[rng.getrandbits(64) for _ in range(25)] for _ in range(trials)]
    S, D, DATA = 0x10000, 0x20000, 0x30000
    stored = lambda st: [w ^ complement_of(i) for i, w in enumerate(st)]
    for st in states:
        # permutation
        mem = {S + 8 * i: w for i, w in enumerate(stored(st))}
        _, writes = call(e, SYMBOL, [S], mem)
        assert [mem[S + 8 * i] for i in range(25)] == stored(keccak_f1600(st))
        assert writes == {S + 8 * i for i in range(25)}
        # digest
        mem = {S + 8 * i: w for i, w in enumerate(stored(st))}
        _, writes = call(e, SYMBOL + "_final", [S, D], mem)
        assert [mem[D + 8 * i] for i in range(4)] == keccak_f1600(st)[:4]
        assert writes == {D + 8 * i for i in range(4)}
        # absorb 1 to 3 blocks, from this state and from zero
        blocks = 1 + rng.randrange(3)
        data = [rng.getrandbits(64) for _ in range(RATE * blocks)]
        for first in (False, True):
            want = [0] * 25 if first else list(st)
            for b in range(blocks):
                for i in range(RATE):
                    want[i] ^= data[RATE * b + i]
                want = keccak_f1600(want)
            mem = {DATA + 8 * i: w for i, w in enumerate(data)}
            if not first:
                mem.update({S + 8 * i: w for i, w in enumerate(stored(st))})
            _, writes = call(e, SYMBOL + ("_absorb_first" if first else "_absorb"), [S, DATA, blocks], mem)
            assert [mem[S + 8 * i] for i in range(25)] == stored(want), (first, blocks)
            assert writes == {S + 8 * i for i in range(25)}
        # a short hash
        lanes = st[:SHORT]
        mem = {S + 8 * i: w for i, w in enumerate(lanes)}
        _, writes = call(e, SYMBOL + "_final64", [S, D], mem)
        full = lanes + [0] * (25 - SHORT)
        full[RATE - 1] ^= 1 << 63
        assert [mem[D + 8 * i] for i in range(4)] == keccak_f1600(full)[:4]
    # FIPS 202 zero-state vector through the assembly
    mem = {S + 8 * i: w for i, w in enumerate(stored([0] * 25))}
    call(e, SYMBOL, [S], mem)
    assert [mem[S + 8 * i] ^ complement_of(i) for i in range(25)] == FIPS_ZERO
    print("check: %d states, every entry, FIPS 202 zero-state vector: ok" % len(states))
    return moves


def counts():
    e, moves = emit()
    S, D, DATA = 0x10000, 0x20000, 0x30000
    rows = []
    mem = {S + 8 * i: 0 for i in range(25)}
    rows.append(("permutation", call(e, SYMBOL, [S], mem)[0]))
    mem = {S + 8 * i: 0 for i in range(25)}
    rows.append(("final", call(e, SYMBOL + "_final", [S, D], mem)[0]))
    mem = {S + 8 * i: 0 for i in range(SHORT)}
    rows.append(("final64", call(e, SYMBOL + "_final64", [S, D], mem)[0]))
    for blocks in (1, 2):
        mem = {DATA + 8 * i: 0 for i in range(RATE * blocks)}
        mem.update({S + 8 * i: 0 for i in range(25)})
        rows.append(("absorb x%d" % blocks, call(e, SYMBOL + "_absorb", [S, DATA, blocks], mem)[0]))
        rows.append(("absorb_first x%d" % blocks,
                     call(e, SYMBOL + "_absorb_first", [S, DATA, blocks], mem)[0]))
    body = sum(1 for op in e.ins if op[0] not in ("label", "raw", "phase"))
    print("%-18s %12s %6s %6s" % ("entry", "instructions", "loads", "stores"))
    for name, c in rows:
        print("%-18s %12d %6d %6d" % (name, c["ins"], c["ld"], c["sd"]))
    print("absorb loop: %d moves between blocks; %d instructions in the text" % (moves, body))


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--check", action="store_true")
    p.add_argument("--counts", action="store_true")
    args = p.parse_args()
    if args.check:
        check()
    elif args.counts:
        counts()
    else:
        check(20)
        with open(OUT, "w") as f:
            f.write(text())
        print("wrote", OUT)


if __name__ == "__main__":
    main()
