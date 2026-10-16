"""Strict adapter for the native follower_route query's CSV corner contract."""
import json
import subprocess
import uuid
from pathlib import Path

from bot_controller import require_point


class NavmeshRoute:
    """query_argv includes the caller's agent-run scope and owned query binary."""

    def __init__(self, query_argv, navmesh, output):
        self.query_argv = list(query_argv)
        self.navmesh = Path(navmesh)
        self.output = Path(output)

    def __call__(self, start, goal, label):
        require_point(start)
        require_point(goal)
        argv = [*self.query_argv, str(self.navmesh), *map(str, [*start, *goal])]
        result = subprocess.run(argv, capture_output=True, text=True, timeout=20)
        receipt = {"label": label, "argv": argv, "exit": result.returncode,
                   "stdout": result.stdout, "stderr": result.stderr}
        (self.output / f"route-{uuid.uuid4().hex}.json").write_text(json.dumps(receipt, indent=2) + "\n")
        if result.returncode:
            raise RuntimeError(f"Native sourced route failed: {result.stderr or result.stdout}")
        try:
            corners = [[float(value) for value in row.split(',')]
                       for row in result.stdout.strip().splitlines()]
            if not corners:
                raise ValueError("no corners")
            for point in corners:
                require_point(point)
        except ValueError as error:
            raise ValueError(f"Invalid native route output: {error}") from error
        return corners
