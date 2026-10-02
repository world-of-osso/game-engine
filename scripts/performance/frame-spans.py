#!/usr/bin/env python3
"""Mean main-thread ms per frame of each GAME_PROFILE_MS span, per frame_benchmark segment.

Usage: frame-spans.py LOG [TOP]
Run the benchmark with GAME_PROFILE_MS=0 so every span prints. Spans nest (a step span
contains its sub-spans), so only same-level labels add up.
"""

import collections
import json
import re
import sys


def main():
    path, top = sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 25
    segment, totals, frames = None, collections.defaultdict(collections.Counter), {}
    for line in open(path, errors="replace"):
        mark = re.match(r"BENCH_MARK (\w+) (start|end)", line)
        if mark:
            segment = mark[1] if mark[2] == "start" else None
            continue
        if line.startswith("BENCH_SEGMENT "):
            report = json.loads(line.split(" ", 1)[1])
            frames[report["segment"]] = report["frames"]
            continue
        span = re.match(r"PROFILE (.+) ms=([\d.]+)$", line.strip())
        if span and segment:
            totals[segment][span[1]] += float(span[2])
    for name, counter in totals.items():
        count = frames.get(name) or 1
        print(f"== {name}: {count} frames")
        for label, total in counter.most_common(top):
            print(f"{total / count:8.2f} ms  {label}")


if __name__ == "__main__":
    main()
