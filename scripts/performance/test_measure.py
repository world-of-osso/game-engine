"""Lightweight behavioral tests; synthetic records are NOT runtime evidence."""

import importlib.util
import json
from pathlib import Path
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

    def test_short_or_inconsistent_sample_window_is_not_adequate(self):
        report = {"settled_elapsed_s": 60, "settled_ms": [10] * 120}
        self.assertFalse(self.measure.summarize(report)["adequate_duration"])
        report = {"settled_elapsed_s": 1, "settled_ms": [1000] * 60}
        self.assertFalse(self.measure.summarize(report)["adequate_duration"])


if __name__ == "__main__":
    unittest.main()
