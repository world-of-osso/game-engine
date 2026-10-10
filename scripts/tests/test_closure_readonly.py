"""Closure's read transaction ends and its SQLite descriptor closes at the context boundary."""

from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
from closure_seeds import readonly


class ReadonlyTests(unittest.TestCase):
    def test_read_transaction_closes_on_exit(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "catalog.sqlite"
            db = sqlite3.connect(path)
            db.execute("CREATE TABLE rows (id)")
            db.execute("INSERT INTO rows VALUES (42)")
            db.commit()
            db.close()
            with readonly(path) as reader:
                self.assertEqual(
                    reader.execute("SELECT id FROM rows").fetchall()[0][0], 42
                )
            with self.assertRaises(sqlite3.ProgrammingError):
                reader.execute("SELECT id FROM rows")


if __name__ == "__main__":
    unittest.main()
