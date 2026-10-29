import os
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

AGENT_RUN = Path(__file__).resolve().parents[1] / "agent" / "agent-run"


def write_script(path: Path, body: str) -> None:
    path.write_text("#!/bin/bash\n" + body)
    path.chmod(path.stat().st_mode | stat.S_IXUSR)


class AgentRunCapacityTest(unittest.TestCase):
    def test_granted_add_starts_cargo_after_one_check(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            calls = root / "capacity-calls"
            # The verdict line comes after bulk output, as capacity.sh's report does, so a reader
            # that stops at the first match leaves the writer with unread data (the SIGPIPE race).
            write_script(
                root / "capacity",
                f'echo x >> "{calls}"\n'
                'for i in $(seq 2000); do echo "agents 1.0/32 CPU padding line $i"; done\n'
                'echo "agents 1.0/32 CPU | ADD: ok"\n'
                'for i in $(seq 2000); do echo "trailing $i"; done\n',
            )
            bin_dir = root / "bin"
            bin_dir.mkdir()
            write_script(bin_dir / "systemd-run", 'echo "started: ${@: -1}"\n')
            env = dict(os.environ, AGENT_RUN_CAPACITY=str(root / "capacity"),
                       PATH=f"{bin_dir}:{os.environ['PATH']}")
            result = subprocess.run(
                [str(AGENT_RUN), "test-agent", "cargo", "check"],
                env=env, capture_output=True, text=True, timeout=60,
            )
            self.assertEqual(result.returncode, 0, result.stderr[-500:])
            self.assertIn("started: check", result.stdout)
            self.assertEqual(calls.read_text().count("x"), 1)


if __name__ == "__main__":
    unittest.main()
