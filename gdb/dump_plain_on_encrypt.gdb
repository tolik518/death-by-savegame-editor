# run from game folder with:
# gdb --command=dump_plain_on_encrypt.gdb --args "./Death by Scrolling"
set pagination off
set confirm off
printf "== gdb attached (dump plaintext on encrypt) ==\n"
set follow-fork-mode child
set detach-on-fork on
set breakpoint pending on

python
import gdb, os
OUT="/tmp/dbs_plain"
os.makedirs(OUT, exist_ok=True)
def dump_mem(path, addr, length):
    try:
        mem = gdb.selected_inferior().read_memory(addr, length).tobytes()
        with open(path, "wb") as f: f.write(mem)
        gdb.write(f"[py] wrote {len(mem)} bytes -> {path}\n")
    except Exception as e:
        gdb.write(f"[py] read_memory failed: {e}\n")
end

break btea
commands
  silent
  set $buf = $rdi
  set $n   = $rsi
  set $key = $rdx
  printf "=== btea() === buf=%p n=%ld key=%p\n", $buf, $n, $key
  if ($n > 0)
    set $len = (unsigned long)($n * 4)
    if ($len < 64 || $len > 1<<20)
      set $len = 4096
    end
    python
import gdb, os
buf = int(gdb.parse_and_eval("$buf"))
ln  = int(gdb.parse_and_eval("$len"))
dump_mem("/tmp/dbs_plain/plain_btea.bin", buf, ln)
dump_mem("/tmp/dbs_plain/key_btea.bin",  int(gdb.parse_and_eval("$key")), 16)
    end
  end
  continue
end

break GGQuickCryptData
commands
  silent
  set $src = $rdi
  set $len = $rsi
  set $key = $rdx
  printf "=== GGQuickCryptData() === src=%p len=%#x key=%p\n", $src, $len, $key
  if ($len >= 0x200)   # we only care about larger data blocks (yeah yeah, I know, size doesnt matter, lmao)
    python
import gdb
dump_mem("/tmp/dbs_plain/plain_quick.bin", int(gdb.parse_and_eval("$src")), int(gdb.parse_and_eval("$len")))
dump_mem("/tmp/dbs_plain/key_quick.bin",   int(gdb.parse_and_eval("$key")), 16)
    end
  end
  continue
end

break *0x004eb550
commands
  silent
  printf "=== GGSaveData::decrypt() === rsi=%p key[8]=%u\n", $rsi, ($rsi?*(unsigned char*)($rsi+8):0)
  continue
end

run
