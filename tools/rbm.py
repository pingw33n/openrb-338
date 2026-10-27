#!/usr/bin/python3
"""Reference reader of ReBirth RB-338 2.01 mods (.rbm): container, key table, descrambling.

Format and derivation: docs/rbm.md (addresses there). Stdlib only.

An .rbm is an IFF `CAT ` of type `PRBM` holding a 256-byte HEAD, EMBF chunks (one embedded file
each: a NUL-terminated name, then the file) and a 1280-byte INFO. The payload of an EMBF chunk is
scrambled from byte 1024 on; the 4096-byte key table that undoes it is two 2048-byte pages of
RB20FUL.DAT (the ReBirth CD) whose offsets follow from the mod's own bytes. RB20FUL.DAT, its
pages, the key tables and the mods are Propellerhead material: never commit them.

CLI:
    rbm.py info MOD                       chunks, INFO fields, the slot mapping, the key pages
    rbm.py extract MOD DAT -o DIR         write every embedded file, descrambled, to DIR
    rbm.py check-dat DAT                  the anti-tamper constants that make the key stride 4
    rbm.py msg MOD DAT --path WINPATH --id N -o FILE
                                          the exe's top-level message 55 for the mod (rbhost --mod-msg)
"""
import argparse
import os
import struct
import sys
import warnings
from dataclasses import dataclass, field
from typing import List, Optional

# EMBF payload bytes [0, CLEAR) are stored as they are; [CLEAR, size) are scrambled (DLL 0x1001A5A9).
CLEAR = 1024
# RB20FUL.DAT: a 256-byte header (sub_404C35 returns 256), then 2^27 bytes of pages.
DAT_HEADER = 256
DAT_BODY = 0x8000000
PAGE = 2048
DAT_MAGIC = 0x03E1F11E
# Initial key sums, dword_4BB290 (ctor sub_414987).
SUM_INIT = (0x2000, 0x3000)
# The stride of the key-sum words in an untampered run (sub_4149EC 0x414A5F..0x414A86): 4.
STRIDE = 4
# CFileSpec serialization tag of the exe (sub_483143: sub_465FFF(v6, -249670420)).
FILESPEC_TAG = 0xF11E54EC

# Sample names the exe looks up, case-insensitively, first match wins (sub_41693C, sub_416710,
# sub_4713D4). 808: 0x4BB388, 909: 0x4BB520 (17-byte stride), startup: 0x4BB378.
SLOTS808 = ["TR808BD.aif", "TR808SD1.aif", "TR808SD2.aif", "TR808SD3.aif", "TR808SD4.aif",
            "TR808SD5.aif", "TR808N1.aif", "TR808N2.aif", "TR808N3.aif", "TR808N4.aif",
            "TR808CB.aif", "TR808CH.aif", "TR808CL.aif", "TR808CP.aif", "TR808CY.aif",
            "TR808HC.aif", "TR808HT.aif", "TR808LC.aif", "TR808LT.aif", "TR808MA.aif",
            "TR808MC.aif", "TR808MT.aif", "TR808OH.aif", "TR808RS.aif"]
SLOTS909 = ["TR909bd1.aif", "TR909bd2.aif", "TR909bd3.aif", "TR909bd4.aif", "TR909sd1.aif",
            "TR909sd2.aif", "TR909sd3.aif", "TR909sd4.aif", "TR909sd5.aif", "TR909sdn1.aif",
            "TR909sdn2.aif", "TR909sdn3.aif", "TR909lt.aif", "TR909mt.aif", "TR909ht.aif",
            "TR909rs.aif", "TR909hc.aif", "TR909chh.aif", "TR909ohh.aif", "TR909ccy.aif",
            "TR909rcy.aif"]
STARTUP = "Startup.aif"
# The DLL's sample table slots (sub_1001A030): 1..24 = 808, 25..45 = 909, 46 = startup.
SLOT_NAMES = [None] + SLOTS808 + SLOTS909 + [STARTUP]


class FormatError(ValueError):
    pass


@dataclass
class Chunk:
    ordinal: int      # index among all chunks of the CAT, HEAD = 0 (sub_41813E v59)
    tag: bytes
    offset: int       # absolute file offset of the payload
    size: int         # payload size
    name: Optional[str] = None   # EMBF only
    name_len: int = 0            # EMBF only: strlen(name) + 1, where the file starts

    @property
    def key(self):
        """The descrambling key: the low byte of the ordinal (sub_41693C v32[1] = Str[272])."""
        return self.ordinal & 0xFF


