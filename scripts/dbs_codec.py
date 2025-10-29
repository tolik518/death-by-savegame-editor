#!/usr/bin/env python3
import sys, os, struct, argparse, secrets

# python3 dbs_codec.py decrypt ../dbs_plain/key_btea.bin ../saves/13337gems payload.txt
# ---------- XXTEA (BTEA) ----------
DELTA = 0x9E3779B9

def _u32(x): return x & 0xFFFFFFFF

def xxtea_encrypt_block(v, k):
    n = len(v)
    if n < 2: return v
    rounds = 6 + 52 // n
    z = v[-1]; y = v[0]; sumv = 0
    for _ in range(rounds):
        sumv = _u32(sumv + DELTA)
        e = (sumv >> 2) & 3
        for p in range(n-1):
            y = v[p+1]
            mx = ((_u32((z>>5) ^ (y<<2)) + _u32((y>>3) ^ (z<<4))) ^ _u32((sumv ^ y) + (k[(p & 3) ^ e] ^ z)))
            v[p] = _u32(v[p] + mx)
            z = v[p]
        y = v[0]
        mx = ((_u32((z>>5) ^ (y<<2)) + _u32((y>>3) ^ (z<<4))) ^ _u32((sumv ^ y) + (k[((n-1) & 3) ^ e] ^ z)))
        v[-1] = _u32(v[-1] + mx)
        z = v[-1]
    return v

def xxtea_decrypt_block(v, k):
    n = len(v)
    if n < 2: return v
    rounds = 6 + 52 // n
    sumv = _u32(rounds * DELTA)
    z = v[-1]; y = v[0]
    while rounds > 0:
        e = (sumv >> 2) & 3
        for p in range(n-1, 0, -1):
            z = v[p-1]
            mx = ((_u32((z>>5) ^ (y<<2)) + _u32((y>>3) ^ (z<<4))) ^ _u32((sumv ^ y) + (k[(p & 3) ^ e] ^ z)))
            v[p] = _u32(v[p] - mx)
            y = v[p]
        z = v[-1]
        mx = ((_u32((z>>5) ^ (y<<2)) + _u32((y>>3) ^ (z<<4))) ^ _u32((sumv ^ y) + (k[(0 & 3) ^ e] ^ z)))
        v[0] = _u32(v[0] - mx)
        y = v[0]
        sumv = _u32(sumv - DELTA)
        rounds -= 1
    return v

def xxtea_encrypt_bytes(b, key16_le):
    # pad to 4-byte words (XXTEA arbeitet auf 32-bit Wörtern, n>=2)
    pad = (-len(b)) & 3
    bb = b + b'\x00'*pad
    v = list(struct.unpack("<%dI" % (len(bb)//4), bb))
    if len(v) == 1: v.append(0)  # n>=2
    k = list(struct.unpack("<4I", key16_le))
    vv = xxtea_encrypt_block(v, k)
    return struct.pack("<%dI" % len(vv), *vv)[:len(bb)]

def xxtea_decrypt_bytes(b, key16_le):
    pad = (-len(b)) & 3
    bb = b + b'\x00'*pad
    v = list(struct.unpack("<%dI" % (len(bb)//4), bb))
    if len(v) == 1: v.append(0)
    k = list(struct.unpack("<4I", key16_le))
    vv = xxtea_decrypt_block(v, k)
    out = struct.pack("<%dI" % len(vv), *vv)
    return out[:len(b)]

# ---------- Trailer helpers ----------
def calc_checksum(payload: bytes) -> int:
    return (0x06583463 + sum(payload)) & 0xFFFFFFFF

def pack_block(payload: bytes, extra4: int = 0, padlen: int = None) -> bytes:
    if padlen is None:
        padlen = secrets.randbelow(9)  # 0..8
    assert 0 <= padlen <= 8
    checksum = calc_checksum(payload)
    footer = struct.pack("<I", checksum) + struct.pack("<I", extra4)
    pad = secrets.token_bytes(padlen) if padlen else b""
    return payload + footer + pad + bytes([padlen])

def unpack_block(plain: bytes):
    padlen = plain[-1]
    if not (0 <= padlen <= 8):
        raise ValueError(f"padlen out of range: {padlen}")
    payload_len = len(plain) - padlen - 9
    payload = plain[:payload_len]
    checksum, extra4 = struct.unpack_from("<II", plain, payload_len)
    return payload, checksum, extra4, padlen

# ---------- CLI ----------
def cmd_decrypt_cipher(args):
    key = open(args.key, "rb").read()
    enc = open(args.cipher, "rb").read()
    plain = xxtea_decrypt_bytes(enc, key)
    payload, csum, extra4, padlen = unpack_block(plain)
    calc = calc_checksum(payload)
    print(f"[info] len(enc)={len(enc)} padlen={padlen} extra4=0x{extra4:08x}")
    print(f"[info] checksum stored=0x{csum:08x} calc=0x{calc:08x} -> {'OK' if csum==calc else 'MISMATCH'}")
    open(args.out_plain, "wb").write(payload)
    print(f"[ok] wrote payload plaintext -> {args.out_plain}")

def cmd_encrypt_plain(args):
    key = open(args.key, "rb").read()
    payload = open(args.plain, "rb").read()
    # optional: keep extra4 from reference block
    extra4 = 0
    if args.ref_block:
        blk = open(args.ref_block, "rb").read()
        _, _, extra4, _ = unpack_block(blk)
        print(f"[info] reusing extra4 from ref: 0x{extra4:08x}")
    block = pack_block(payload, extra4=extra4, padlen=args.padlen)
    enc = xxtea_encrypt_bytes(block, key)
    open(args.out_cipher, "wb").write(enc)
    print(f"[ok] wrote encrypted block -> {args.out_cipher}")

def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)

    d = sub.add_parser("decrypt", help="cipher -> payload")
    d.add_argument("key")
    d.add_argument("cipher")
    d.add_argument("out_plain")
    d.set_defaults(func=cmd_decrypt_cipher)

    e = sub.add_parser("encrypt", help="payload -> cipher")
    e.add_argument("key")
    e.add_argument("plain")
    e.add_argument("out_cipher")
    e.add_argument("--ref-block", help="optional plain_btea.bin to reuse extra4", default=None)
    e.add_argument("--padlen", type=int, default=None)
    e.set_defaults(func=cmd_encrypt_plain)

    args = ap.parse_args()
    args.func(args)

if __name__ == "__main__":
    main()
