#!/usr/bin/env python3
"""
Measure real shell startup timings for Pack, gem, and Bundler commands.

This script intentionally avoids synthetic parser simulations. For parser
benchmarks, use the Criterion benches under crates/pack-gemfile/benches.
"""

from __future__ import annotations

import statistics
import subprocess
import sys
import time
from pathlib import Path
from shutil import which


COMMANDS = [
    ("pack --version", [["pack"], ["pack.exe"], [str(Path("target/debug/pack.exe"))], [str(Path("target/release/pack.exe"))]], ["--version"]),
    ("bundle --version", [["bundle"], ["bundle.bat"], ["bundle.cmd"]], ["--version"]),
    ("gem --version", [["gem"], ["gem.bat"], ["gem.cmd"]], ["--version"]),
    ("pack list", [["pack"], ["pack.exe"], [str(Path("target/debug/pack.exe"))], [str(Path("target/release/pack.exe"))]], ["list"]),
    ("gem list", [["gem"], ["gem.bat"], ["gem.cmd"]], ["list"]),
]


def resolve_command(candidate_groups: list[list[str]]) -> list[str] | None:
    for candidate_group in candidate_groups:
        program = candidate_group[0]
        if Path(program).exists():
            return candidate_group
        resolved = which(program)
        if resolved:
            return [resolved, *candidate_group[1:]]
    return None


def measure(command: list[str], iterations: int = 10) -> tuple[float, float]:
    timings = []
    for _ in range(iterations):
        start = time.perf_counter()
        if command[0].lower().endswith((".bat", ".cmd")):
            subprocess.run(["cmd", "/c", *command], stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
        else:
            subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
        end = time.perf_counter()
        timings.append((end - start) * 1000)

    return statistics.mean(timings), statistics.median(timings)


def main() -> int:
    print("Pack shell timing benchmark")
    print()
    print("These are real command timings. For parser hot paths, run:")
    print("  cargo bench -p pack-gemfile --bench gemfile_parse")
    print("  cargo bench -p pack-gemfile --bench lockfile_bench")
    print()
    print(f"{'Command':<22} {'Mean (ms)':>10} {'Median (ms)':>12}")
    print("-" * 48)

    for label, candidate_groups, args in COMMANDS:
        resolved = resolve_command(candidate_groups)
        if not resolved:
            print(f"{label:<22} {'missing':>10} {'missing':>12}")
            continue
        try:
            mean_ms, median_ms = measure([*resolved, *args])
        except FileNotFoundError:
            print(f"{label:<22} {'missing':>10} {'missing':>12}")
            continue
        print(f"{label:<22} {mean_ms:>10.2f} {median_ms:>12.2f}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