@dataclass
class Info:
    text: bytes       # 0..600, the description shown for the mod
    ftp: bytes        # 601..801
    web: bytes        # 802..1002
    name: bytes       # 1003..1067, the name in the Mods menu
    version: Optional[tuple]  # 1073..1076: 0xBC tag, then major, minor, revision; None if absent


@dataclass
class Rbm:
    data: bytes
    chunks: List[Chunk] = field(default_factory=list)
    info: Optional[Info] = None

    def embf(self):
        return [c for c in self.chunks if c.tag == b"EMBF"]

    def find(self, name):
        """The first EMBF whose name equals `name` ignoring case (sub_416710), or None."""
        low = name.lower()
        for c in self.embf():
            if c.name.lower() == low:
                return c
        return None


def _cstr(b):
    return b.split(b"\0", 1)[0]


def parse(data):
    if len(data) < 12 or data[:4] != b"CAT ":
        raise FormatError("not an IFF CAT")
    size = struct.unpack_from(">I", data, 4)[0]
    if size + 8 != len(data):
        raise FormatError("CAT size %d does not match the file (%d)" % (size, len(data)))
    if data[8:12] != b"PRBM":
        # the exe does not check the type (docs/rbm.md section 2): accept, as it does
        warnings.warn("CAT type %r is not PRBM" % data[8:12])
    rbm = Rbm(data)
    pos, end, n = 12, len(data), 0
    while pos + 8 <= end:
        tag = data[pos:pos + 4]
        csize = struct.unpack_from(">I", data, pos + 4)[0]
        body = pos + 8
        if body + csize > end:
            raise FormatError("chunk %r at %d overruns the file" % (tag, pos))
        c = Chunk(n, tag, body, csize)
        if tag == b"EMBF":
            nul = data.find(b"\0", body, body + csize)
            if nul < 0:
                raise FormatError("EMBF at %d has no file name" % pos)
            c.name = data[body:nul].decode("latin-1")
            c.name_len = nul - body + 1
        elif tag == b"INFO" and csize == 1280:
            b = data[body:body + 1280]
            ver = tuple(b[1074:1077]) if b[1073] == 0xBC else None
            rbm.info = Info(_cstr(b[0:600]), _cstr(b[601:801]), _cstr(b[802:1002]), _cstr(b[1003:1067]), ver)
        rbm.chunks.append(c)
        n += 1
        pos = body + csize + (csize & 1)
    if pos != end:
        raise FormatError("trailing bytes at %d" % pos)
    return rbm


def read(path):
    with open(path, "rb") as f:
        return parse(f.read())


# ---------------------------------------------------------------- key table


def key_sums(rbm, stride=STRIDE):
    """The two u32 sums that pick the key pages (sub_414D18 0x4151B7..0x4152EA -> sub_4149EC)."""
    acc = list(SUM_INIT)
    for c in rbm.embf():
        n = min(c.size, 1024)
        buf = bytearray(b"\x65" * 1024)
        buf[:n] = rbm.data[c.offset:c.offset + n]
        idx = 0
        for j in range(2):
            s = 0
            for _ in range(128):
                o = idx * stride
                s = (s + struct.unpack_from(">I", buf, o)[0]) & 0xFFFFFFFF
                idx += 1
            acc[j] = (acc[j] + s) & 0xFFFFFFFF
    return acc


def key_pages(rbm):
    """The file offsets of the two 2048-byte pages of RB20FUL.DAT (sub_4158A9 0x415ED5..0x415F2F)."""
    return [DAT_HEADER + s % (DAT_BODY - PAGE) for s in key_sums(rbm)]


def check_dat(f):
    head = f.read(DAT_HEADER)
    if len(head) != DAT_HEADER or struct.unpack(">I", head[:4])[0] != DAT_MAGIC:
        raise FormatError("not RB20FUL.DAT (magic 0x%08X)" % DAT_MAGIC)


def check_stride_pages(dat_path):
    """The anti-tamper constants behind the key stride (docs/rbm.md 5.2), over all 16384 pages
    k + 0x4000 that sub_4065F2 may read into unk_4DAD78. Returns two {value: count} dicts:
    ((dword_4DAE58 & 0xFFFFF000) >> 14) - k (18 everywhere gives stride 18 - 14 = 4) and
    ((dword_4DB124 & 0xFFFFF000) >> 12) - k (8 everywhere gives sub_420568's 3 * 8 = 24)."""
    x, y = {}, {}
    with open(dat_path, "rb") as f:
        check_dat(f)
        for k in range(0x4000):
            f.seek(DAT_HEADER + ((k + 0x4000) << 11))
            page = f.read(PAGE)
            g = struct.unpack_from("<I", page, 0x4DAE58 - 0x4DAD78)[0]
            g2 = struct.unpack_from("<I", page, 0x4DB124 - 0x4DAD78)[0]
            a = ((g & 0xFFFFF000) >> 14) - k
            b = ((g2 & 0xFFFFF000) >> 12) - k
            x[a] = x.get(a, 0) + 1
            y[b] = y.get(b, 0) + 1
    return x, y


