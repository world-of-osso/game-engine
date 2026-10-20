"""The game-server checkout's db2_casc: the one WDC5 decoder for both repositories.

The checkout is the engine's sibling `game-server`, or DEPOT_SIBLING_GAME_SERVER.
"""

import sys
from pathlib import Path

from siblings import sibling_repo

SERVER_SCRIPTS = (
    sibling_repo(Path(__file__).resolve().parents[1], "game-server") / "scripts"
).resolve()
if not (SERVER_SCRIPTS / "db2_casc.py").is_file():
    raise ImportError(
        f"{SERVER_SCRIPTS / 'db2_casc.py'} is missing; set DEPOT_SIBLING_GAME_SERVER "
        "to the game-server checkout"
    )
sys.path.append(str(SERVER_SCRIPTS))
import db2_casc  # noqa: E402

if Path(db2_casc.__file__).resolve() != SERVER_SCRIPTS / "db2_casc.py":
    raise ImportError(
        f"db2_casc already loaded from {db2_casc.__file__}, not {SERVER_SCRIPTS}"
    )
