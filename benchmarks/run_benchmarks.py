#!/usr/bin/env python3
import os
import sys
import time
import shutil
import subprocess
import json

CONFIGS = [
    {
        "name": "C (GCC 16.1)",
        "lang": "C",
        "compiler": "gcc",
        "build_cmd": lambda src, out: ["gcc", "-O3", "-fomit-frame-pointer", "-march=native", "-ffp-contract=off", src, "-o", out, "-lm"],
        "src_pattern": "benchmarks/c/{bench}.c",
        "out_pattern": "benchmarks/c/{bench}_gcc",
    },
    {
        "name": "C (Clang 22.1)",
        "lang": "C",
        "compiler": "clang",
        "build_cmd": lambda src, out: ["clang", "-O3", "-fomit-frame-pointer", "-march=native", "-ffp-contract=off", src, "-o", out, "-lm"],
        "src_pattern": "benchmarks/c/{bench}.c",
        "out_pattern": "benchmarks/c/{bench}_clang",
    },
    {
        "name": "C++ (G++ 16.1)",
        "lang": "C++",
        "compiler": "g++",
        "build_cmd": lambda src, out: ["g++", "-O3", "-fomit-frame-pointer", "-march=native", "-ffp-contract=off", src, "-o", out, "-lm"],
        "src_pattern": "benchmarks/cpp/{bench}.cpp",
        "out_pattern": "benchmarks/cpp/{bench}_gpp",
    },
    {
        "name": "C++ (Clang++ 22.1)",
        "lang": "C++",
        "compiler": "clang++",
        "build_cmd": lambda src, out: ["clang++", "-O3", "-fomit-frame-pointer", "-march=native", "-ffp-contract=off", src, "-o", out, "-lm"],
        "src_pattern": "benchmarks/cpp/{bench}.cpp",
        "out_pattern": "benchmarks/cpp/{bench}_clangpp",
    },
    {
        "name": "Rust (rustc 1.97)",
        "lang": "Rust",
        "compiler": "rustc",
        "build_cmd": lambda src, out: ["rustc", "-C", "opt-level=3", "-C", "target-cpu=native", src, "-o", out],
        "src_pattern": "benchmarks/rust/{bench}.rs",
        "out_pattern": "benchmarks/rust/{bench}_rust",
    },
    {
        "name": "Zig (0.14.0)",
        "lang": "Zig",
        "compiler": "zig",
        "build_cmd": lambda src, out: ["zig", "build-exe", "-O", "ReleaseFast", src, f"-femit-bin={out}"],
        "src_pattern": "benchmarks/zig/{bench}.zig",
        "out_pattern": "benchmarks/zig/{bench}_zig",
    },
    {
        "name": "Odin (dev-2026-07)",
        "lang": "Odin",
        "compiler": "odin",
        "build_cmd": lambda src, out: ["odin", "build", src, "-file", "-o:speed", f"-out:{out}"],
        "src_pattern": "benchmarks/odin/{bench}.odin",
        "out_pattern": "benchmarks/odin/{bench}_odin",
    },
    {
        "name": "Go (1.26.5)",
        "lang": "Go",
        "compiler": "go",
        "build_cmd": lambda src, out: ["go", "build", "-ldflags=-s -w", "-o", out, src],
        "src_pattern": "benchmarks/go/{bench}.go",
        "out_pattern": "benchmarks/go/{bench}_go",
    },
    {
        "name": "V (0.5.2)",
        "lang": "V",
        "compiler": "v",
        "build_cmd": lambda src, out: ["v", "-prod", "-cflags", "-O3 -march=native", "-o", out, src],
        "src_pattern": "benchmarks/v/{bench}.v",
        "out_pattern": "benchmarks/v/{bench}_v",
    },
    {
        "name": "Nim (2.2.10)",
        "lang": "Nim",
        "compiler": "nim",
        "build_cmd": lambda src, out: ["nim", "c", "-d:release", "--opt:speed", "--threads:off", "--verbosity:0", f"--out:{out}", src],
        "src_pattern": "benchmarks/nim/{bench}.nim",
        "out_pattern": "benchmarks/nim/{bench}_nim",
    },
    {
        "name": "D (LDC2 1.42)",
        "lang": "D",
        "compiler": "ldc2",
        "build_cmd": lambda src, out: ["ldc2", "-O3", "-release", "-boundscheck=off", f"-of={out}", src],
        "src_pattern": "benchmarks/d/{bench}.d",
        "out_pattern": "benchmarks/d/{bench}_ldc",
    },
    {
        "name": "Cez (0.1.0 / LLVM 22)",
        "lang": "Cez",
        "compiler": "cez",
        "build_cmd": lambda src, out: ["cez", "build", "-O3", src, "-o", out],
        "src_pattern": "benchmarks/cez/{bench}.cez",
        "out_pattern": "benchmarks/cez/{bench}_cez",
    },
]

