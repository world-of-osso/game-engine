#!/usr/bin/env python3
"""Bounded foreground workload recorder. MAIN alone runs real clients.

No dependencies. Use: uv run --no-project python scripts/performance/measure.py
 --manifest manifest.json --output NEW_DIRECTORY --timeout 1100 -- <argv...>
Command must keep cage/Godot in foreground, under agent-run. No server management.
"""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import selectors
import signal
import subprocess
import time


COMPARISON_KEYS = (
    "workload",
    "assets",
    "options",
    "cache",
    "hardware_driver",
    "resolution",
    "renderer",
    "server_state",
    "camera",
    "sampling",
    "resource_limits",
)


def distribution(values, limit=100.0):
    if not math.isfinite(limit) or limit <= 0:
        raise ValueError("fixture limit must be finite and positive")
    if any(not math.isfinite(value) or value < 0 for value in values):
        raise ValueError("frame intervals must be finite and nonnegative")
    ordered = sorted(values)

    def percentile(fraction):
        return (
            ordered[max(0, math.ceil(len(ordered) * fraction) - 1)] if ordered else None
        )

    return {
        "samples": len(values),
        "interval_sum_s": sum(values) / 1000,
        "p50_ms": percentile(0.5),
        "p95_ms": percentile(0.95),
        "p99_ms": percentile(0.99),
        "max_ms": max(values) if values else None,
        "fixture_limit_ms": limit,
        "over_fixture_limit": sum(value > limit for value in values),
    }


def summarize(report):
    settled = distribution(
        report.get("settled_ms", []), report.get("frame_limit_ms", 100)
    )
    elapsed = report.get("settled_elapsed_s", 0)
    memory = report.get("memory", {})
    memory_gaps = [
        phase
        for phase in (
            "script_start",
            "client_mounted",
            "character_selected",
            "enter_world",
            "loading_hidden",
            "queue_drained",
            "settled_end",
        )
        if phase not in memory
        or memory[phase].get("error")
        or any(memory[phase].get(key) is None for key in ("VmRSS_kib", "VmHWM_kib"))
    ]
    return {
        "startup": distribution(report.get("startup_ms", [])),
        "loading": distribution(
            report.get("loading_ms", []), report.get("loading_limit_ms", 1000)
        ),
        "transition": distribution(
            report.get("transition_ms", []), report.get("frame_limit_ms", 100)
        ),
        "settled": settled,
        "loading_s": report.get("loading_s"),
        "queue_drain_s": report.get("queue_drain_s"),
        "settled_elapsed_s": elapsed,
        "adequate_duration": 60 <= elapsed <= 301
        and 60 <= settled["interval_sum_s"] <= 301
        and settled["samples"] >= 2,
        "memory": memory,
        "memory_gaps": memory_gaps,
        "settled_queue_stable": report.get("queue_drained", False)
        and not report.get("settled_pending_changed", False)
        and report.get("objects", {}).get("pending") == 0,
        "percentile_scope": "Empirical nearest-rank only; sample count reported, no statistical tail-stability claim",
        "objects": report.get("objects", {}),
        "limits_scope": "fixture policy only; no product budget or parity assertion",
    }


def compare(manifest, baseline):
    current = manifest.get("comparison", {})
    previous = baseline.get("comparison", {}) if baseline is not None else {}
    gaps = [
        key
        for key in COMPARISON_KEYS
        if not current.get(key) or current.get(key) != previous.get(key)
    ]
    return {
        "status": "gap" if baseline is None or gaps else "matching_declared_inputs",
        "gaps": gaps,
        "note": "No retained baseline supplied"
        if baseline is None
        else "Declared input match is not independently verified parity",
    }


def capture_inputs(manifest):
    snapshots = {}
    for name, filename in manifest.get("input_paths", {}).items():
        path = Path(filename).resolve()
        digest = hashlib.sha256()
        with path.open("rb") as source:
            for block in iter(lambda: source.read(1024 * 1024), b""):
                digest.update(block)
        snapshots[name] = {
            "path": str(path),
            "sha256": digest.hexdigest(),
            "bytes": path.stat().st_size,
        }
    return snapshots


