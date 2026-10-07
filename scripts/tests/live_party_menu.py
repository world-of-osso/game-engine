"""Real-process regression for party context menus.

Run against the private live.gd capture harness: <script> <evidence-dir>
<label> <frame-name> <expected-member> <expected-entry>. Commands are mouse
input, not injected account state. Retains a screenshot and state on failure.
"""
import argparse
import json
from pathlib import Path
import time


def read_json(path):
    for _ in range(100):
        try:
            return json.loads(path.read_text())
        except json.JSONDecodeError:
            time.sleep(0.05)
    raise RuntimeError(f"Invalid JSON: {path}")


def send_command(directory, label, action, **fields):
    command_path = directory / f"{label}-command.json"
    result_path = directory / f"{label}-result.json"
    seq = max(read_json(command_path)["seq"], read_json(result_path)["seq"]) + 1
    temporary = command_path.with_suffix(".tmp")
    temporary.write_text(json.dumps(dict(seq=seq, action=action, **fields)))
    temporary.replace(command_path)
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        result = read_json(result_path)
        if result["seq"] == seq:
            if result["error"]:
                raise RuntimeError(result["error"])
            return
        time.sleep(0.1)
    raise TimeoutError(f"Live harness did not finish {action}")


def check_menu(directory, label, frame, member, entry):
    send_command(directory, label, "click", name=frame, button=2)
    stem = f"{label}-{frame}-menu-regression"
    send_command(directory, label, "snapshot", stem=stem)
    state = read_json(directory / f"{stem}.json")
    controls = state["controls"]
    titles = [control.get("text") for control in controls
              if control["name"] == "UnitFrameContextMenuTitle"]
    assert member in titles, f"Expected menu for {member}; observed titles: {titles}"
    assert any(control["name"] == entry for control in controls), entry
    print(f"PASS {frame}: {member}, {entry}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("label")
    parser.add_argument("frame")
    parser.add_argument("member")
    parser.add_argument("entry")
    arguments = parser.parse_args()
    check_menu(arguments.directory, arguments.label, arguments.frame,
               arguments.member, arguments.entry)