BENCHMARKS = [
    {
        "id": "fib",
        "name": "Recursive Fibonacci (fib 40)",
        "expected": "102334155",
        "description": "331M recursive function calls, tests prologue/epilogue overhead, register saving & stack frame management.",
    },
    {
        "id": "sieve",
        "name": "Prime Sieve (10M limit, 5 runs)",
        "expected": "664579",
        "description": "Sequential buffer indexing, tight loops, bit/byte operations, cache memory throughput.",
    },
    {
        "id": "mandel",
        "name": "Mandelbrot Escape (600x600, max 200 iter)",
        "expected": "20140609",
        "description": "Intensive f64 double-precision floating point arithmetic, loops & branch execution.",
    },
]

def measure_compile(cfg, bench_id):
    src = cfg["src_pattern"].format(bench=bench_id)
    out = cfg["out_pattern"].format(bench=bench_id)
    cmd = cfg["build_cmd"](src, out)

    if os.path.exists(out):
        os.remove(out)

    times = []
    for _ in range(3):
        if os.path.exists(out):
            os.remove(out)
        t0 = time.perf_counter()
        res = subprocess.run(cmd, capture_output=True, text=True)
        t1 = time.perf_counter()
        if res.returncode != 0:
            print(f"FAILED to build {cfg['name']} for {bench_id}: {res.stderr}", file=sys.stderr)
            return None, None
        times.append(t1 - t0)

    median_time = sorted(times)[1]
    
    # Measure stripped binary size
    stripped_out = out + ".stripped"
    shutil.copyfile(out, stripped_out)
    subprocess.run(["strip", "--strip-all", stripped_out], capture_output=True)
    size_kb = os.path.getsize(stripped_out) / 1024.0
    if os.path.exists(stripped_out):
        os.remove(stripped_out)

    return median_time, size_kb

def measure_runtime(cfg, bench_id, expected):
    out = cfg["out_pattern"].format(bench=bench_id)
    if not os.path.exists(out):
        return None

    # Warmup run + verify
    res = subprocess.run([out], capture_output=True, text=True)
    if res.returncode != 0 or res.stdout.strip() != expected:
        print(f"FAILED execution or mismatch in {cfg['name']} for {bench_id}: code={res.returncode}, got={res.stdout.strip()}, expected={expected}", file=sys.stderr)
        return None

    times = []
    # 5 runs, discard highest & lowest, average middle 3
    for _ in range(5):
        t0 = time.perf_counter()
        res = subprocess.run([out], capture_output=True, text=True)
        t1 = time.perf_counter()
        times.append(t1 - t0)

    sorted_times = sorted(times)
    avg_time = sum(sorted_times[1:4]) / 3.0
    return avg_time

def main():
    print("=" * 80)
    print("BENCHMARK SUITE: 10 Systems Languages / 12 Toolchains")
    print("Machine: 12th Gen Intel(R) Core(TM) i5-12450H (Arch Linux x86_64)")
    print("=" * 80)

    results = []

    for cfg in CONFIGS:
        print(f"\n---> Benchmarking {cfg['name']}...")
        cfg_result = {
            "name": cfg["name"],
            "lang": cfg["lang"],
            "benchmarks": {}
        }

        for b in BENCHMARKS:
            bench_id = b["id"]
            print(f"  [{bench_id}] Compiling...", end="", flush=True)
            ctime, size_kb = measure_compile(cfg, bench_id)
            if ctime is None:
                print(" FAILED")
                continue
            print(f" {ctime:.3f}s, size: {size_kb:.1f} KB. Running...", end="", flush=True)
            rtime = measure_runtime(cfg, bench_id, b["expected"])
            if rtime is None:
                print(" FAILED")
                continue
            print(f" {rtime:.3f}s")
            cfg_result["benchmarks"][bench_id] = {
                "compile_time_s": ctime,
                "binary_size_kb": size_kb,
                "runtime_s": rtime,
            }

        results.append(cfg_result)

    # Save to JSON
    with open("benchmarks/benchmark_results.json", "w") as f:
        json.dump(results, f, indent=2)

    print("\nSaved raw data to benchmarks/benchmark_results.json")
    print("\n" + "=" * 80)
    print("SUMMARY TABLES")
    print("=" * 80)

    for b in BENCHMARKS:
        bid = b["id"]
        print(f"\n### Benchmark: {b['name']}")
        print(f"*{b['description']}*\n")
        print("| Язык / Компилятор | Время исполнения (сек) | Время сборки (сек) | Размер бинарника (KB) |")
        print("|---|:---:|:---:|:---:|")

        valid_items = [c for c in results if bid in c["benchmarks"]]
        valid_items.sort(key=lambda x: x["benchmarks"][bid]["runtime_s"])

        for c in valid_items:
            m = c["benchmarks"][bid]
            print(f"| **{c['name']}** | **{m['runtime_s']:.4f}s** | {m['compile_time_s']:.3f}s | {m['binary_size_kb']:.1f} KB |")

if __name__ == "__main__":
    main()
