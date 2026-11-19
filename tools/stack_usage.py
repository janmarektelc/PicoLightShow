#!/usr/bin/env python3
"""Worst-case stack usage per Embassy task, estimated from the disassembly.

    python tools/stack_usage.py [path/to/elf]

Sums the stack frames along direct calls (`bl`) starting at every
`TaskStorage<...>::poll`. Indirect calls are not followed, so treat the
numbers as a lower bound. Core 0 has roughly 70 KiB of stack, core 1 16 KiB.
"""
import re
import subprocess
import sys
from functools import lru_cache

elf = sys.argv[1] if len(sys.argv) > 1 else "firmware/target/thumbv6m-none-eabi/release/pico-light-show"
dis = subprocess.run(["rust-objdump", "-d", "--no-show-raw-insn", "-C", elf], capture_output=True, text=True,
                     encoding="utf-8", errors="replace").stdout.splitlines()
raw = subprocess.run(["rust-objdump", "-d", elf], capture_output=True, text=True,
                     encoding="utf-8", errors="replace").stdout.splitlines()

words = {}
for l in raw:
    m = re.match(r"\s*([0-9a-f]+):\s+(?:[0-9a-f]{2} ){4}\s*\.word\s+0x([0-9a-f]+)", l)
    if m:
        words[int(m.group(1), 16)] = int(m.group(2), 16)

frame, calls, names = {}, {}, {}
func = None
regs = {}
for l in dis:
    m = re.match(r"^([0-9a-f]+) <(.*)>:$", l)
    if m:
        func = int(m.group(1), 16)
        names[func] = m.group(2)
        frame[func] = 0
        calls[func] = set()
        regs = {}
        continue
    if func is None:
        continue
    m = re.match(r"\s*[0-9a-f]+:\s+push\s+\{(.*)\}", l)
    if m and frame[func] == 0:
        frame[func] += 4 * len(m.group(1).split(","))
    m = re.match(r"\s*[0-9a-f]+:\s+sub\s+sp, #0x([0-9a-f]+)", l)
    if m:
        frame[func] += int(m.group(1), 16)
    m = re.match(r"\s*[0-9a-f]+:\s+ldr\s+(r\d+), \[pc, #0x[0-9a-f]+\]\s+@ 0x([0-9a-f]+)", l)
    if m:
        regs[m.group(1)] = words.get(int(m.group(2), 16))
    m = re.match(r"\s*[0-9a-f]+:\s+add\s+sp, (r\d+)\s*$", l)
    if m and regs.get(m.group(1)) is not None and regs[m.group(1)] >= 0x80000000:
        frame[func] += 0x100000000 - regs[m.group(1)]
    m = re.match(r"\s*[0-9a-f]+:\s+bl\s+0x([0-9a-f]+)", l)
    if m:
        calls[func].add(int(m.group(1), 16))

sys.setrecursionlimit(100000)
visiting = set()


@lru_cache(maxsize=None)
def worst(f):
    if f in visiting:
        return 0
    visiting.add(f)
    best = max((worst(c) for c in calls.get(f, ()) if c in frame), default=0)
    visiting.discard(f)
    return frame.get(f, 0) + best


rows = []
for f, n in names.items():
    if "TaskStorage<" in n and ">::poll" in n:
        task = re.sub(r".*?__(\w+?)_task_task.*", r"\1", n)
        rows.append((worst(f), task))
for s, t in sorted(rows, reverse=True):
    print(f"{s:7d} B  {t}")
