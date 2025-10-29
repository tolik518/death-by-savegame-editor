#!/usr/bin/env python3
import sys, os, struct, argparse
# 1. python3 dbs_codec.py decrypt ../saves/13337gems payload.hocon
# 2. ???
# 3. python3 dbs_codec.py encrypt payload.hocon ../save.bin

# 16-Byte XXTEA/BTEA Key (little endian, gedumpt)
KEY = bytes.fromhex(
    "93 9d ab 7a 2a 56 f8 af b4 db a9 b5 22 a3 4b 2b".replace(" ", "")
    )

# extra4-Feld im Footer
EXTRA4 = 0x0169027d

# XXTEA / BTEA (mit ghidra gedumped)
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
            mx = ((_u32((z>>5) ^ (y<<2)) + _u32((y>>3) ^ (z<<4)))
                  ^ _u32((sumv ^ y) + (k[(p & 3) ^ e] ^ z)))
            v[p] = _u32(v[p] + mx); z = v[p]
        y = v[0]
        mx = ((_u32((z>>5) ^ (y<<2)) + _u32((y>>3) ^ (z<<4)))
              ^ _u32((sumv ^ y) + (k[((n-1) & 3) ^ e] ^ z)))
        v[-1] = _u32(v[-1] + mx); z = v[-1]
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
            mx = ((_u32((z>>5) ^ (y<<2)) + _u32((y>>3) ^ (z<<4)))
                  ^ _u32((sumv ^ y) + (k[(p & 3) ^ e] ^ z)))
            v[p] = _u32(v[p] - mx); y = v[p]
        z = v[-1]
        mx = ((_u32((z>>5) ^ (y<<2)) + _u32((y>>3) ^ (z<<4)))
              ^ _u32((sumv ^ y) + (k[(0 & 3) ^ e] ^ z)))
        v[0] = _u32(v[0] - mx); y = v[0]
        sumv = _u32(sumv - DELTA); rounds -= 1
    return v

def xxtea_encrypt_bytes(b, key16_le):
    pad = (-len(b)) & 3
    bb = b + b'\x00'*pad
    v = list(struct.unpack("<%dI" % (len(bb)//4), bb))
    if len(v) == 1: v.append(0)
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
    return struct.pack("<%dI" % len(vv), *vv)[:len(b)]

# Footer + Checksum
# Layout: [payload | checksum(4 LE) | extra4(4 LE) | pad(0..8) | padlen(1)]
def calc_checksum(payload: bytes) -> int:
    # Engine: 0x06583463 + Summe aller Payload-Bytes (mod 2^32)
    return (0x06583463 + sum(payload)) & 0xFFFFFFFF

def pack_block(payload: bytes, extra4: int = EXTRA4, pad_bytes: bytes | None = None) -> bytes:
    """
    baut den Klartext-Block so, dass seine Länge % 8 == 0 bleibt (Engine-Requirement).
    Wenn pad_bytes None ist, generieren wir deterministische Null-Bytes (kein RNG nötig).
    """
    base = len(payload) + 9  # 4(checksum) + 4(extra4) + 1(padlen)
    # kleinste padlen (0..7), die total_len % 8 == 0 macht
    padlen = (-base) & 7
    checksum = calc_checksum(payload)
    footer = struct.pack("<II", checksum, extra4)
    if padlen:
        if pad_bytes is None:
            pad = b"\x00" * padlen
        else:
            if len(pad_bytes) < padlen:
                raise ValueError("pad_bytes zu kurz")
            pad = pad_bytes[:padlen]
    else:
        pad = b""
    block = payload + footer + pad + bytes([padlen])
    # Sanity: Engine verlangt multiples of 8
    assert len(block) % 8 == 0, "plain block must be multiple of 8"
    return block

def unpack_block(plain: bytes):
    if len(plain) < 9:
        raise ValueError("Block zu kurz")
    padlen = plain[-1]
    if not (0 <= padlen <= 8):
        raise ValueError(f"padlen out of range: {padlen}")
    payload_len = len(plain) - padlen - 9
    if payload_len < 0:
        raise ValueError("payload_len < 0 (korrupt?)")
    payload = plain[:payload_len]
    checksum, extra4 = struct.unpack_from("<II", plain, payload_len)
    return payload, checksum, extra4, padlen

# CLI (decrypt/encrypt)
def cmd_decrypt(args):
    enc = open(args.cipher, "rb").read()
    if (len(enc) & 7) != 0:
        print(f"[warn] cipher len {len(enc)} ist kein Vielfaches von 8 – Engine würde das ablehnen.")
    plain = xxtea_decrypt_bytes(enc, KEY)
    payload, csum, extra4, padlen = unpack_block(plain)
    calc = calc_checksum(payload)
    print(f"[info] len(enc)={len(enc)}  padlen={padlen}  extra4=0x{extra4:08x}")
    print(f"[info] checksum stored=0x{csum:08x}  calc=0x{calc:08x}  -> {'OK' if csum==calc else 'MISMATCH'}")
    open(args.out_plain, "wb").write(payload)
    print(f"[ok] wrote payload -> {args.out_plain}")

def cmd_encrypt(args):
    payload = open(args.plain, "rb").read()
    # Baue Plain-Block so, dass (len % 8 == 0), checksum passt, und EXTRA4 drin ist
    block = pack_block(payload, extra4=EXTRA4)
    enc = xxtea_encrypt_bytes(block, KEY)
    # Sanity
    assert (len(enc) & 7) == 0, "cipher must be multiple of 8"
    open(args.out_cipher, "wb").write(enc)
    print(f"[ok] wrote encrypted block -> {args.out_cipher}")

def main():
    ap = argparse.ArgumentParser(description="Death by Scrolling save (de|en)crypt – minimal, hardcoded")
    sub = ap.add_subparsers(dest="cmd", required=True)

    d = sub.add_parser("decrypt", help="cipher -> plaintext payload")
    d.add_argument("cipher")
    d.add_argument("out_plain")
    d.set_defaults(func=cmd_decrypt)

    e = sub.add_parser("encrypt", help="plaintext payload -> cipher")
    e.add_argument("plain")
    e.add_argument("out_cipher")
    e.set_defaults(func=cmd_encrypt)

    args = ap.parse_args()
    args.func(args)

if __name__ == "__main__":
    main()