def key_table(rbm, dat_path):
    """The 4096-byte key table of the mod: pages k0 and k1 of RB20FUL.DAT, concatenated."""
    out = bytearray()
    with open(dat_path, "rb") as f:
        check_dat(f)
        for off in key_pages(rbm):
            f.seek(off)
            page = f.read(PAGE)
            if len(page) != PAGE:
                raise FormatError("RB20FUL.DAT is short")
            out += page
    return bytes(out)


def descramble(payload, key, table):
    """Undo the scrambling of one EMBF payload (DLL sub_100163D0 on bytes [1024, size))."""
    out = bytearray(payload)
    if len(out) <= CLEAR:
        return bytes(out)
    period = (17 * key) % 0x785 + 123
    base = (key & 1) << 11
    j = 0
    for n in range(CLEAR, len(out)):
        out[n] = (out[n] + table[base + j]) & 0xFF
        j += 1
        if j == period:
            j = 0
    return bytes(out)


def scramble(payload, key, table):
    """The inverse of descramble (what the mod packer did)."""
    out = bytearray(payload)
    period = (17 * key) % 0x785 + 123
    base = (key & 1) << 11
    j = 0
    for n in range(CLEAR, len(out)):
        out[n] = (out[n] - table[base + j]) & 0xFF
        j += 1
        if j == period:
            j = 0
    return bytes(out)


def embedded_file(rbm, chunk, table):
    """The descrambled embedded file of an EMBF chunk (without its name)."""
    payload = rbm.data[chunk.offset:chunk.offset + chunk.size]
    return descramble(payload, chunk.key, table)[chunk.name_len:]


# ---------------------------------------------------------------- AIFF check


def aiff_info(b):
    """(channels, frames, bits, rate, ssnd_offset) of an AIFF; FormatError if it does not parse
    the way the DLL reads it (sub_10004F94, sub_10001674, sub_10002FC4)."""
    if len(b) < 12 or b[:4] != b"FORM" or b[8:12] != b"AIFF":
        raise FormatError("not FORM/AIFF")
    pos, end = 12, min(len(b), 8 + struct.unpack_from(">I", b, 4)[0])
    comm = ssnd = None
    while pos + 8 <= end:
        tag = b[pos:pos + 4]
        size = struct.unpack_from(">I", b, pos + 4)[0]
        if tag == b"COMM" and comm is None:
            comm = pos + 8
        elif tag == b"SSND" and ssnd is None:
            ssnd = (pos + 8, size)
        pos += 8 + size + (size & 1)
    if comm is None or ssnd is None:
        raise FormatError("no COMM or SSND")
    ch, frames, bits = struct.unpack_from(">HIH", b, comm)
    exp = struct.unpack_from(">H", b, comm + 8)[0] & 0x7FFF
    mant = struct.unpack_from(">Q", b, comm + 10)[0]
    rate = int(mant * 2.0 ** (exp - 16383 - 63)) if mant else 0
    off = struct.unpack_from(">I", b, ssnd[0])[0]
    if not ch or not frames or bits != 16:
        raise FormatError("COMM %d ch %d frames %d bits" % (ch, frames, bits))
    if ssnd[0] + 8 + off + 2 * ch * frames > len(b):
        raise FormatError("SSND shorter than COMM's frames")
    return ch, frames, bits, rate, off


def aiff_samples(b):
    """Channel 0 of an AIFF as a list of int16, as the DLL's CWaveSample holds it."""
    ch, frames, _, _, off = aiff_info(b)
    pos = 12
    while b[pos:pos + 4] != b"SSND":
        size = struct.unpack_from(">I", b, pos + 4)[0]
        pos += 8 + size + (size & 1)
    start = pos + 16 + off
    return [struct.unpack_from(">h", b, start + 2 * ch * i)[0] for i in range(frames)]


# ---------------------------------------------------------------- message 55


def _record(chunk):
    """14 bytes: present, key, u32 payload offset, u32 name length, u32 payload size (BE)."""
    if chunk is None:
        return bytes(14)
    return bytes([1, chunk.key]) + struct.pack(">III", chunk.offset, chunk.name_len, chunk.size)


