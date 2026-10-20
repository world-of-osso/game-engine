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

    def test_server_override_keeps_required_catalogs_with_checkout_data(self):
        content = self.root / "content"
        data = self.root / "data"
        content.mkdir()
        data.mkdir()
        for name in ("trading-post-monthly-catalog.json", "quest-reward-bonus.json"):
            (content / name).write_text('{"source": "tracked"}')
            (data / name).symlink_to(content / name)
        with mock.patch.dict(os.environ, {"DEPOT_SIBLING_SHARED_PROTOCOL": str(self.override)}):
            with mock.patch.object(Path, "home", return_value=self.home):
                source = self.depot.native_source_root(self.root, "game-server", ("shared-protocol",))
        for name in ("trading-post-monthly-catalog.json", "quest-reward-bonus.json"):
            self.assertEqual((source / "data" / name).read_bytes(), (content / name).read_bytes())
        self.assertEqual((source / "data").resolve(), data.resolve())

    def test_override_change_moves_the_source_root(self):
        # Cargo keeps a path dependency's build while its package path is unchanged and
        # its files are no newer than the last build, so retargeting one link would
        # reuse artifacts compiled from the previous sibling.
        first = self.source_root({"DEPOT_SIBLING_ASSET_RESOLVER": str(self.override)})
        second_override = self.override.parent / "asset-resolver-other"
        second_override.mkdir()
        second = self.source_root({"DEPOT_SIBLING_ASSET_RESOLVER": str(second_override)})
        self.assertNotEqual(second, first)
        self.assertEqual((second.parent / "asset-resolver").resolve(), second_override.resolve())
        self.assertEqual((first.parent / "asset-resolver").resolve(), self.override.resolve())

    def test_same_override_keeps_the_source_root(self):
        first = self.source_root({"DEPOT_SIBLING_ASSET_RESOLVER": str(self.override)})
        second = self.source_root({"DEPOT_SIBLING_ASSET_RESOLVER": str(self.override)})
        self.assertEqual(second, first)


if __name__ == "__main__":
    unittest.main()
