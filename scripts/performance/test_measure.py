"""Lightweight behavioral tests; synthetic records are NOT runtime evidence."""

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


PATH = Path(__file__).with_name("measure.py")


class MeasurementTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("measure", PATH)
        cls.measure = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.measure)

    def test_frame_distribution_keeps_stalls_and_uses_nearest_rank(self):
        result = self.measure.distribution([10.0] * 99 + [1000.0])
        self.assertEqual(result["samples"], 100)
        self.assertEqual(result["p99_ms"], 10)
        self.assertEqual(result["max_ms"], 1000)
        self.assertEqual(result["interval_sum_s"], 1.99)
        self.assertEqual(result["over_fixture_limit"], 1)
        self.assertEqual(self.measure.distribution([])["p99_ms"], None)

    def test_invalid_frame_data_is_rejected(self):
        for values in [[-1], [float("nan")], [float("inf")]]:
            with self.subTest(values=values), self.assertRaises(ValueError):
                self.measure.distribution(values)

    def test_comparison_requires_frozen_inputs_and_not_matching_builds(self):
        native = {"comparison": {"workload": "northshire", "options": "sha256:a"}}
        baseline = {"comparison": {"workload": "northshire", "options": "sha256:b"}}
        self.assertEqual(self.measure.compare(native, None)["status"], "gap")
        self.assertEqual(self.measure.compare(native, baseline)["status"], "gap")
        native["comparison"] = {key: "concrete" for key in self.measure.COMPARISON_KEYS}
        baseline["comparison"] = dict(native["comparison"])
        self.assertEqual(
            self.measure.compare(native, baseline)["status"], "matching_declared_inputs"
        )
        baseline["comparison"]["cache"] = "different"
        self.assertIn("cache", self.measure.compare(native, baseline)["gaps"])

    def test_real_child_logs_phases_report_and_nonzero_exit(self):
        report = {
            "loading_s": 2.0,
            "queue_drain_s": 3.0,
            "settled_elapsed_s": 60.0,
            "startup_ms": [20.0],
            "loading_ms": [2000.0],
            "transition_ms": [3000.0],
            "settled_ms": [1000.0] * 60,
            "frame_limit_ms": 100.0,
            "memory": {"settled_end": {"VmRSS_kib": 123, "VmHWM_kib": 234}},
            "objects": {"pending": 0},
        }
        child = (
            "import json; print('PERF_PHASE '+json.dumps({'phase':'script_start'})); print('PERF_REPORT '+"
            + repr(json.dumps(report))
            + "); raise SystemExit(3)"
        )
        with tempfile.TemporaryDirectory() as tmp:
            result = self.measure.run_measurement(
                [sys.executable, "-c", child], Path(tmp), {}, 5
            )
            self.assertEqual(result["exit_code"], 3)
            self.assertFalse(result["timed_out"])
            self.assertTrue((Path(tmp) / "stdout.log").is_file())
            self.assertEqual(result["measurement"]["loading"]["max_ms"], 2000)
            self.assertEqual(result["measurement"]["settled"]["samples"], 60)
            self.assertTrue(result["measurement"]["adequate_duration"])
            self.assertEqual(
                result["measurement"]["memory"]["settled_end"]["VmRSS_kib"], 123
            )
            self.assertGreaterEqual(result["phases"][0]["since_launch_s"], 0)

    def dynamic_queue_report(self):
        return {
            "settled_elapsed_s": 60.0,
            "settled_ms": [1000.0] * 60,
            "frame_limit_ms": 2000.0,
            "queue_drained": True,
            "settled_pending_changed": True,
            "objects": {"pending": 0},
            "workload_snapshots": {
                "first_readiness_change": {
                    "terrain": {"pending_count": 0, "parsed_tiles": [[1, 2]]},
                    "world_objects": {"pending": 0},
                    "unit_visuals_pending": 1,
                    "observed_process_frame": 42,
                    "observed_ticks_usec": 123456,
                }
            },
            "memory": {
                phase: {"VmRSS_kib": 123, "VmHWM_kib": 234}
                for phase in (
                    "script_start",
                    "client_mounted",
                    "character_selected",
                    "enter_world",
                    "loading_hidden",
                    "queue_drained",
                    "settled_end",
                )
            },
        }

    def run_cli_child(self, child):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest = root / "manifest.json"
            options = root / "options.json"
            options.write_text('{"vsync":false}')
            manifest.write_text(
                json.dumps(
                    {
                        "input_paths": {"options": str(options)},
                        "build_provenance": "synthetic Python child; no native runtime claim",
                    }
                )
            )
            output = root / "output"
            runner = subprocess.run(
                [
                    sys.executable,
                    str(PATH),
                    "--manifest",
                    str(manifest),
                    "--output",
                    str(output),
                    "--timeout",
                    "5",
                    "--",
                    sys.executable,
                    "-c",
                    child,
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            result = json.loads((output / "result.json").read_text())
            return runner, result, (output / "stdout.log").read_text()

    def test_cli_accepts_dynamic_queues_and_preserves_workload_context(self):
        report = self.dynamic_queue_report()
        child = "print(" + repr("PERF_REPORT " + json.dumps(report)) + ")"
        runner, result, log = self.run_cli_child(child)
        self.assertEqual(runner.returncode, 0, runner.stdout + runner.stderr)
        self.assertEqual(result["exit_code"], 0)
        self.assertTrue(result["measurement"]["adequate_duration"])
        self.assertFalse(result["measurement"]["settled_queue_stable"])
        self.assertEqual(result["measurement"]["settled"]["max_ms"], 1000)
        self.assertEqual(result["measurement"]["settled"]["over_fixture_limit"], 0)
        self.assertEqual(result["measurement"]["memory_gaps"], [])
        self.assertEqual(result["gaps"], [])
        self.assertEqual(result["raw_reports"], [report])
        self.assertEqual(json.loads(log.removeprefix("PERF_REPORT ")), report)

    def test_cli_rejects_missing_short_reports_and_nonzero_child(self):
        short = self.dynamic_queue_report()
        short["settled_elapsed_s"] = 59.0
        short["settled_ms"] = [1000.0] * 59
        adequate = self.dynamic_queue_report()
        cases = (
            ("missing", "print('no report')", 0, "missing PERF_REPORT"),
            (
                "short",
                "print(" + repr("PERF_REPORT " + json.dumps(short)) + ")",
                0,
                "settled sample window is not 60–300 seconds; not final acceptance evidence",
            ),
            (
                "nonzero",
                "print(" + repr("PERF_REPORT " + json.dumps(adequate))
                + "); raise SystemExit(3)",
                3,
                None,
            ),
        )
        for name, child, exit_code, gap in cases:
            with self.subTest(case=name):
                runner, result, _ = self.run_cli_child(child)
                self.assertEqual(runner.returncode, 1, runner.stdout + runner.stderr)
                self.assertEqual(result["exit_code"], exit_code)
                self.assertFalse(result["timed_out"])
                if gap is not None:
                    self.assertEqual(result["gaps"], [gap])
                else:
                    self.assertEqual(result["gaps"], [])
                    self.assertTrue(result["measurement"]["adequate_duration"])
                    self.assertEqual(result["measurement"]["memory_gaps"], [])
                    self.assertEqual(result["raw_reports"], [adequate])

    def test_timeout_and_missing_report_remain_failures(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = self.measure.run_measurement(
                [sys.executable, "-c", "import time; time.sleep(10)"],
                Path(tmp),
                {},
                0.1,
            )
            self.assertTrue(result["timed_out"])
            self.assertIsNone(result["measurement"])
            self.assertIn("missing PERF_REPORT", result["gaps"])

    def test_file_snapshots_detect_changed_options(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "options.json"
            path.write_text('{"vsync":true}')
            manifest = {"input_paths": {"options": str(path)}}
            before = self.measure.capture_inputs(manifest)
            path.write_text('{"vsync":false}')
            after = self.measure.capture_inputs(manifest)
            self.assertNotEqual(before["options"]["sha256"], after["options"]["sha256"])
            self.assertEqual(after["options"]["bytes"], path.stat().st_size)

    def test_malformed_measurement_preserves_raw_log_and_reports_gap(self):
        child = "print('PERF_REPORT {\\\"settled_ms\\\":[-1]}')"
        with tempfile.TemporaryDirectory() as tmp:
            result = self.measure.run_measurement(
                [sys.executable, "-c", child], Path(tmp), {}, 5
            )
            self.assertEqual(result["exit_code"], 0)
            self.assertIsNone(result["measurement"])
            self.assertTrue(result["gaps"])
            self.assertEqual(result["raw_reports"], [{"settled_ms": [-1]}])
            self.assertEqual(
                (Path(tmp) / "stdout.log").read_text(),
                'PERF_REPORT {"settled_ms":[-1]}\n',
            )

    def test_memory_gaps_and_queue_change_remain_explicit(self):
        report = {
            "settled_elapsed_s": 60,
            "settled_ms": [1000] * 60,
            "settled_pending_changed": True,
            "memory": {
                "settled_end": {"VmRSS_kib": None, "VmHWM_kib": None, "error": "denied"}
            },
        }
        result = self.measure.summarize(report)
        self.assertFalse(result["settled_queue_stable"])
        self.assertIn("settled_end", result["memory_gaps"])

    def test_short_or_inconsistent_sample_window_is_not_adequate(self):
        report = {"settled_elapsed_s": 60, "settled_ms": [10] * 120}
        self.assertFalse(self.measure.summarize(report)["adequate_duration"])
        report = {"settled_elapsed_s": 1, "settled_ms": [1000] * 60}
        self.assertFalse(self.measure.summarize(report)["adequate_duration"])


if __name__ == "__main__":
    unittest.main()
