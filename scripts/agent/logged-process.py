#!/usr/bin/env python3
"""Exec an owned agent process with live file logs, never undrained spawn pipes.

Usage: agent-run NAME python3 logged-process.py LOG PROGRAM [ARGS...]
"""
import os
import sys


def main():
    if len(sys.argv) < 3:
        raise SystemExit("Usage: logged-process.py LOG PROGRAM [ARGS...]")
    descriptor = os.open(sys.argv[1], os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    os.dup2(descriptor, 1)
    os.dup2(descriptor, 2)
    os.close(descriptor)
    os.execvp(sys.argv[2], sys.argv[2:])


if __name__ == "__main__":
    main()
