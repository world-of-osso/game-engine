"""Real process-lifetime regression: unconsumed pipes must not stall a worker."""
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

LAUNCHER = Path(__file__).resolve().parents[1] / "agent" / "logged-process.py"
PAYLOAD_BYTES = 1_048_576


class LoggedProcessTests(unittest.TestCase):
    def test_large_output_does_not_block_with_unconsumed_parent_pipes(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "worker.log"
            worker = (
                "import os; os.write(1, b'x'*1048576); "
                "os.write(2, b'y'*1048576); os.write(2, b'READY\\n')"
            )
            process = subprocess.Popen(
                [sys.executable, str(LAUNCHER), str(log), sys.executable, "-c", worker],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            try:
                # Do not drain while alive: exactly the failed server boundary.
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                self.fail("Worker stalled behind unconsumed pipes before READY")
            finally:
                if process.poll() is None:
                    process.kill()
                process.communicate()
            self.assertEqual(process.returncode, 0)
            expected = b"x" * PAYLOAD_BYTES + b"y" * PAYLOAD_BYTES + b"READY\n"
            self.assertEqual(log.read_bytes(), expected)

    def test_preserves_failure_exit_code_and_logs_error(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "failure.log"
            worker = "import sys; print('failed', file=sys.stderr); sys.exit(17)"
            result = subprocess.run(
                [sys.executable, str(LAUNCHER), str(log), sys.executable, "-c", worker],
                capture_output=True,
                timeout=3,
            )
            self.assertEqual(result.returncode, 17)
            self.assertTrue(log.exists(), "Worker stderr was not logged")
            self.assertEqual(log.read_text(), "failed\n")
            self.assertEqual(result.stdout, b"")
            self.assertEqual(result.stderr, b"")


if __name__ == "__main__":
    unittest.main()
