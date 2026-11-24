#!/usr/bin/env python3
"""Generator of `keccak_f1600_rv64.S`: Keccak-f[1600], 24 rounds unrolled, for RV64.

    python3 keccak_f1600_rv64_gen.py                 # rewrite keccak_f1600_rv64.S (variant rv64i)
    python3 keccak_f1600_rv64_gen.py --check         # emulate every variant against a reference
    python3 keccak_f1600_rv64_gen.py --counts        # instruction counts of every variant
    python3 keccak_f1600_rv64_gen.py --variant NAME  # print one variant to stdout
    python3 keccak_f1600_rv64_gen.py --search        # search the complemented lanes again (minutes)

Only the variant `rv64i` is compiled into the crate. The others exist to count what an opcode
would save: `zbb-rot` (with `rori`), `andn`, and `xor3` (a hypothetical `rd ^= rs1 ^ rs2`;
`xor3nd` is the form `rd = rs1 ^ rs2 ^ rs3`), alone and combined. They are printed, never
written next to the sources; the `xor3` ones cannot be assembled, only emulated here.

Every emitted line with an instruction is exactly one machine instruction (no pseudo-instruction
expands to two), so the counts are counts of the text. `--check` and `--counts` parse the emitted
text back: the emulator runs what the assembler will see, macros expanded, from each of the three
entries, and checks the state or digest, the preserved registers and where memory was written.

Design (see also the header of the emitted file):

* Every lane has one register for the whole permutation; the 25 lanes take 25 of the 30 general
  registers (all but `zero` and `sp`), which leaves five temporaries (`a0`-`a4`).
* Theta applies each column's D as soon as it exists, so the five column parities and the five
  D never need more than the temporaries: no lane is spilled. With `rori` a D needs no scratch
  register, four temporaries are enough, and the fifth register holds the round constants.
* Rho and pi walk the single 24-cycle of pi backwards: a rotated lane is written straight into
  the register of its destination, which the walk has just vacated. No moves.
* Chi works row by row in place with three temporaries. Without `andn` the lane-complementing
  transform keeps some lanes complemented in their registers *and in memory* (the caller holds the
  state in that representation), which leaves one NOT per row.
* Iota loads the round constant PC-relative (or through the table register), or uses `xori` when
  the constant fits.
* Three entries share the rounds: state to state, the same after absorbing whole blocks (a loop
  around the rounds that keeps the lanes in registers), and the last permutation of a hash, which
  stores the four lanes of the digest only.
* The shipped variant has two more entries, for a state that no permutation has touched (the
  caller initialises no lane of it): whole blocks absorbed from the zero state, the first one
  loaded straight from the input; and a whole hash of at most 64 bytes from its nine lanes,
  whose round 0 knows the fifteen zero lanes. Its last round of a hash computes the digest
  lanes only: a third of a round.
"""
import argparse
import os
import random
import re
import sys

M64 = (1 << 64) - 1

# ---------------------------------------------------------------------------------------------
# Keccak-f[1600] from its definition (FIPS 202, section 3.2), and a check of it against hashlib
# ---------------------------------------------------------------------------------------------


def rol(value, amount):
    amount %= 64
    return ((value << amount) | (value >> (64 - amount))) & M64 if amount else value


