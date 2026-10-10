import importlib.util
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[1]


def load_depot_build():
    spec = importlib.util.spec_from_file_location("depot_build", SCRIPTS / "depot-build.py")
    module = importlib.util.module_from_spec(spec)
    import sys

    sys.path.insert(0, str(SCRIPTS))
    spec.loader.exec_module(module)
    return module


class NativeSourceRootTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        base = Path(self.temporary.name)
        self.home = base / "home"
        self.home.mkdir()
        self.root = base / "slots" / "game-engine-x"
        self.root.mkdir(parents=True)
        self.override = base / "other" / "asset-resolver-branch"
        self.override.mkdir(parents=True)
        self.depot = load_depot_build()

    def tearDown(self):
        self.temporary.cleanup()

    def source_root(self, environment):
        clean = {k: v for k, v in os.environ.items() if not k.startswith("DEPOT_SIBLING_")}
        with mock.patch.dict(os.environ, clean | environment, clear=True):
            with mock.patch.object(Path, "home", return_value=self.home):
                return self.depot.native_source_root(self.root)

    def test_without_overrides_cargo_reads_the_checkout_itself(self):
        self.assertEqual(self.source_root({}), self.root)

    def test_override_places_the_checkout_beside_the_overridden_sibling(self):
        source = self.source_root({"DEPOT_SIBLING_ASSET_RESOLVER": str(self.override)})
        self.assertEqual(source.resolve(), self.root.resolve())
        self.assertEqual((source.parent / "asset-resolver").resolve(), self.override.resolve())
        self.assertEqual(
            (source.parent / "shared-protocol").resolve(),
            (self.root.parent / "shared-protocol").resolve(),
        )

    def test_override_change_relinks_the_sibling(self):
        self.source_root({"DEPOT_SIBLING_ASSET_RESOLVER": str(self.override)})
        second = self.override.parent / "asset-resolver-other"
        second.mkdir()
        source = self.source_root({"DEPOT_SIBLING_ASSET_RESOLVER": str(second)})
        self.assertEqual((source.parent / "asset-resolver").resolve(), second.resolve())


if __name__ == "__main__":
    unittest.main()