def run_measurement(command, output, manifest, timeout):
    output.mkdir(parents=True, exist_ok=True)
    phases, reports, parse_errors = [], [], []
    started = time.monotonic()
    deadline = started + timeout
    pending = b""
    timed_out = False

    def consume(line):
        text = line.decode("utf-8", errors="replace")
        for prefix, records in (("PERF_PHASE ", phases), ("PERF_REPORT ", reports)):
            if text.startswith(prefix):
                try:
                    value = json.loads(text[len(prefix) :])
                    if not isinstance(value, dict):
                        raise ValueError("record must be a JSON object")
                    if prefix == "PERF_PHASE ":
                        value["since_launch_s"] = time.monotonic() - started
                    records.append(value)
                except (ValueError, TypeError) as error:
                    parse_errors.append(str(error))

    with (output / "stdout.log").open("wb") as log:
        process = subprocess.Popen(
            command,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )
        with selectors.DefaultSelector() as selector:
            selector.register(process.stdout, selectors.EVENT_READ)
            while selector.get_map():
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    timed_out = True
                    os.killpg(process.pid, signal.SIGKILL)
                    break
                for key, _ in selector.select(min(remaining, 0.2)):
                    block = os.read(key.fd, 65536)
                    if not block:
                        selector.unregister(key.fileobj)
                        continue
                    log.write(block)
                    log.flush()
                    pending += block
                    while b"\n" in pending:
                        line, pending = pending.split(b"\n", 1)
                        consume(line)
        if timed_out:
            tail, _ = process.communicate(timeout=5)
            log.write(tail)
            pending += tail
        else:
            try:
                process.wait(timeout=max(0.01, deadline - time.monotonic()))
            except subprocess.TimeoutExpired:
                timed_out = True
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=5)
        for line in pending.splitlines():
            consume(line)
        process.stdout.close()
    gaps = list(parse_errors)
    measurement = None
    if len(reports) != 1:
        gaps.append(
            "missing PERF_REPORT" if not reports else "multiple PERF_REPORT records"
        )
    else:
        try:
            measurement = summarize(reports[0])
        except (ValueError, TypeError, AttributeError) as error:
            gaps.append("invalid PERF_REPORT: " + str(error))
        if measurement is not None:
            if not measurement["adequate_duration"]:
                gaps.append(
                    "settled sample window is not 60–300 seconds; not final acceptance evidence"
                )
            if measurement["memory_gaps"]:
                gaps.append(
                    "missing/invalid phase memory: "
                    + ", ".join(measurement["memory_gaps"])
                )
            if not measurement["settled_queue_stable"]:
                gaps.append(
                    "readiness or parsed tile set not continuously settled during observation"
                )
    return {
        "command": command,
        "manifest": manifest,
        "exit_code": process.returncode,
        "timed_out": timed_out,
        "wall_s": time.monotonic() - started,
        "phases": phases,
        "raw_reports": reports,
        "measurement": measurement,
        "gaps": gaps,
        "log_streams": "stdout and stderr combined in stdout.log",
        "proof_limits": "Wall-clock process-frame intervals, not GPU/presentation time. Phase VmHWM is lifetime high-water, not phase allocation. No leak, shutdown or performance parity claim. Timeout cleanup is SIGKILL.",
    }


def git_record(repo):
    result = {}
    for name, args in (
        ("observed_head", ["rev-parse", "HEAD"]),
        ("status", ["status", "--short"]),
        ("tracked_diff", ["diff", "HEAD"]),
    ):
        record = subprocess.run(
            ["git", "-C", str(repo), *args], capture_output=True, text=True, check=True
        )
        result[name] = record.stdout
    result["scope"] = (
        "Checkout observation only; not root binary/native build provenance. Concurrent/dirty sources remain explicit."
    )
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--baseline", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout", type=float, default=1100)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if not 1 <= args.timeout <= 1500:
        parser.error("timeout must be 1–1500 seconds")
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("foreground workload argv required")
    manifest = json.loads(args.manifest.read_text())
    baseline = json.loads(args.baseline.read_text()) if args.baseline else None
    args.output.mkdir(parents=True, exist_ok=False)
    before = capture_inputs(manifest)
    repo = Path(__file__).resolve().parents[2]
    provenance = git_record(repo)
    result = run_measurement(command, args.output, manifest, args.timeout)
    result["inputs_before"] = before
    result["inputs_after"] = capture_inputs(manifest)
    result["checkout_before"] = provenance
    result["checkout_after"] = git_record(repo)
    result["host"] = {"platform": platform.platform(), "uname": list(platform.uname())}
    result["environment"] = {
        name: os.environ.get(name)
        for name in (
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_CACHE_HOME",
            "WAYLAND_DISPLAY",
            "DISPLAY",
            "WLR_BACKENDS",
            "WLR_RENDERER",
            "WLR_LIBINPUT_NO_DEVICES",
            "GODOT_BIN",
            "GODOT_TEST_SERVER",
            "WORLD_ENTRY_ACCOUNT",
            "WORLD_ENTRY_CHARACTER",
            "WORLD_ENTRY_MEASURE_S",
            "WORLD_ENTRY_SETTLE_S",
            "WORLD_ENTRY_FRAME_MS",
            "WORLD_ENTRY_LOADING_FRAME_MS",
        )
    }
    result["comparison"] = compare(manifest, baseline)
    if before != result["inputs_after"]:
        result["gaps"].append("frozen input files changed during workload")
        result["comparison"]["status"] = "gap"
    if provenance != result["checkout_after"]:
        result["gaps"].append(
            "checkout changed during workload; artifact build provenance must be independently established"
        )
    if not manifest.get("input_paths"):
        result["gaps"].append("no frozen input file hashes supplied")
    if not manifest.get("build_provenance"):
        result["gaps"].append("no independently supplied root/native build provenance")
    (args.output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(
        json.dumps(
            {
                "output": str(args.output),
                "exit_code": result["exit_code"],
                "gaps": result["gaps"],
                "comparison": result["comparison"],
            }
        )
    )
    return 1 if result["timed_out"] or result["exit_code"] != 0 or result["gaps"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
