#!/usr/bin/env python3
"""Usage: link-worktree-data.py <canonical-repo> <worktree>. Link canonical data/ into a worktree without shadowing tracked files: recurse into dirs that exist in the worktree."""
import os, sys
def link(src, dst):
    for e in os.listdir(src):
        s, d = os.path.join(src, e), os.path.join(dst, e)
        if os.path.lexists(d):
            if os.path.isdir(d) and not os.path.islink(d) and os.path.isdir(s): link(s, d)
            continue
        # SQLite sidecars belong to whichever db file the worktree opens; never share them.
        if e.startswith("auth_token") or e.endswith(("-wal", "-shm", "-journal")): continue
        os.symlink(s, d)
canon, wt = sys.argv[1], sys.argv[2]
os.makedirs(os.path.join(wt, "data"), exist_ok=True)
link(os.path.join(canon, "data"), os.path.join(wt, "data"))