def mod_message(rbm, table, win_path, mod_id):
    """The exe's top-level message 55 for this mod: type byte + 9542 bytes (sub_420568,
    unpacked by the DLL's sub_10018870), in the layout of an untampered run."""
    p = bytearray(0x2546)
    struct.pack_into(">H", p, 0, mod_id)
    spec = struct.pack(">I", FILESPEC_TAG) + win_path.encode("latin-1") + b"\0"
    if 2 + len(spec) > 4802:
        raise ValueError("path too long")
    p[2:2 + len(spec)] = spec
    p[4802:4802 + 4096] = table
    for i, name in enumerate(SLOTS808):
        p[8898 + 14 * i:8898 + 14 * i + 14] = _record(rbm.find(name))
    for i, name in enumerate(SLOTS909):
        p[9234 + 14 * i:9234 + 14 * i + 14] = _record(rbm.find(name))
    p[9528:9542] = _record(rbm.find(STARTUP))
    return bytes([55]) + bytes(p)


# ---------------------------------------------------------------- CLI


def cmd_info(a):
    rbm = read(a.mod)
    i = rbm.info
    if i:
        print("name: %s" % i.name.decode("latin-1"))
        print("version: %s" % (".".join(map(str, i.version)) if i.version else "(app's)"))
        print("ftp: %s\nweb: %s" % (i.ftp.decode("latin-1"), i.web.decode("latin-1")))
    print("chunks: %d (%d EMBF)" % (len(rbm.chunks), len(rbm.embf())))
    for c in rbm.chunks:
        print("  %3d %s off %8d size %7d key %3d %s" % (c.ordinal, c.tag.decode("latin-1"), c.offset, c.size,
                                                        c.key, c.name or ""))
    print("slots:")
    for s, name in enumerate(SLOT_NAMES[1:], 1):
        c = rbm.find(name)
        print("  %2d %-14s %s" % (s, name, ("chunk %d" % c.ordinal) if c else "built-in"))
    sums = key_sums(rbm)
    print("key sums: 0x%08X 0x%08X" % tuple(sums))
    print("key pages (RB20FUL.DAT offsets): %d %d" % tuple(key_pages(rbm)))


def cmd_extract(a):
    rbm = read(a.mod)
    table = key_table(rbm, a.dat)
    os.makedirs(a.out, exist_ok=True)
    bad = 0
    for c in rbm.embf():
        b = embedded_file(rbm, c, table)
        note = ""
        if c.name.lower().endswith((".aif", ".aiff")):
            try:
                ch, frames, bits, rate, _ = aiff_info(b)
                note = "AIFF %d ch %d frames %d Hz" % (ch, frames, rate)
            except FormatError as e:
                note = "BAD AIFF: %s" % e
                bad += 1
        with open(os.path.join(a.out, os.path.basename(c.name)), "wb") as f:
            f.write(b)
        print("%-16s %7d %s" % (c.name, len(b), note))
    return 1 if bad else 0


def cmd_check_dat(a):
    x, y = check_stride_pages(a.dat)
    print("((dword_4DAE58 & 0xFFFFF000) >> 14) - k: %s" % sorted(x.items()))
    print("((dword_4DB124 & 0xFFFFF000) >> 12) - k: %s" % sorted(y.items()))
    ok = x == {18: 0x4000} and y == {8: 0x4000}
    print("stride %s" % ("4 on every page" if ok else "NOT constant"))
    return 0 if ok else 1


def cmd_msg(a):
    rbm = read(a.mod)
    with open(a.out, "wb") as f:
        f.write(mod_message(rbm, key_table(rbm, a.dat), a.path, a.id))


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("info")
    p.add_argument("mod")
    p.set_defaults(fn=cmd_info)
    p = sub.add_parser("extract")
    p.add_argument("mod")
    p.add_argument("dat")
    p.add_argument("-o", "--out", required=True)
    p.set_defaults(fn=cmd_extract)
    p = sub.add_parser("check-dat")
    p.add_argument("dat")
    p.set_defaults(fn=cmd_check_dat)
    p = sub.add_parser("msg")
    p.add_argument("mod")
    p.add_argument("dat")
    p.add_argument("--path", required=True, help="the mod's path on the machine that runs the DLL")
    p.add_argument("--id", type=int, default=412, help="the mod id (default 412)")
    p.add_argument("-o", "--out", required=True)
    p.set_defaults(fn=cmd_msg)
    a = ap.parse_args(argv)
    return a.fn(a) or 0


if __name__ == "__main__":
    sys.exit(main())
