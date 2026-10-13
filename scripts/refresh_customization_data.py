#!/usr/bin/env python3
"""Refresh customization as part of the single active-install DB2 export.

Use the same arguments as refresh_db2_install.py. Remote CSV services and
former-build backups are no longer an extraction path.
"""
from refresh_db2_install import main

if __name__ == '__main__':
    main()