def rho_offsets():
    offsets = [0] * 25
    x, y = 1, 0
    for t in range(24):
        offsets[x + 5 * y] = ((t + 1) * (t + 2) // 2) % 64
        x, y = y, (2 * x + 3 * y) % 5
    return offsets


def round_constants():
    constants = []
    lfsr = 1
    for _ in range(24):
        constant = 0
        for j in range(7):
            if lfsr & 1:
                constant ^= 1 << ((1 << j) - 1)
            lfsr = ((lfsr << 1) ^ 0x71) & 0xFF if lfsr & 0x80 else lfsr << 1
        constants.append(constant)
    return constants


RHO = rho_offsets()
RC = round_constants()
assert RC[0] == 1 and RC[1] == 0x8082 and RC[23] == 0x8000000080008008


def pi_dest(lane):
    """Index of the lane that receives lane `lane` (x, y) -> (y, 2x + 3y)."""
    x, y = lane % 5, lane // 5
    return y + 5 * ((2 * x + 3 * y) % 5)


PI_SRC = [0] * 25
for _lane in range(25):
    PI_SRC[pi_dest(_lane)] = _lane


def keccak_f1600(state):
    a = list(state)
    for constant in RC:
        c = [a[x] ^ a[x + 5] ^ a[x + 10] ^ a[x + 15] ^ a[x + 20] for x in range(5)]
        d = [c[(x + 4) % 5] ^ rol(c[(x + 1) % 5], 1) for x in range(5)]
        a = [a[i] ^ d[i % 5] for i in range(25)]
        b = [0] * 25
        for i in range(25):
            b[pi_dest(i)] = rol(a[i], RHO[i])
        a = [
            b[i] ^ (~b[5 * (i // 5) + (i % 5 + 1) % 5] & b[5 * (i // 5) + (i % 5 + 2) % 5] & M64)
            for i in range(25)
        ]
        a[0] ^= constant
    return a


def check_reference():
    import hashlib

    for message in (b"", b"abc", bytes(range(200)) * 3):
        state = [0] * 25
        padded = bytearray(message) + b"\x06"
        padded += bytes(-len(padded) % 136)
        padded[-1] |= 0x80
        for start in range(0, len(padded), 136):
            for k in range(17):
                state[k] ^= int.from_bytes(padded[start + 8 * k : start + 8 * k + 8], "little")
            state = keccak_f1600(state)
        digest = b"".join(lane.to_bytes(8, "little") for lane in state[:4])
        assert digest == hashlib.sha3_256(message).digest()


# ---------------------------------------------------------------------------------------------
# The lane-complementing transform
# ---------------------------------------------------------------------------------------------
#
# A set P of lanes is held complemented at every round boundary. Theta is linear: with k[x] the
# parity of P in column x, the computed D[x] comes out complemented when k[x-1] ^ k[x+1] = 1, and
# then flips the whole column. Rho and pi move the lanes with their polarity. Chi must then turn
# a row with input polarities `m` into a row with output polarities `o` (the row of P):
#
#     A[i] = B[i] ^ (~B[i+1] & B[i+2])
#
# With AND, OR and XOR only, a term whose two operands have the same polarity needs one NOT; five
# operands around a cycle cannot alternate, so a row needs at least one NOT, a round five. P below
# reaches five, so no other choice of P can do better.


def row_solutions(m, o, free_not_on_0, wanted=range(5)):
    """Cheapest ways of computing a chi row. `m`, `o`: 5-bit polarity masks (bit = complemented).
    Only the outputs in `wanted` are computed; the others are None in a solution.

    A solution is (cost, negated, outputs): `negated` is the set (bit mask) of inputs of which a
    complement is made, `outputs[i]` = (px, op, p1, p2, out_not) with the stored
    A[i] = (B[i] ^ px) ^ ((B[i+1] ^ p1) op (B[i+2] ^ p2)) ^ out_not over the stored inputs.
    The NOT of output 0 is free in row 0: iota absorbs it into the round constant.
    """
    best_cost, best = None, []
    for negated in range(32):
        cost = bin(negated).count("1")
        outputs = []
        for i in range(5):
            j1, j2 = (i + 1) % 5, (i + 2) % 5
            candidates = []
            if i not in wanted:
                outputs.append(None)
                continue
            for px in (0, 1):
                for p1 in (0, 1):
                    for p2 in (0, 1):
                        if (px and not negated >> i & 1) or (p1 and not negated >> j1 & 1):
                            continue
                        if p2 and not negated >> j2 & 1:
                            continue
                        for op in ("and", "or"):
                            for out_not in (0, 1):
                                good = True
                                for bits in range(8):
                                    b0, b1, b2 = bits & 1, bits >> 1 & 1, bits >> 2 & 1
                                    u, v = b1 ^ p1, b2 ^ p2
                                    got = (b0 ^ px) ^ ((u & v) if op == "and" else (u | v)) ^ out_not
                                    t0, t1, t2 = b0 ^ (m >> i & 1), b1 ^ (m >> j1 & 1), b2 ^ (m >> j2 & 1)
                                    want = t0 ^ ((1 ^ t1) & t2) ^ (o >> i & 1)
                                    good &= got == want
                                if good:
                                    paid = out_not and not (free_not_on_0 and i == 0)
                                    candidates.append((int(paid), px + p1 + p2, (px, op, p1, p2, out_not)))
            if not candidates:
                break
            candidates.sort()
            cost += candidates[0][0]
            outputs.append(candidates[0][2])
        else:
            if best_cost is None or cost < best_cost:
                best_cost, best = cost, []
            if cost == best_cost:
                best.append((cost, negated, outputs))
    return best


def theta_flips(mask):
    parity = [sum(mask >> (x + 5 * y) & 1 for y in range(5)) & 1 for x in range(5)]
    return [parity[(x + 4) % 5] ^ parity[(x + 1) % 5] for x in range(5)]


def chi_input_mask(mask):
    """Polarities of the 25 lanes in front of chi, given the polarities `mask` at round entry."""
    flips = theta_flips(mask)
    result = 0
    for lane in range(25):
        if (mask >> lane & 1) ^ flips[lane % 5]:
            result |= 1 << pi_dest(lane)
    return result


_ROW_COST = {}


def row_cost(m, o, row):
    key = (m, o, row == 0)
    if key not in _ROW_COST:
        _ROW_COST[key] = row_solutions(m, o, row == 0)[0][0]
    return _ROW_COST[key]


def round_cost(mask):
    before_chi = chi_input_mask(mask)
    return sum(row_cost(before_chi >> 5 * y & 31, mask >> 5 * y & 31, y) for y in range(5))


def search_masks():
    """Every P with five NOTs per round (all 2^25 sets: minutes of pure Python)."""
    return [mask for mask in range(1 << 25) if round_cost(mask) == 5]


# Lanes 3, 7, 8, 12, 17 and 23. `--search` finds nine sets with five NOTs per round, among them
# {1, 2, 8, 12, 17, 20} of the Keccak implementation overview. None leaves all of the lanes 0-3
# (the 256-bit digest) true; this one complements only lane 3, with six lanes in all, so the
# caller fixes one word of the digest.
COMPLEMENTED = sum(1 << lane for lane in (3, 7, 8, 12, 17, 23))

# With the hypothetical `xor3` (three inputs xored in one instruction) chi needs no NOT at all:
#
#     ~b & c  =  c ^ (b & c)  =  b ^ (b | c)
#
# so A[i] = B[i] ^ (B[i+1] op B[i+2]) ^ E with E one of the two operands, an AND or OR and one
# `xor3`. When the operands of a term are stored with different polarities the term needs no E,
# and the second input of the `xor3` is free: on lane 0 it takes the round constant, which
# merges iota's xor. A set of complemented lanes is then worth having only for that.


def xor3_row_forms(m, o, row):
    """Per output of a chi row, the cheapest (cost, op, extra, out_not) with `xor3`.

    The stored A[i] = B[i] ^ (B[i+1] op B[i+2]) ^ (B[extra] if extra is not None) ^ out_not.
    `extra` is the row index of the operand xored in, restricted for output 4 to index 0 (see
    `emit_chi_xor3`). cost counts a paid NOT, and on lane 0 a round constant that finds the
    second input of the `xor3` taken.
    """
    forms = []
    for i in range(5):
        j1, j2 = (i + 1) % 5, (i + 2) % 5
        candidates = []
        for op in ("and", "or"):
            for extra in (None, j1, j2):
                if i == 4 and extra == j2:
                    continue
                for out_not in (0, 1):
                    good = True
                    for bits in range(8):
                        b0, b1, b2 = bits & 1, bits >> 1 & 1, bits >> 2 & 1
                        term = (b1 & b2) if op == "and" else (b1 | b2)
                        got = b0 ^ term ^ (0 if extra is None else (b1 if extra == j1 else b2)) ^ out_not
                        t0, t1, t2 = b0 ^ (m >> i & 1), b1 ^ (m >> j1 & 1), b2 ^ (m >> j2 & 1)
                        good &= got == t0 ^ ((1 ^ t1) & t2) ^ (o >> i & 1)
                    if good:
                        on_lane_0 = row == 0 and i == 0
                        cost = (out_not and not on_lane_0) + (on_lane_0 and extra is not None)
                        candidates.append((int(cost), extra is not None, op, extra, out_not))
        candidates.sort(key=lambda c: (c[0], c[1], c[2]))
        cost, _, op, extra, out_not = candidates[0]
        forms.append((cost, op, extra, out_not))
    return forms


_XOR3_ROW_COST = {}


def xor3_round_cost(mask):
    """Instructions of chi and iota's xor beyond the 50 of five AND/OR and five `xor3` per row."""
    before_chi = chi_input_mask(mask)
    total = 0
    for y in range(5):
        key = (before_chi >> 5 * y & 31, mask >> 5 * y & 31, y == 0)
        if key not in _XOR3_ROW_COST:
            _XOR3_ROW_COST[key] = sum(form[0] for form in xor3_row_forms(key[0], key[1], y))
        total += _XOR3_ROW_COST[key]
    return total


# ---------------------------------------------------------------------------------------------
# Emission
# ---------------------------------------------------------------------------------------------

VARIANTS = {
    # name: (rotation opcode, and-not opcode, three-input xor)
    "rv64i": (False, False, None),
    "zbb-rot": (True, False, None),
    "andn": (False, True, None),
    "zbb-rot+andn": (True, True, None),
    # `xor3 rd, rs1, rs2`: rd ^= rs1 ^ rs2, a hypothetical opcode
    "xor3": (False, False, "rd ^= rs1 ^ rs2"),
    "zbb-rot+xor3": (True, False, "rd ^= rs1 ^ rs2"),
    "andn+xor3": (False, True, "rd ^= rs1 ^ rs2"),
    "zbb-rot+andn+xor3": (True, True, "rd ^= rs1 ^ rs2"),
    # `xor3 rd, rs1, rs2, rs3`: rd = rs1 ^ rs2 ^ rs3, the same with a destination of its own
    "xor3nd": (False, False, "rd = rs1 ^ rs2 ^ rs3"),
    "zbb-rot+xor3nd": (True, False, "rd = rs1 ^ rs2 ^ rs3"),
    "andn+xor3nd": (False, True, "rd = rs1 ^ rs2 ^ rs3"),
    "zbb-rot+andn+xor3nd": (True, True, "rd = rs1 ^ rs2 ^ rs3"),
}

LANE_REGS = (
    "t0 t1 t2 t3 t4 "  # lanes 0-4
    "t5 t6 a5 a6 a7 "  # lanes 5-9
    "s0 s1 s2 s3 s4 "  # lanes 10-14
    "s5 s6 s7 s8 s9 "  # lanes 15-19
    "s10 s11 ra gp tp"  # lanes 20-24
).split()
SAVED = ["ra", "gp", "tp"] + ["s%d" % i for i in range(12)]
FRAME = 144  # state pointer, blocks left, data pointer, 15 saved registers
SYMBOL = "airbender_keccak_f1600_rv64"


class Emitter:
    def __init__(self):
        self.lines = []

    def raw(self, text=""):
        self.lines.append(text)

    def ins(self, text, comment=""):
        self.lines.append("    %-33s # %s" % (text, comment) if comment else "    " + text)

    def phase(self, name):
        self.lines.append("    # -- %s --" % name)

    def text(self):
        return "\n".join(self.lines) + "\n"


def polarity(mask, lane):
    return "~" if mask >> lane & 1 else ""


def emit_parity(e, dst, x, L):
    e.ins("xor %s, %s, %s" % (dst, L[x], L[x + 5]), "C%d = A%d ^ A%d" % (x, x, x + 5))
    for y in (2, 3, 4):
        e.ins("xor %s, %s, %s" % (dst, dst, L[x + 5 * y]), "C%d ^= A%d" % (x, x + 5 * y))


def emit_apply(e, reg, x, L, skip=None):
    for y in range(5):
        lane = x + 5 * y
        if lane != skip:
            e.ins("xor %s, %s, %s" % (L[lane], L[lane], reg), "A%d ^= D%d" % (lane, x))


def emit_theta_shifts(e, L, T):
    """Theta with slli/srli: 20 + 20 + 25 instructions, five temporaries, nothing spilled.

    D[x] = C[x-1] ^ rotl(C[x+1], 1). Each D is applied to its column as soon as it exists and
    C[x] has been read off that column, which frees its register for the next parity.
    """
    T0, T1, T2, T3, T4 = T
    emit_parity(e, T0, 0, L)
    emit_parity(e, T1, 2, L)
    e.ins("slli %s, %s, 1" % (T2, T1), "D1 = C0 ^ rotl(C2, 1)")
    e.ins("srli %s, %s, 63" % (T3, T1))
    e.ins("xor %s, %s, %s" % (T2, T2, T3))
    e.ins("xor %s, %s, %s" % (T2, T2, T0))
    emit_parity(e, T3, 4, L)
    e.ins("slli %s, %s, 1" % (T4, T3), "D3 = C2 ^ rotl(C4, 1), over C2")
    e.ins("xor %s, %s, %s" % (T1, T1, T4))
    e.ins("srli %s, %s, 63" % (T4, T3))
    e.ins("xor %s, %s, %s" % (T1, T1, T4))
    emit_parity(e, T4, 1, L)
    emit_apply(e, T2, 1, L)
    e.ins("slli %s, %s, 1" % (T2, T4), "D0 = C4 ^ rotl(C1, 1), over C4")
    e.ins("xor %s, %s, %s" % (T3, T3, T2))
    e.ins("srli %s, %s, 63" % (T2, T4))
    e.ins("xor %s, %s, %s" % (T3, T3, T2))
    emit_apply(e, T3, 0, L)
    emit_parity(e, T2, 3, L)
    emit_apply(e, T1, 3, L)
    e.ins("srli %s, %s, 63" % (T1, T0), "D4 = C3 ^ rotl(C0, 1), over C0")
    e.ins("slli %s, %s, 1" % (T0, T0))
    e.ins("xor %s, %s, %s" % (T0, T0, T1))
    e.ins("xor %s, %s, %s" % (T0, T0, T2))
    emit_apply(e, T0, 4, L)
    e.ins("srli %s, %s, 63" % (T1, T2), "D2 = C1 ^ rotl(C3, 1), over C1")
    e.ins("slli %s, %s, 1" % (T2, T2))
    e.ins("xor %s, %s, %s" % (T4, T4, T1))
    e.ins("xor %s, %s, %s" % (T4, T4, T2))
    emit_apply(e, T4, 2, L)


RORI_FIRST = 20  # the lane whose theta xor lands in a temporary and starts the walk of pi


def emit_theta_rori(e, L, T):
    """Theta with rori: 20 + 10 + 25 instructions and four temporaries.

    The last xor of D0 writes lane 20 into the temporary that held D0, which vacates the
    register of lane 20 for the walk of pi.
    """
    T0, T1, T2, T3 = T
    emit_parity(e, T0, 1, L)
    emit_parity(e, T1, 0, L)
    emit_parity(e, T2, 2, L)
    e.ins("rori %s, %s, 63" % (T3, T2), "D1 = C0 ^ rotl(C2, 1)")
    e.ins("xor %s, %s, %s" % (T3, T3, T1))
    emit_apply(e, T3, 1, L)
    emit_parity(e, T3, 3, L)
    e.ins("rori %s, %s, 63" % (T1, T1), "D4 = C3 ^ rotl(C0, 1), over C0")
    e.ins("xor %s, %s, %s" % (T1, T1, T3))
    e.ins("rori %s, %s, 63" % (T3, T3), "D2 = C1 ^ rotl(C3, 1), over C3")
    e.ins("xor %s, %s, %s" % (T3, T3, T0))
    emit_apply(e, T3, 2, L)
    emit_parity(e, T3, 4, L)
    emit_apply(e, T1, 4, L)
    e.ins("rori %s, %s, 63" % (T1, T3), "D3 = C2 ^ rotl(C4, 1), over C2")
    e.ins("xor %s, %s, %s" % (T2, T2, T1))
    emit_apply(e, T2, 3, L)
    e.ins("rori %s, %s, 63" % (T0, T0), "D0 = C4 ^ rotl(C1, 1), over C1")
    e.ins("xor %s, %s, %s" % (T0, T0, T3))
    emit_apply(e, T0, 0, L, skip=RORI_FIRST)
    e.ins(
        "xor %s, %s, %s" % (T0, L[RORI_FIRST], T0),
        "A%d ^ D0, into the temporary: %s is vacant" % (RORI_FIRST, L[RORI_FIRST]),
    )


def pi_walk(first):
    """Lanes in the order the walk reads them: `first`, then each time the lane that pi sends
    into the register just vacated. Lane 0 does not move and is not on the 24-cycle."""
    order = [first]
    while PI_SRC[order[-1]] != first:
        order.append(PI_SRC[order[-1]])
    assert len(order) == 24 and 0 not in order
    return order


def emit_rho_pi_shifts(e, L, T):
    first = 6
    order = pi_walk(first)
    r = RHO[first]
    e.ins("slli %s, %s, %d" % (T[0], L[first], r), "rotl(A%d, %d) waits in two halves" % (first, r))
    e.ins("srli %s, %s, %d" % (T[1], L[first], 64 - r), "%s is vacant" % L[first])
    for src in order[1:]:
        dst, r = pi_dest(src), RHO[src]
        e.ins("slli %s, %s, %d" % (L[dst], L[src], r), "B%d = rotl(A%d, %d)" % (dst, src, r))
        e.ins("srli %s, %s, %d" % (L[src], L[src], 64 - r))
        e.ins("or %s, %s, %s" % (L[dst], L[dst], L[src]))
    dst = pi_dest(first)
    e.ins("or %s, %s, %s" % (L[dst], T[0], T[1]), "B%d = rotl(A%d, %d)" % (dst, first, RHO[first]))


def emit_rho_pi_rori(e, L, T, first=RORI_FIRST, holder=None):
    """`holder` (the first temporary by default) holds lane `first` after theta."""
    order = pi_walk(first)
    for src in order[1:]:
        dst, r = pi_dest(src), RHO[src]
        e.ins("rori %s, %s, %d" % (L[dst], L[src], 64 - r), "B%d = rotl(A%d, %d)" % (dst, src, r))
    dst, r = pi_dest(first), RHO[first]
    e.ins("rori %s, %s, %d" % (L[dst], holder or T[0], 64 - r), "B%d = rotl(A%d, %d)" % (dst, first, r))


def emit_chi_andn(e, L, T):
    X, Y, Z = T[:3]
    for y in range(5):
        R = L[5 * y : 5 * y + 5]
        n = [5 * y + i for i in range(5)]
        e.ins("andn %s, %s, %s" % (X, R[0], R[4]), "~B%d & B%d" % (n[4], n[0]))
        e.ins("andn %s, %s, %s" % (Y, R[1], R[0]), "~B%d & B%d" % (n[0], n[1]))
        for i in range(3):
            e.ins("andn %s, %s, %s" % (Z, R[i + 2], R[i + 1]), "~B%d & B%d" % (n[i + 1], n[i + 2]))
            e.ins("xor %s, %s, %s" % (R[i], R[i], Z), "A%d" % n[i])
        e.ins("xor %s, %s, %s" % (R[3], R[3], X), "A%d" % n[3])
        e.ins("xor %s, %s, %s" % (R[4], R[4], Y), "A%d" % n[4])


def chi_row_program(y, solution, R, temps):
    """Instructions of one chi row in place, or None when it needs more than `temps`.

    Order: the terms of A3 and A4 first (they read B0 and B1, which A0 and A1 overwrite), then
    A0, A1, A2 with one more temporary, then A3 and A4. A complement of an input lives in a
    temporary from its first use to its last.
    """
    _, negated, outputs = solution
    n = [5 * y + i for i in range(5)]
    free = list(temps)
    program = []
    # per input, the steps that read its complement
    uses = {j: [] for j in range(5)}
    steps = [("term", 3), ("term", 4), ("out", 0), ("out", 1), ("out", 2), ("xor", 3), ("xor", 4)]
    for index, (kind, i) in enumerate(steps):
        px, _, p1, p2, _ = outputs[i]
        if kind in ("term", "out"):
            if p1:
                uses[(i + 1) % 5].append(index)
            if p2:
                uses[(i + 2) % 5].append(index)
        if kind in ("out", "xor") and px:
            uses[i].append(index)
    complement = {}
    texts = {}

    def operand(j, negative, index):
        if not negative:
            return R[j], "b%d" % n[j]
        if j not in complement:
            if not free:
                raise LookupError
            complement[j] = free.pop(0)
            program.append(("xori %s, %s, -1" % (complement[j], R[j]), "~b%d" % n[j]))
        return complement[j], "~b%d" % n[j]

    def release(index):
        for j in list(complement):
            if uses[j][-1] <= index:
                free.insert(0, complement.pop(j))

    def term(i, dst, index):
        _, op, p1, p2, _ = outputs[i]
        r1, s1 = operand((i + 1) % 5, p1, index)
        r2, s2 = operand((i + 2) % 5, p2, index)
        text = "%s %s %s" % (s1, "&" if op == "and" else "|", s2)
        program.append(("%s %s, %s, %s" % (op, dst, r1, r2), text))
        texts[i] = text

    def output(i, term_reg, index):
        px, _, _, _, out_not = outputs[i]
        r, s = operand(i, px, index)
        program.append(("xor %s, %s, %s" % (R[i], r, term_reg), "a%d = %s ^ (%s)" % (n[i], s, texts[i])))
        if out_not and not (y == 0 and i == 0):
            program.append(("xori %s, %s, -1" % (R[i], R[i]), "a%d = ~a%d" % (n[i], n[i])))

    try:
        if len(free) < 3:
            raise LookupError
        held = {}
        for index, (kind, i) in enumerate(steps):
            if kind == "term":
                held[i] = free.pop(0)
                term(i, held[i], index)
            elif kind == "out":
                scratch = free.pop(0)
                term(i, scratch, index)
                output(i, scratch, index)
                free.insert(0, scratch)
            else:
                output(i, held[i], index)
                free.insert(0, held.pop(i))
            release(index)
    except LookupError:
        return None
    return program


def emit_chi_complemented(e, L, T):
    """Chi over the complemented representation. Returns whether iota must complement lane 0."""
    before = chi_input_mask(COMPLEMENTED)
    absorbed = False
    for y in range(5):
        m, o = before >> 5 * y & 31, COMPLEMENTED >> 5 * y & 31
        programs = []
        for solution in row_solutions(m, o, y == 0):
            program = chi_row_program(y, solution, L[5 * y : 5 * y + 5], T)
            if program is not None:
                programs.append((len(program), solution[1], program, solution))
        assert programs, "chi row %d does not fit the temporaries" % y
        programs.sort(key=lambda item: item[:2])
        _, _, program, solution = programs[0]
        if y == 0 and solution[2][0][4]:
            absorbed = True
        stored = lambda mask: " ".join(
            "%s%s%d" % ("~" if mask >> i & 1 else "", "%s", 5 * y + i) for i in range(5)
        )
        e.ins("", "")
        e.lines[-1] = "    # row %d: b = %s; a = %s" % (
            y,
            stored(m) % (("B",) * 5),
            stored(o) % (("A",) * 5),
        )
        for text, comment in program:
            e.ins(text, comment)
    return absorbed


# The two specialisations of the shipped variant for short and for finished hashes.


def emit_digest_round(e, L, T, mask):
    """The last round of a hash: only lanes 0-3 of its result, the digest, are wanted.

    They are four outputs of row 0 of chi, which reads the five lanes that pi sends there
    (0, 6, 12, 18, 24): theta is applied to those five lanes and four of them are rotated.
    The parities still read all 25 lanes, so nothing of the round before is dead. Every other
    register is free here, so nothing is in place. Returns the registers of the four lanes,
    plain, and whether iota must complement lane 0.
    """
    row = [PI_SRC[j] for j in range(5)]
    e.phase("last theta")
    for x in range(5):
        emit_parity(e, T[x], x, L)
    pool = [L[i] for i in range(25) if i not in row]
    S, U = pool.pop(), pool.pop()
    for lane in row:
        x = lane % 5
        cn, cp = T[(x + 1) % 5], T[(x + 4) % 5]
        e.ins("slli %s, %s, 1" % (S, cn), "A%d ^= C%d ^ rotl(C%d, 1)" % (lane, (x + 4) % 5, (x + 1) % 5))
        e.ins("srli %s, %s, 63" % (U, cn))
        e.ins("xor %s, %s, %s" % (L[lane], L[lane], S))
        e.ins("xor %s, %s, %s" % (L[lane], L[lane], U))
        e.ins("xor %s, %s, %s" % (L[lane], L[lane], cp))
    e.phase("last rho-pi")
    for j, lane in enumerate(row):
        if RHO[lane]:
            e.ins("slli %s, %s, %d" % (S, L[lane], RHO[lane]), "B%d = rotl(A%d, %d), in the register of A%d" % (j, lane, RHO[lane], lane))
            e.ins("srli %s, %s, %d" % (L[lane], L[lane], 64 - RHO[lane]))
            e.ins("or %s, %s, %s" % (L[lane], L[lane], S))
    e.phase("last chi")
    m = chi_input_mask(mask) & 31
    solutions = row_solutions(m, 0, True, wanted=range(4))
    solutions.sort(key=lambda solution: (solution[0], bin(solution[1]).count("1"), solution[1]))
    _, negated, outputs = solutions[0]
    R = [L[lane] for lane in row]
    e.lines.append("    # b = %s; the digest lanes leave plain" % " ".join(
        "%sB%d" % ("~" if m >> i & 1 else "", i) for i in range(5)))
    complement = {}
    for j in range(5):
        if negated >> j & 1:
            complement[j] = pool.pop()
            e.ins("xori %s, %s, -1" % (complement[j], R[j]), "~b%d" % j)
    digest = []
    for i in range(4):
        px, op, p1, p2, out_not = outputs[i]
        j1, j2 = i + 1, (i + 2) % 5
        dst = pool.pop()
        e.ins("%s %s, %s, %s" % (op, dst, complement[j1] if p1 else R[j1], complement[j2] if p2 else R[j2]),
              "%sb%d %s %sb%d" % ("~" if p1 else "", j1, "&" if op == "and" else "|", "~" if p2 else "", j2))
        e.ins("xor %s, %s, %s" % (dst, complement[i] if px else R[i], dst), "A%d" % i)
        if out_not and i:
            e.ins("xori %s, %s, -1" % (dst, dst), "A%d = ~A%d" % (i, i))
        digest.append(dst)
    return digest, bool(outputs[0][4]), U


def emit_round0_short(e, L, T, mask, data, rotation=0):
    """Theta, rho and pi of round 0 when only the lanes `data` are in registers, plain, and
    every other lane is zero.

    A column parity has only the data lanes of the column; theta makes a zero lane a copy of
    D, so its rotation reads D and the lane costs no xor. The lanes must leave in the
    representation of the rounds (those of `mask` complemented, and theta flips columns, see
    above): a lane needs D in the polarity `mask bit ^ flip of the column`, so some D exist in
    both. Raises LookupError when the walk of pi finds no free destination; the caller then
    tries another `rotation` of the registers used for the second polarities.
    """
    zero = [i for i in range(25) if i not in data]
    flips = theta_flips(mask)
    wanted = lambda lane: (mask >> lane & 1) ^ flips[lane % 5]
    need = [set(wanted(lane) for lane in range(x, 25, 5)) for x in range(5)]
    columns = [[i for i in data if i % 5 == x] for x in range(5)]
    assert all(columns)
    # which parities to complement, so that the fewest D need a NOT afterwards
    best = None
    for subset in range(32):
        pol = [(subset >> ((x + 4) % 5) & 1) ^ (subset >> ((x + 1) % 5) & 1) for x in range(5)]
        cost = bin(subset).count("1") + sum(need[x] != {pol[x]} for x in range(5))
        if best is None or cost < best[0]:
            best = (cost, subset, pol)
    _, subset, pol = best
    spare = [L[i] for i in zero]
    spare = spare[rotation:] + spare[:rotation]
    C, borrowed = {}, []
    for x in range(5):
        lanes = columns[x]
        names = " ^ ".join("A%d" % i for i in lanes)
        if len(lanes) == 1 and not subset >> x & 1:
            C[x] = L[lanes[0]]
            continue
        C[x] = spare.pop()
        borrowed.append(C[x])
        if len(lanes) == 1:
            e.ins("xori %s, %s, -1" % (C[x], L[lanes[0]]), "~C%d = ~A%d" % (x, lanes[0]))
            continue
        e.ins("xor %s, %s, %s" % (C[x], L[lanes[0]], L[lanes[1]]), "C%d = %s" % (x, names))
        for lane in lanes[2:]:
            e.ins("xor %s, %s, %s" % (C[x], C[x], L[lane]))
        if subset >> x & 1:
            e.ins("xori %s, %s, -1" % (C[x], C[x]), "~C%d" % x)
    U = spare.pop()
    D = {}
    for x in range(5):
        cn, cp = C[(x + 1) % 5], C[(x + 4) % 5]
        e.ins("slli %s, %s, 1" % (T[x], cn), "%sD%d = C%d ^ rotl(C%d, 1)" % ("~" if pol[x] else "", x, (x + 4) % 5, (x + 1) % 5))
        e.ins("srli %s, %s, 63" % (U, cn))
        e.ins("xor %s, %s, %s" % (T[x], T[x], U))
        e.ins("xor %s, %s, %s" % (T[x], T[x], cp))
        D[(x, pol[x])] = T[x]
    spare = borrowed + [U] + spare
    for x in range(5):
        other = 1 ^ pol[x]
        if need[x] == {other}:
            e.ins("xori %s, %s, -1" % (T[x], T[x]), "%sD%d" % ("~" if other else "", x))
            D[(x, other)] = D.pop((x, pol[x]))
        elif len(need[x]) == 2:
            D[(x, other)] = spare.pop()
            e.ins("xori %s, %s, -1" % (D[(x, other)], T[x]), "%sD%d as well" % ("~" if other else "", x))
    for lane in data:
        e.ins("xor %s, %s, %s" % (L[lane], L[lane], D[(lane % 5, wanted(lane))]), "A%d ^= D%d" % (lane, lane % 5))
    e.phase("short rho-pi")
    source = {lane: D[(lane % 5, wanted(lane))] for lane in zero}
    occupied = set(L[i] for i in data) | set(D.values())
    pending = [lane for lane in pi_walk(6) if True]
    while pending:
        progress = False
        for src in list(pending):
            dst, r = pi_dest(src), RHO[src]
            if L[dst] in occupied:
                continue
            if src in data:
                e.ins("slli %s, %s, %d" % (L[dst], L[src], r), "B%d = rotl(A%d, %d)" % (dst, src, r))
                e.ins("srli %s, %s, %d" % (L[src], L[src], 64 - r))
                e.ins("or %s, %s, %s" % (L[dst], L[dst], L[src]))
                occupied.discard(L[src])
            else:
                scratch = [reg for reg in list(T) + L if reg not in occupied and reg != L[dst]][0]
                e.ins("slli %s, %s, %d" % (L[dst], source[src], r), "B%d = rotl(D%d, %d): A%d was zero" % (dst, src % 5, r, src))
                e.ins("srli %s, %s, %d" % (scratch, source[src], 64 - r))
                e.ins("or %s, %s, %s" % (L[dst], L[dst], scratch))
                del source[src]
                if D[(src % 5, wanted(src))] not in source.values():
                    occupied.discard(D[(src % 5, wanted(src))])
            occupied.add(L[dst])
            pending.remove(src)
            progress = True
        if not progress:
            raise LookupError


# The hypothetical three-input xor. Where it pays:
#
# * a column parity (five inputs) is `xor`, `xor3`, `xor` (two `xor3` in the form with its own
#   destination) instead of four `xor`;
# * D = C[x-1] ^ rotl(C[x+1], 1) is two shifts and one `xor3`; with `rori` no D is formed at
#   all, each lane takes C[x-1] and the rotated C[x+1] in one `xor3`;
# * chi needs no NOT (see `xor3_row_forms`), so the state is plain; without `andn` one lane per
#   row is written to a free register instead of in place (the old value is still read), and
#   the five lanes concerned alternate between two registers from round to round;
# * iota: with `andn` the second input of lane 0's `xor3` in chi takes the round constant.
#   Without it both inputs are taken; the form `rd ^= rs1 ^ rs2` then has two free inputs in
#   the next theta (the third instruction of the parity of column 0, and the xor of D0 into
#   lane 0), which take the constant instead: iota is then only the load.


def emit_x3(e, nd, rd, a, b, comment=""):
    e.ins(("xor3 %s, %s, %s, %s" % (rd, rd, a, b)) if nd else ("xor3 %s, %s, %s" % (rd, a, b)), comment)


def emit_parity_x3(e, nd, dst, x, L, owed=None):
    """C[x] into `dst`; `owed` is a register with the round constant that lane 0 still owes."""
    r = [L[x + 5 * y] for y in range(5)]
    names = tuple("A%d" % (x + 5 * y) for y in range(5))
    if nd:
        assert owed is None
        e.ins("xor3 %s, %s, %s, %s" % (dst, r[0], r[1], r[2]), "C%d = %s ^ %s ^ %s" % ((x,) + names[:3]))
        e.ins("xor3 %s, %s, %s, %s" % (dst, dst, r[3], r[4]), "C%d ^= %s ^ %s" % ((x,) + names[3:]))
        return
    e.ins("xor %s, %s, %s" % (dst, r[0], r[1]), "C%d = %s ^ %s" % ((x,) + names[:2]))
    emit_x3(e, nd, dst, r[2], r[3], "C%d ^= %s ^ %s" % ((x,) + names[2:4]))
    if owed:
        emit_x3(e, nd, dst, r[4], owed, "C%d ^= %s ^ the constant owed to A0" % (x, names[4]))
    else:
        e.ins("xor %s, %s, %s" % (dst, dst, r[4]), "C%d ^= %s" % (x, names[4]))


def emit_theta_shifts_x3(e, L, T, nd, load_owed=None):
    """Theta with slli/srli and xor3: 15 (or 10) + 15 + 25 instructions, five temporaries.

    `load_owed(reg)` emits the load of the previous round's constant, which lane 0 has not
    taken yet: column 0's parity and lane 0 take it here through inputs of `xor3` that would
    otherwise be unused.
    """
    T0, T1, T2, T3, T4 = T

    def fresh(d, scratch, cn, cp, x):
        e.ins("slli %s, %s, 1" % (d, cn), "D%d = C%d ^ rotl(C%d, 1)" % (x, (x + 4) % 5, (x + 1) % 5))
        e.ins("srli %s, %s, 63" % (scratch, cn))
        emit_x3(e, nd, d, scratch, cp)

    def over(cn, scratch, cp, x):
        e.ins("srli %s, %s, 63" % (scratch, cn), "D%d = C%d ^ rotl(C%d, 1), over C%d" % (x, (x + 4) % 5, (x + 1) % 5, (x + 1) % 5))
        e.ins("slli %s, %s, 1" % (cn, cn))
        emit_x3(e, nd, cn, scratch, cp)

    emit_parity_x3(e, nd, T0, 4, L)
    emit_parity_x3(e, nd, T1, 1, L)
    fresh(T2, T3, T1, T0, 0)
    if load_owed:
        e.phase("iota")
        load_owed(T3)
        e.phase("theta")
        emit_parity_x3(e, nd, T4, 0, L, owed=T3)
        emit_x3(e, nd, L[0], T2, T3, "A0 ^= D0 ^ the constant owed to A0")
        emit_apply(e, T2, 0, L, skip=0)
    else:
        emit_parity_x3(e, nd, T4, 0, L)
        emit_apply(e, T2, 0, L)
    emit_parity_x3(e, nd, T2, 2, L)
    over(T0, T3, T2, 3)
    over(T2, T3, T4, 1)
    emit_apply(e, T2, 1, L)
    emit_parity_x3(e, nd, T2, 3, L)
    emit_apply(e, T0, 3, L)
    over(T4, T3, T2, 4)
    emit_apply(e, T4, 4, L)
    over(T2, T3, T1, 2)
    emit_apply(e, T2, 2, L)


SHORT_DATA = list(range(9)) + [16]  # lanes that are not zero in a block of at most 64 bytes


RORI_X3_FIRST = 18  # a lane of column 3, the last column theta reaches


def emit_theta_rori_x3(e, L, T, nd):
    """Theta with rori and xor3: 15 (or 10) + 5 + 25 instructions, five temporaries, no D.

    Each lane takes C[x-1] and rotl(C[x+1], 1) in one xor3. Returns the temporary that holds
    lane `RORI_X3_FIRST` afterwards (its register is vacant for the walk of pi).
    """
    T0, T1, T2, T3, T4 = T

    def apply(x, cp, rn, skip=None):
        for y in range(5):
            lane = x + 5 * y
            if lane != skip:
                emit_x3(e, nd, L[lane], cp, rn, "A%d ^= C%d ^ rotl(C%d, 1)" % (lane, (x + 4) % 5, (x + 1) % 5))

    emit_parity_x3(e, nd, T0, 0, L)
    emit_parity_x3(e, nd, T1, 4, L)
    emit_parity_x3(e, nd, T2, 1, L)
    e.ins("rori %s, %s, 63" % (T3, T2), "rotl(C1, 1)")
    apply(0, T1, T3)
    emit_parity_x3(e, nd, T3, 3, L)
    e.ins("rori %s, %s, 63" % (T4, T0), "rotl(C0, 1)")
    apply(4, T3, T4)
    emit_parity_x3(e, nd, T4, 2, L)
    e.ins("rori %s, %s, 63" % (T3, T3), "rotl(C3, 1), over C3")
    apply(2, T2, T3)
    e.ins("rori %s, %s, 63" % (T2, T4), "rotl(C2, 1)")
    apply(1, T0, T2)
    e.ins("rori %s, %s, 63" % (T1, T1), "rotl(C4, 1), over C4")
    apply(3, T4, T1, skip=RORI_X3_FIRST)
    if nd:
        e.ins("xor3 %s, %s, %s, %s" % (T1, T1, T4, L[RORI_X3_FIRST]), "A%d after theta, in the temporary" % RORI_X3_FIRST)
    else:
        emit_x3(e, nd, T1, T4, L[RORI_X3_FIRST], "A%d after theta, in the temporary" % RORI_X3_FIRST)
    return T1


def emit_chi_andn_x3(e, L, T, nd, constant):
    """Chi with andn; lane 0 takes the round constant in `constant` with its own xor."""
    X, Y, Z = [t for t in T if t != constant][:3]
    for y in range(5):
        R = L[5 * y : 5 * y + 5]
        n = [5 * y + i for i in range(5)]
        e.ins("andn %s, %s, %s" % (X, R[0], R[4]), "~B%d & B%d" % (n[4], n[0]))
        e.ins("andn %s, %s, %s" % (Y, R[1], R[0]), "~B%d & B%d" % (n[0], n[1]))
        for i in range(3):
            e.ins("andn %s, %s, %s" % (Z, R[i + 2], R[i + 1]), "~B%d & B%d" % (n[i + 1], n[i + 2]))
            if n[i] == 0:
                emit_x3(e, nd, R[i], Z, constant, "A0, and iota")
            else:
                e.ins("xor %s, %s, %s" % (R[i], R[i], Z), "A%d" % n[i])
        e.ins("xor %s, %s, %s" % (R[3], R[3], X), "A%d" % n[3])
        e.ins("xor %s, %s, %s" % (R[4], R[4], Y), "A%d" % n[4])


def emit_chi_xor3(e, L, free, nd):
    """Chi with xor3 and without NOT over the plain state. Changes `L` and `free`.

    The first lane of each row is written to a free register: its old value is still read by
    the last two outputs of the row. That lane alternates between its register of
    `LANE_REGS` and the temporary of the same index.
    """
    for y in range(5):
        forms = xor3_row_forms(0, 0, y)
        assert all(extra is not None and not out_not for _, _, extra, out_not in forms)
        R = L[5 * y : 5 * y + 5]
        n = [5 * y + i for i in range(5)]
        home, other = LANE_REGS[5 * y], ["a0", "a1", "a2", "a3", "a4"][y]
        fresh = other if R[0] == home else home
        assert fresh in free and R[0] in (home, other)
        Y, Z = [t for t in free if t != fresh][:2]
        sign = {"and": "&", "or": "|"}
        _, op, extra, _ = forms[4]
        assert extra == 0
        e.ins("%s %s, %s, %s" % (op, Y, R[0], R[1]), "B%d %s B%d" % (n[0], sign[op], n[1]))
        for i in range(4):
            _, op, extra, _ = forms[i]
            dst = fresh if i == 0 else Z
            e.ins("%s %s, %s, %s" % (op, dst, R[i + 1], R[(i + 2) % 5]), "B%d %s B%d" % (n[i + 1], sign[op], n[(i + 2) % 5]))
            if i == 0:
                emit_x3(e, nd, fresh, R[0], R[extra], "A%d, in %s: %s stays readable" % (n[0], fresh, R[0]))
            else:
                emit_x3(e, nd, R[i], Z, R[extra], "A%d" % n[i])
        emit_x3(e, nd, R[4], Y, R[0], "A%d" % n[4])
        L[5 * y] = fresh
        free.remove(fresh)
        free.append(R[0])


def emit(variant):
    rot, andn, x3 = VARIANTS[variant]
    nd = x3 is not None and x3.startswith("rd =")
    complemented = 0 if andn or x3 else COMPLEMENTED
    L = LANE_REGS
    # With `rori` theta needs four temporaries, and the fifth register holds the table of round
    # constants: iota is then a load and a xor.
    T = ["a0", "a1", "a2", "a3"] if rot and not x3 else ["a0", "a1", "a2", "a3", "a4"]
    table = "a4" if rot and not x3 else None
    assert sorted(L + T + ([table] if table else [])) == sorted(
        ["ra", "gp", "tp"] + ["t%d" % i for i in range(7)] + ["s%d" % i for i in range(12)]
        + ["a%d" % i for i in range(8)]
    )
    macro = "%s_round" % SYMBOL
    rc_label = ".L%s_rc" % SYMBOL
    # the entries for fresh states and the shortened last round exist in the shipped variant
    hot = variant == "rv64i"

    e = Emitter()
    raw = e.raw
    raw("# Keccak-f[1600] for RV64, variant `%s`. GENERATED by keccak_f1600_rv64_gen.py: do not" % variant)
    raw("# edit, regenerate (`python3 keccak_f1600_rv64_gen.py`; `--check` emulates the text).")
    raw("#")
    if variant != "rv64i":
        raw("# EVALUATION VARIANT: uses %s. It is not compiled; it exists to be counted." % (
            " and ".join(name for name, used in (("`rori`", rot), ("`andn`", andn), ("`xor3`", x3)) if used)))
        if x3:
            raw("# `xor3` (%s) is a hypothetical opcode: no assembler takes this text, only the" % x3)
            raw("# emulator of the generator runs it. The rounds are written out, not a macro: they differ")
            raw("# in the register of five lanes and in where the round constant enters.")
        raw("#")
    raw("# %s(state: *mut [u64; 25])" % SYMBOL)
    raw("#     state <- Keccak-f[1600](state)")
    raw("# %s_absorb(state: *mut [u64; 25], data: *const u64, blocks: usize)" % SYMBOL)
    raw("#     `blocks` >= 1 times: state[0..17] ^= the next 17 words of `data`, then the permutation")
    raw("# %s_final(state: *const [u64; 25], digest: *mut [u64; 4])" % SYMBOL)
    raw("#     digest <- lanes 0-3 of Keccak-f[1600](state), plain; the state in memory is not written")
    raw("#     (the last permutation of a hash: 4 stores instead of 25). `digest` may be `state`.")
    if hot:
        raw("#     Its last round computes those four lanes only.")
        raw("# %s_absorb_first(state: *mut [u64; 25], data: *const u64, blocks: usize)" % SYMBOL)
        raw("#     the same as `_absorb` from the all-zero Keccak state: `state` is only written. The")
        raw("#     first 17 lanes are loaded from `data`, the other 8 are constants.")
        raw("# %s_final64(lanes: *const [u64; 9], digest: *mut [u64; 4])" % SYMBOL)
        raw("#     the whole of a hash of at most 64 bytes: `lanes` are the first nine lanes of its only")
        raw("#     block, PLAIN (the message, the padding byte 0x01, zeros). The routine supplies the")
        raw("#     final padding bit and the zero lanes; its round 0 knows the zero lanes. `digest` may")
        raw("#     be `lanes`. Nothing else is read.")
    raw("#")
    if complemented:
        lanes = ", ".join(str(i) for i in range(25) if complemented >> i & 1)
        raw("# REPRESENTATION. Lanes %s of the state are held complemented, in memory" % lanes)
        raw("# as in the registers (the lane-complementing transform: this machine has no and-not, and")
        raw("# chi then needs one NOT per row instead of five). The caller keeps the state that way: an")
        raw("# all-zero Keccak state has these lanes all-ones. Absorbing is a xor and is not affected.")
        if hot:
            raw("# A digest is plain. `_absorb_first` and `_final64` read plain lanes: a state that no")
            raw("# permutation has touched is not in this representation yet.")
        else:
            raw("# The digest of `_final` is plain: the routine flips lane%s %s back as it stores." % (
                "s" if bin(complemented & 15).count("1") != 1 else "",
                ", ".join(str(i) for i in range(4) if complemented >> i & 1)))
    elif x3 and not andn:
        raw("# REPRESENTATION. The state is the plain Keccak state: with `xor3` chi needs no NOT")
        raw("# (~b & c = c ^ (b & c), and the extra c is the third input of the xor).")
    else:
        raw("# REPRESENTATION. The state is the plain Keccak state: with `andn` chi needs no NOT, and")
        raw("# the lane-complementing transform has nothing to remove.")
    raw("#")
    raw("# REGISTERS. Every lane keeps one register for the whole permutation:")
    for y in range(5):
        raw("#     lanes %2d-%2d   %s" % (5 * y, 5 * y + 4, " ".join("%-3s" % r for r in L[5 * y : 5 * y + 5])))
    raw("#     temporaries   %s" % " ".join(T))
    if table:
        raw("#     %s            the table of round constants" % table)
    if x3 and not andn:
        raw("# Lanes 0, 5, 10, 15 and 20 are in a0-a4 instead after every odd round (chi writes the")
        raw("# first lane of a row to a free register), and their registers above are then temporaries.")
    raw("# That is every register but `zero` and `sp`. `ra`, `gp`, `tp` and `s0`-`s11` are saved in the")
    raw("# frame and restored. No lane is spilled and no register is moved in a round.")
    raw("#")
    raw("# FRAME (%d bytes, `sp` stays 16-byte aligned and is not otherwise touched):" % FRAME)
    raw("#     0(sp) state (or digest) pointer   8(sp) blocks left, or -1 for `_final`")
    raw("#     16(sp) data pointer               24(sp).. saved registers")
    raw("#")
    raw("# ROUND. theta: each D is applied to its column as soon as it exists, so parities and D fit")
    raw("# the temporaries. rho, pi: the 24-cycle of pi is walked backwards, each rotated lane written")
    raw("# into the register of its destination, which the walk has just vacated. chi: in place, row")
    raw("# by row. iota: the constant is loaded PC-relative or is an immediate.")
    raw("#")
    raw("# SAFETY: `state` must be valid for reads and writes of 25 aligned u64 (reads only for")
    raw("# `_final`, writes only for `_absorb_first`), `data` for reads of 17 * blocks aligned u64,")
    raw("# not overlapping `state`, `lanes` for reads of 9 and `digest` for writes of 4 aligned u64.")
    raw("# The routines follow the C calling")
    raw("# convention: they clobber only caller-saved registers, use %d bytes of stack below the" % FRAME)
    raw("# entry `sp`, and read and write nothing else. They are straight-line code but for the block")
    raw("# loop; they must not be interrupted by anything that expects `gp` or `tp` to hold their")
    raw("# usual values (this guest has no interrupts and no threads).")
    raw()
    raw("    .option push")
    raw("    .option norvc")
    raw("    .option norelax")
    if rot or andn:
        raw("    .option arch, +zbb")
    raw()
    raw('    .section .rodata.%s_rc,"a",@progbits' % SYMBOL)
    raw("    .balign 8")
    raw("%s:" % rc_label)
    rc_lines_at = len(e.lines)
    raw()
    raw('    .section .text.%s,"ax",@progbits' % SYMBOL)
    raw("    .balign 4")
    raw()
    absorbed = False
    if not x3:
        raw("# One round without iota. A and B are the lanes of the definition, before theta and before")
        raw("# chi. In theta, rho and pi a complemented lane stays complemented, and a D made of two")
        raw("# parities of different polarity flips its column: the comments there name the lanes without")
        raw("# their polarity. In chi, a and b are the contents of the registers, as each row states.")
        raw(".macro %s" % macro)
        e.phase("theta")
        (emit_theta_rori if rot else emit_theta_shifts)(e, L, T)
        e.phase("rho-pi")
        (emit_rho_pi_rori if rot else emit_rho_pi_shifts)(e, L, T)
        e.phase("chi")
        if andn:
            emit_chi_andn(e, L, T)
        else:
            absorbed = emit_chi_complemented(e, L, T)
        raw(".endm")
        raw()

    constants = [c ^ (M64 if absorbed else 0) for c in RC]
    e.lines[rc_lines_at:rc_lines_at] = [
        "    .dword 0x%016x%s" % (c, "  # round %d%s" % (i, ", complemented: iota also does the NOT that chi owes lane 0" if absorbed and i == 0 else ""))
        for i, c in enumerate(constants)
    ]

    digest_fix = [lane for lane in range(4) if complemented >> lane & 1]

    def prologue(kind):
        e.phase("entry")
        e.ins("addi sp, sp, -%d" % FRAME)
        if kind == "absorb":
            e.ins("sd a2, 8(sp)", "blocks left")
            e.ins("sd a1, 16(sp)", "data pointer")
            e.ins("sd a0, 0(sp)", "state pointer")
        elif kind in ("final", "final64"):
            e.ins("addi a2, zero, -1")
            e.ins("sd a2, 8(sp)", "-1: only the digest leaves the registers")
            e.ins("sd a1, 0(sp)", "digest pointer")
        elif kind == "absorb_first":
            e.ins("addi a2, a2, -1")
            e.ins("addi a3, a1, 136")
            e.ins("sd a2, 8(sp)", "blocks left after this one")
            e.ins("sd a3, 16(sp)", "data pointer of the next block")
            e.ins("sd a0, 0(sp)", "state pointer")
        else:
            e.ins("sd zero, 8(sp)", "0: the state leaves the registers")
            e.ins("sd a0, 0(sp)", "state pointer")
        for k, reg in enumerate(SAVED):
            e.ins("sd %s, %d(sp)" % (reg, 24 + 8 * k))
        if kind == "final64":
            for lane in SHORT_DATA[:-1]:
                e.ins("ld %s, %d(a0)" % (L[lane], 8 * lane), "A%d" % lane)
            e.ins("addi %s, zero, -1" % L[16])
            e.ins("slli %s, %s, 63" % (L[16], L[16]), "A16: the last bit of the padding")
            return
        if kind == "absorb_first":
            for lane in range(17):
                e.ins("ld %s, %d(a1)" % (L[lane], 8 * lane), "A%d = data[%d]" % (lane, lane))
            for lane in range(17):
                if complemented >> lane & 1:
                    e.ins("xori %s, %s, -1" % (L[lane], L[lane]), "lane %d is held complemented" % lane)
            for lane in range(17, 25):
                e.ins("addi %s, zero, %d" % (L[lane], -(complemented >> lane & 1)), "A%d = 0%s" % (lane, ", complemented" if complemented >> lane & 1 else ""))
            return
        for lane in range(25):
            e.ins("ld %s, %d(a0)" % (L[lane], 8 * lane), "A%d" % lane)
        if table:
            label = ".L%s_table_%s" % (SYMBOL, kind)
            raw("%s:" % label)
            e.ins("auipc %s, %%pcrel_hi(%s)" % (table, rc_label))
            e.ins("addi %s, %s, %%pcrel_lo(%s)" % (table, table, label))

    def epilogue():
        for k, reg in enumerate(SAVED):
            e.ins("ld %s, %d(sp)" % (reg, 24 + 8 * k))
        e.ins("addi sp, sp, %d" % FRAME)
        e.ins("jalr zero, 0(ra)", "ret")

    def function(name, is_global=True):
        if is_global:
            raw("    .globl %s" % name)
        raw("    .type %s, @function" % name)
        raw("%s:" % name)

    # The last permutation of a hash is the common entry: it falls through into the rounds.
    function(SYMBOL + "_final")
    prologue("final")
    raw("    .size %s_final, . - %s_final" % (SYMBOL, SYMBOL))
    raw()
    raw("# The 24 rounds and the three ways out, shared by the entries (a local symbol of its own, so")
    raw("# that a profile by function shows the rounds under one name).")
    function(SYMBOL + "_rounds", is_global=False)
    def load_constant(reg, index):
        """The round constant into a register: an immediate, or a PC-relative load."""
        if constants[index] < 0x800:
            e.ins("addi %s, zero, %d" % (reg, constants[index]), "0x%x" % constants[index])
        else:
            label = ".L%s_pc%d" % (SYMBOL, index)
            raw("%s:" % label)
            e.ins("auipc %s, %%pcrel_hi(%s+%d)" % (reg, rc_label, 8 * index))
            e.ins("ld %s, %%pcrel_lo(%s)(%s)" % (reg, label, reg), "0x%016x" % constants[index])

    lanes, free, owed = list(L), list(T), None
    for index in range(24 if x3 else 0):
        raw("    # round %d" % index)
        e.phase("theta")
        if rot:
            holder = emit_theta_rori_x3(e, lanes, free, nd)
            e.phase("rho-pi")
            emit_rho_pi_rori(e, lanes, free, RORI_X3_FIRST, holder)
        else:
            loader = None if owed is None else (lambda reg, k=owed: load_constant(reg, k))
            emit_theta_shifts_x3(e, lanes, free, nd, loader)
            owed = None
            e.phase("rho-pi")
            emit_rho_pi_shifts(e, lanes, free)
        if andn:
            e.phase("iota")
            load_constant(free[3], index)
            e.phase("chi")
            emit_chi_andn_x3(e, lanes, free, nd, free[3])
            continue
        e.phase("chi")
        emit_chi_xor3(e, lanes, free, nd)
        if not rot and not nd and index < 23:
            owed = index  # the next theta has two unused inputs of xor3 for it
            continue
        e.phase("iota")
        if constants[index] < 0x800:
            e.ins("xori %s, %s, %d" % (lanes[0], lanes[0], constants[index]), "A0 ^= 0x%x" % constants[index])
        else:
            load_constant(free[0], index)
            e.ins("xor %s, %s, %s" % (lanes[0], lanes[0], free[0]), "A0 ^= the constant")
    assert lanes == L and owed is None
    for index in range(0 if x3 else 24):
        if hot and index == 1:
            raw(".L%s_round1:" % SYMBOL)
        if hot and index == 23:
            e.phase("dispatch")
            e.ins("ld a0, 8(sp)", "-1: only the digest is wanted")
            e.ins("blt a0, zero, .L%s_digest" % SYMBOL)
        raw("    # round %d" % index)
        raw("    %s" % macro)
        e.phase("iota")
        immediate = constants[index] if constants[index] < 0x800 else constants[index] - (1 << 64)
        if -0x800 <= immediate < 0x800:
            e.ins("xori %s, %s, %d" % (L[0], L[0], immediate), "A0 ^= 0x%x" % constants[index])
        elif table:
            e.ins("ld %s, %d(%s)" % (T[0], 8 * index, table))
            e.ins("xor %s, %s, %s" % (L[0], L[0], T[0]), "A0 ^= 0x%016x" % constants[index])
        else:
            label = ".L%s_pc%d" % (SYMBOL, index)
            raw("%s:" % label)
            e.ins("auipc %s, %%pcrel_hi(%s+%d)" % (T[0], rc_label, 8 * index))
            e.ins("ld %s, %%pcrel_lo(%s)(%s)" % (T[0], label, T[0]))
            e.ins("xor %s, %s, %s" % (L[0], L[0], T[0]), "A0 ^= 0x%016x" % constants[index])
    e.phase("exit")
    if hot:
        e.ins("ld a0, 8(sp)", "0: store the state; else blocks left")
    else:
        e.ins("ld a0, 8(sp)", "-1: digest only; 0: store the state; else blocks left")
        e.ins("blt a0, zero, .L%s_digest" % SYMBOL)
    e.ins("bne a0, zero, .L%s_block" % SYMBOL)
    e.ins("ld a0, 0(sp)", "state pointer")
    for lane in range(25):
        e.ins("sd %s, %d(a0)" % (L[lane], 8 * lane), "A%d" % lane)
    epilogue()
    raw(".L%s_digest:" % SYMBOL)
    if hot:
        raw("    # round 23 of a hash")
        digest, absorbed_last, scratch = emit_digest_round(e, L, T, complemented)
        e.phase("last iota")
        label = ".L%s_pc_last" % SYMBOL
        raw("%s:" % label)
        e.ins("auipc %s, %%pcrel_hi(%s+%d)" % (scratch, rc_label, 8 * (24 if absorbed_last else 23)))
        e.ins("ld %s, %%pcrel_lo(%s)(%s)" % (scratch, label, scratch))
        e.ins("xor %s, %s, %s" % (digest[0], digest[0], scratch), "A0 ^= the constant%s" % (", complemented" if absorbed_last else ""))
        if absorbed_last:
            position = e.lines.index("    .dword 0x%016x  # round 23" % constants[23]) + 1
            e.lines.insert(position, "    .dword 0x%016x  # round 23 of a hash, complemented: its iota also does a NOT" % (constants[23] ^ M64))
        e.phase("exit")
        e.ins("ld a0, 0(sp)", "digest pointer")
        for lane in range(4):
            e.ins("sd %s, %d(a0)" % (digest[lane], 8 * lane), "A%d" % lane)
    else:
        e.ins("ld a0, 0(sp)", "digest pointer")
        for lane in range(4):
            if lane in digest_fix:
                e.ins("xori a1, %s, -1" % L[lane], "lane %d is held complemented" % lane)
                e.ins("sd a1, %d(a0)" % (8 * lane), "A%d" % lane)
            else:
                e.ins("sd %s, %d(a0)" % (L[lane], 8 * lane), "A%d" % lane)
    epilogue()
    raw("    .size %s_rounds, . - %s_rounds" % (SYMBOL, SYMBOL))
    raw()
    function(SYMBOL)
    prologue("permute")
    e.ins("jal zero, %s_rounds" % SYMBOL, "j")
    raw("    .size %s, . - %s" % (SYMBOL, SYMBOL))
    raw()
    function(SYMBOL + "_absorb")
    prologue("absorb")
    raw(".L%s_block:" % SYMBOL)
    e.phase("block")
    e.ins("ld a1, 16(sp)", "data pointer")
    e.ins("ld a0, 8(sp)", "blocks left")
    for lane in range(17):
        e.ins("ld a2, %d(a1)" % (8 * lane))
        e.ins("xor %s, %s, a2" % (L[lane], L[lane]), "A%d ^= data[%d]" % (lane, lane))
    e.ins("addi a1, a1, 136")
    e.ins("addi a0, a0, -1")
    e.ins("sd a1, 16(sp)")
    e.ins("sd a0, 8(sp)")
    e.ins("jal zero, %s_rounds" % SYMBOL, "j")
    raw("    .size %s_absorb, . - %s_absorb" % (SYMBOL, SYMBOL))
    raw()
    if hot:
        function(SYMBOL + "_absorb_first")
        prologue("absorb_first")
        e.ins("jal zero, %s_rounds" % SYMBOL, "j")
        raw("    .size %s_absorb_first, . - %s_absorb_first" % (SYMBOL, SYMBOL))
        raw()
        function(SYMBOL + "_final64")
        prologue("final64")
        raw("    # round 0 of a hash of at most 64 bytes: lanes 9-15 and 17-24 are zero")
        for rotation in range(15):
            attempt = Emitter()
            attempt.phase("short theta")
            try:
                emit_round0_short(attempt, L, T, complemented, SHORT_DATA, rotation)
            except LookupError:
                continue
            e.lines += attempt.lines
            break
        else:
            raise AssertionError("no walk of pi for the short round")
        e.phase("short chi")
        assert not emit_chi_complemented(e, L, T)
        e.phase("short iota")
        e.ins("xori %s, %s, %d" % (L[0], L[0], constants[0]), "A0 ^= 0x%x" % constants[0])
        assert constants[0] < 0x800
        e.phase("entry")
        e.ins("jal zero, .L%s_round1" % SYMBOL, "j")
        raw("    .size %s_final64, . - %s_final64" % (SYMBOL, SYMBOL))
        raw()
    raw("    .option pop")
    return e.text(), complemented


# ---------------------------------------------------------------------------------------------
# The emitted text read back: an emulator of the instructions used, and the counts
# ---------------------------------------------------------------------------------------------

REGISTERS = (
    ["zero", "ra", "sp", "gp", "tp", "t0", "t1", "t2", "s0", "s1"]
    + ["a%d" % i for i in range(8)]
    + ["s%d" % i for i in range(2, 12)]
    + ["t3", "t4", "t5", "t6"]
)
RC_ADDRESS = 0x200000
RETURN_ADDRESS = 0x7FFF0000


class Program:
    """Instructions of the text with macros expanded, labels, the constant table and the phase
    (`# -- name --` marker) each instruction belongs to."""

    def __init__(self, text):
        self.code = []  # (mnemonic, operands, phase, in_macro)
        self.labels = {}
        self.table = []
        macros, current, phase = {}, None, "entry"
        for line in text.splitlines():
            marker = re.match(r"\s*# -- (.+) --$", line)
            line = line.split("#")[0].strip()
            if marker:
                item = ("phase", marker.group(1))
            elif not line:
                continue
            elif line.startswith(".macro"):
                current = macros.setdefault(line.split()[1], [])
                continue
            elif line == ".endm":
                current = None
                continue
            elif line.endswith(":"):
                item = ("label", line[:-1])
            elif line.startswith(".dword"):
                self.table.append(int(line.split()[1], 16))
                continue
            elif line.startswith("."):
                continue
            else:
                mnemonic, _, rest = line.partition(" ")
                item = (mnemonic, [part.strip() for part in rest.split(",")] if rest else [])
            if current is not None:
                current.append(item)
                continue
            for expanded, in_macro in ([(i, True) for i in macros[item[0]]] if item[0] in macros else [(item, False)]):
                if expanded[0] == "phase":
                    phase = expanded[1]
                elif expanded[0] == "label":
                    self.labels[expanded[1]] = len(self.code)
                else:
                    self.code.append((expanded[0], expanded[1], phase, in_macro))

    def run(self, entry, a0, a1, a2, memory):
        """Runs from `entry` to the return. Returns (registers at entry, at exit, executed)."""
        rng = random.Random(a0 ^ a1 ^ a2)
        regs = {name: rng.getrandbits(64) for name in REGISTERS}
        regs.update(zero=0, sp=0x00FF0000, ra=RETURN_ADDRESS, a0=a0, a1=a1, a2=a2)
        before = dict(regs)
        pc, executed = self.labels[entry], 0

        def load(address):
            assert address % 8 == 0, "unaligned access"
            return memory[address]

        def address_of(operand):
            match = re.match(r"(?:(-?\d+)|%pcrel_lo\((\S+)\))\((\w+)\)$", operand)
            assert match, operand
            return (regs[match.group(3)] + int(match.group(1) or 0)) & M64

        while True:
            mnemonic, ops, _, _ = self.code[pc]
            executed += 1
            pc += 1
            value = None
            if mnemonic in ("xor", "or", "and", "andn"):
                x, y = regs[ops[1]], regs[ops[2]]
                value = {"xor": x ^ y, "or": x | y, "and": x & y, "andn": x & ~y & M64}[mnemonic]
            elif mnemonic == "xor3":
                # the hypothetical opcode: three operands xor into rd, four give rd a value
                value = regs[ops[-1]] ^ regs[ops[-2]] ^ regs[ops[-3]]
            elif mnemonic == "xori":
                value = regs[ops[1]] ^ (int(ops[2]) & M64)
            elif mnemonic == "slli":
                value = (regs[ops[1]] << int(ops[2])) & M64
            elif mnemonic == "srli":
                value = regs[ops[1]] >> int(ops[2])
            elif mnemonic == "rori":
                value = rol(regs[ops[1]], 64 - int(ops[2]))
            elif mnemonic == "addi":
                if ops[2].startswith("%pcrel_lo"):
                    value = regs[ops[1]]  # the whole address came with the auipc
                else:
                    value = (regs[ops[1]] + int(ops[2])) & M64
            elif mnemonic == "auipc":
                match = re.match(r"%pcrel_hi\((\S+?)(?:\+(\d+))?\)$", ops[1])
                assert match and match.group(1).endswith("_rc")
                value = RC_ADDRESS + int(match.group(2) or 0)
            elif mnemonic == "ld":
                value = load(address_of(ops[1]))
            elif mnemonic == "sd":
                address = address_of(ops[1])
                assert address % 8 == 0, "unaligned access"
                memory[address] = regs[ops[0]]
            elif mnemonic == "bne":
                if regs[ops[0]] != regs[ops[1]]:
                    pc = self.labels[ops[2]]
            elif mnemonic == "blt":
                assert ops[1] == "zero"
                if regs[ops[0]] >> 63:
                    pc = self.labels[ops[2]]
            elif mnemonic == "jal":
                assert ops[0] == "zero"
                pc = self.labels[ops[1]]
            elif mnemonic == "jalr":
                assert ops == ["zero", "0(ra)"]
                assert regs["ra"] == RETURN_ADDRESS, "return address lost"
                return before, regs, executed
            else:
                raise ValueError("instruction outside the emulated set: " + mnemonic)
            if value is not None:
                assert ops[0] != "sp" or mnemonic == "addi"
                if ops[0] != "zero":
                    regs[ops[0]] = value

    def counts(self):
        """Static counts: per phase of the round macro, per round of iota, entry and exit."""
        result = {}
        for mnemonic, _, phase, in_macro in self.code:
            result[phase] = result.get(phase, 0) + 1
        return result


def check(variant, runs=300):
    """Emulates the three entries of a variant against the reference.

    Returns the program and the instructions executed by: a permutation, a last permutation
    (`_final`), and each further block of `_absorb`.
    """
    text, complemented = emit(variant)
    program = Program(text)
    lanes_mask = [M64 if complemented >> i & 1 else 0 for i in range(25)]
    rng = random.Random(1600)
    state_at, data_at, digest_at = 0x300000, 0x400000, 0x500000
    preserved = ["sp", "gp", "tp"] + ["s%d" % i for i in range(12)]

    def run(entry, state, a1, a2, blocks=(), writes=None, plain=False):
        # `state`: the lanes that exist in memory (a load of any other one fails the run);
        # `plain`: they are not in the representation of the rounds
        memory = {RC_ADDRESS + 8 * i: c for i, c in enumerate(program.table)}
        for i in range(len(state)):
            memory[state_at + 8 * i] = state[i] ^ (0 if plain else lanes_mask[i])
        for i, word in enumerate(word for block in blocks for word in block):
            memory[data_at + 8 * i] = word
        initial = dict(memory)
        before, after, executed = program.run(entry, state_at, a1, a2, memory)
        for name in preserved:
            assert before[name] == after[name], "%s not preserved" % name
        for address in memory:
            if memory[address] != initial.get(address):
                in_frame = before["sp"] - FRAME <= address < before["sp"]
                assert in_frame or writes[0] <= address < writes[1], "write outside the contract"
        return memory, executed

    def stored_state(memory):
        return [memory[state_at + 8 * i] ^ lanes_mask[i] for i in range(25)]

    states = [[0] * 25, [M64] * 25]
    states += [[1 << (bit % 64) if lane == bit // 64 else 0 for lane in range(25)] for bit in range(0, 1600, 7)]
    states += [[rng.getrandbits(64) for _ in range(25)] for _ in range(runs)]
    counts = {}
    for state in states:
        expected = keccak_f1600(state)
        memory, executed = run(SYMBOL, state, 0, 0, writes=(state_at, state_at + 200))
        assert stored_state(memory) == expected, "permutation mismatch"
        assert counts.setdefault("permute", executed) == executed
        # the last permutation: the digest elsewhere, then over the state itself
        memory, executed = run(SYMBOL + "_final", state, digest_at, 0, writes=(digest_at, digest_at + 32))
        assert [memory[digest_at + 8 * i] for i in range(4)] == expected[:4], "digest mismatch"
        assert counts.setdefault("final", executed) == executed
        memory, _ = run(SYMBOL + "_final", state, state_at, 0, writes=(state_at, state_at + 32))
        assert [memory[state_at + 8 * i] for i in range(4)] == expected[:4], "digest mismatch"
    executed_blocks = {}
    for count in (1, 2, 3):
        for _ in range(20):
            state = [rng.getrandbits(64) for _ in range(25)]
            blocks = [[rng.getrandbits(64) for _ in range(17)] for _ in range(count)]
            expected = list(state)
            for block in blocks:
                expected = keccak_f1600([lane ^ (block[i] if i < 17 else 0) for i, lane in enumerate(expected)])
            memory, executed = run(SYMBOL + "_absorb", state, data_at, count, blocks, (state_at, state_at + 200))
            assert stored_state(memory) == expected, "absorb mismatch"
            assert executed_blocks.setdefault(count, executed) == executed
    if SYMBOL + "_final64" in program.labels:
        # a whole hash of at most 64 bytes from its nine plain lanes; only those are in memory
        shorts = [[0] * 9, [M64] * 9, [1] + [0] * 8, [0] * 8 + [1]]
        shorts += [[rng.getrandbits(64) for _ in range(9)] for _ in range(runs)]
        for lanes in shorts:
            block = lanes + [0] * 16
            block[16] ^= 1 << 63
            expected = keccak_f1600(block)[:4]
            memory, executed = run(SYMBOL + "_final64", lanes, digest_at, 0, writes=(digest_at, digest_at + 32), plain=True)
            assert [memory[digest_at + 8 * i] for i in range(4)] == expected, "short digest mismatch"
            assert counts.setdefault("final64", executed) == executed
            memory, _ = run(SYMBOL + "_final64", lanes, state_at, 0, writes=(state_at, state_at + 32), plain=True)
            assert [memory[state_at + 8 * i] for i in range(4)] == expected, "short digest mismatch"
        # whole blocks from the zero state: no lane of the state is in memory before
        first_blocks = {}
        for count in (1, 2, 3):
            for _ in range(20):
                blocks = [[rng.getrandbits(64) for _ in range(17)] for _ in range(count)]
                expected = [0] * 25
                for block in blocks:
                    expected = keccak_f1600([lane ^ (block[i] if i < 17 else 0) for i, lane in enumerate(expected)])
                memory, executed = run(SYMBOL + "_absorb_first", [], data_at, count, blocks, (state_at, state_at + 200))
                assert stored_state(memory) == expected, "first absorb mismatch"
                assert first_blocks.setdefault(count, executed) == executed
        counts["absorb_first, first block"] = first_blocks[1]
        assert first_blocks[2] - first_blocks[1] == executed_blocks[3] - executed_blocks[2]
    counts["absorb, first block"] = executed_blocks[1]
    counts["absorb, next block"] = executed_blocks[3] - executed_blocks[2]
    assert executed_blocks[2] - executed_blocks[1] == counts["absorb, next block"]
    return program, counts, len(states)


def rounds_of(program):
    """The 24 rounds of a program, each the list of (mnemonic, operands, phase, in_macro)."""
    rounds = []
    for item in program.code[program.labels[SYMBOL + "_rounds"] :]:
        if item[2] == "exit":
            break
        if item[2] not in ("theta", "rho-pi", "chi", "iota"):
            continue  # the test before the last round of the shipped variant
        # a round starts with theta and ends after chi (iota may come before, inside or after)
        if item[2] == "theta" and (not rounds or any(other[2] == "chi" for other in rounds[-1])):
            rounds.append([])
        rounds[-1].append(item)
    assert len(rounds) == 24
    return rounds


def report_counts():
    today = 6491
    rows = []
    for variant in VARIANTS:
        program, executed, _ = check(variant, runs=20)
        rounds = rounds_of(program)
        phases = [
            {name: sum(1 for item in items if item[2] == name) for name in ("theta", "rho-pi", "chi", "iota")}
            for items in rounds
        ]
        assert all(len(items) == sum(count.values()) for items, count in zip(rounds, phases))
        # a load, a store or a register copy inside a round would be a spill or a move
        spills = sum(
            1
            for items in rounds
            for mnemonic, ops, phase, _ in items
            if phase != "iota" and (mnemonic in ("ld", "sd") or (mnemonic == "addi" and ops[2] == "0"))
        )
        rows.append((variant, phases, spills, sum(len(items) for items in rounds), executed))
    print("Instructions per round, counted in the emitted text: the commonest round of each variant.")
    print("Iota is shorter where the constant is an immediate (rounds 0, 8, 9), and in `xor3` its")
    print("load sits in the next round's theta, so the first and last rounds differ: `iota x24` and")
    print("`24 rounds` are the exact totals.")
    print("%-19s %6s %7s %5s %5s %13s %6s %9s %10s   %s" % (
        "variant", "theta", "rho-pi", "chi", "iota", "moves+spills", "round", "iota x24", "24 rounds", "temporaries"))
    for variant, phases, spills, total, _ in rows:
        keys = [tuple(sorted(count.items())) for count in phases]
        common = dict(max(set(keys), key=keys.count))
        print("%-19s %6d %7d %5d %5d %13d %6d %9d %10d   %s" % (
            variant, common["theta"], common["rho-pi"], common["chi"], common["iota"], spills,
            sum(common.values()), sum(count["iota"] for count in phases), total,
            "5" if VARIANTS[variant][2] or not VARIANTS[variant][0] else "4, and a register for the constants"))
    print()
    print("Instructions per call, executed by the emulator (entry and exit included, the caller's")
    print("`jal` not). permutation: state to state; last: the digest-only entry; absorb: the first")
    print("block of a call, and each further block.")
    print("%-19s %10s %12s %7s %7s %13s %12s" % (
        "variant", "24 rounds", "permutation", "in+out", "last", "absorb first", "absorb next"))
    for variant, _, _, rounds, executed in rows:
        print("%-19s %10d %12d %7d %7d %13d %12d" % (
            variant, rounds, executed["permute"], executed["permute"] - rounds, executed["final"],
            executed["absorb, first block"], executed["absorb, next block"]))
    program, executed, _ = check("rv64i", runs=20)
    phases = program.counts()
    print()
    print("rv64i only. The last round of a hash (lanes 0-3 alone): theta %d, rho-pi %d, chi %d, iota %d = %d" % (
        phases["last theta"], phases["last rho-pi"], phases["last chi"], phases["last iota"],
        sum(phases[name] for name in ("last theta", "last rho-pi", "last chi", "last iota"))))
    print("instead of 195. Round 0 of a hash of at most 64 bytes (15 zero lanes): theta %d, rho-pi %d," % (
        phases["short theta"], phases["short rho-pi"]))
    print("chi %d, iota %d = %d instead of 193. Per call: a hash of at most 64 bytes from its nine" % (
        phases["short chi"], phases["short iota"],
        sum(phases[name] for name in ("short theta", "short rho-pi", "short chi", "short iota"))))
    print("lanes %d; the first block of `_absorb_first` %d (of `_absorb` %d)." % (
        executed["final64"], executed["absorb_first, first block"], executed["absorb, first block"]))
    print()
    base = rows[0][4]["permute"]
    print("%-19s %12s %18s %22s" % ("variant", "permutation", "against rv64i", "against today's %d" % today))
    for variant, _, _, _, executed in rows:
        count = executed["permute"]
        print("%-19s %12d %9d (%5.1f%%) %13d (%5.1f%%)" % (
            variant, count, count - base, 100.0 * (count - base) / base,
            count - today, 100.0 * (count - today) / today))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--variant", choices=list(VARIANTS), help="print this variant to stdout")
    parser.add_argument("--check", action="store_true", help="emulate every variant against the reference")
    parser.add_argument("--counts", action="store_true", help="print the instruction counts of every variant")
    parser.add_argument("--search", action="store_true", help="search the sets of complemented lanes")
    args = parser.parse_args()
    check_reference()
    assert round_cost(COMPLEMENTED) == 5
    if args.search:
        found = search_masks()
        found.sort(key=lambda mask: (bin(mask & 15).count("1"), bin(mask).count("1"), mask))
        print("%d sets of complemented lanes with five NOTs per round:" % len(found))
        for mask in found:
            print("   ", [lane for lane in range(25) if mask >> lane & 1])
    elif args.check:
        for variant in VARIANTS:
            _, executed, states = check(variant)
            print("%-19s ok: %d states through each entry, absorb of 1-3 blocks; instructions: %s" % (
                variant, states, ", ".join("%s %d" % item for item in executed.items())))
    elif args.counts:
        report_counts()
    elif args.variant:
        sys.stdout.write(emit(args.variant)[0])
    else:
        check("rv64i", runs=50)
        path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "keccak_f1600_rv64.S")
        with open(path, "w") as output:
            output.write(emit("rv64i")[0])
        print("wrote", path)


if __name__ == "__main__":
    main()
