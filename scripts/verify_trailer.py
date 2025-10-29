#!/usr/bin/env python3
import sys, struct

fn = sys.argv[1] if len(sys.argv) > 1 else "/tmp/dbs_plain/plain_btea.bin"
data = open(fn, "rb").read()
padlen = data[-1]
if not (0 <= padlen <= 8):
    print(f"[!] padlen out of range: {padlen} (last byte)"); sys.exit(1)

payload_len = len(data) - padlen - 9
checksum_stored = struct.unpack_from("<I", data, payload_len)[0]
extra4 = struct.unpack_from("<I", data, payload_len + 4)[0]
payload = data[:payload_len]

checksum_calc = (0x06583463 + sum(payload)) & 0xFFFFFFFF

print(f"[ok] total_len={len(data)} padlen={padlen} payload_len={payload_len}")
print(f"[ok] checksum_stored=0x{checksum_stored:08x}  checksum_calc=0x{checksum_calc:08x}")
print(f"[info] extra4=0x{extra4:08x}")
print(f"[tail] last 16 bytes: {data[-16:].hex(' ')}")
print(f"[tail] footer raw   : {data[payload_len:payload_len+9].hex(' ')}")
