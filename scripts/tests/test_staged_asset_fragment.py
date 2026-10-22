"""Staged receipt fragments must not duplicate any canonical receipt owner."""

import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import model_asset_publication as publication


class StagedFragmentTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.data = Path(directory.name)
        self.cache = self.data / "cache"
        self.cache.mkdir()

    def receipt(self, fdid, scope="a", payload=b"SKIN authentic"):
        return {
            "product": "wow",
            "fdid": fdid,
            "kind": "skin",
            "build_key": scope * 32,
            "build": "12.1.5.70448",
            "path": f"products/wow/{scope * 32}/models/{fdid}.skin",
            "bytes": len(payload),
            "sha256": hashlib.sha256(payload).hexdigest(),
            "content_key": hashlib.md5(payload).hexdigest(),
        }

    def document(self, assets=(), aliases=()):
        return {
            "version": 1,
            "assets": list(assets),
            "aliases": list(aliases),
            "chains": [],
        }

    def write(self, name, document):
        path = self.cache / name
        path.write_text(json.dumps(document))
        return path

    def test_dedupe_reads_base_and_fragments_without_mutating_canonical(self):
        first, second = self.receipt(42), self.receipt(43)
        self.write("model-asset-index.json", self.document([first]))
        self.write("model-asset-index-other.json", self.document([second]))
        before = {p: p.read_bytes() for p in self.cache.iterdir()}
        incoming = self.document(
            [self.receipt(42, "b"), self.receipt(43, "b"), self.receipt(44, "b")]
        )
        prepared = publication.deduplicate_staged_fragment(self.data, incoming)
        self.assertEqual([row["fdid"] for row in prepared["assets"]], [44])
        self.assertEqual(len(incoming["assets"]), 3)
        self.assertEqual({p: p.read_bytes() for p in self.cache.iterdir()}, before)
        publication.validate_staged_fragment(
            self.data, json.loads(json.dumps(prepared))
        )

    def test_preflight_rejects_cross_fragment_duplicate_even_when_bytes_match(self):
        self.write("model-asset-index-old.json", self.document([self.receipt(42)]))
        with self.assertRaisesRegex(ValueError, "Duplicate.*receipt"):
            publication.validate_staged_fragment(
                self.data, self.document([self.receipt(42, "b")])
            )

    def test_payload_collision_is_not_silently_discarded(self):
        path = self.write("model-asset-index.json", self.document([self.receipt(42)]))
        original = path.read_bytes()
        with self.assertRaisesRegex(ValueError, "conflicting.*receipt"):
            publication.deduplicate_staged_fragment(
                self.data, self.document([self.receipt(42, "b", b"SKIN different")])
            )
        self.assertEqual(path.read_bytes(), original)

    def test_aliases_dedupe_by_product_and_owner_path_not_child_fdid(self):
        old = dict(self.receipt(42), path=f"products/wow/{'a' * 32}/models/4200.skin")
        repeated = dict(
            self.receipt(42, "b"), path=f"products/wow/{'b' * 32}/models/4200.skin"
        )
        distinct_owner = dict(
            repeated, path=f"products/wow/{'b' * 32}/models/4300.skin"
        )
        self.write("model-asset-index.json", self.document([self.receipt(42)], [old]))
        prepared = publication.deduplicate_staged_fragment(
            self.data,
            self.document([self.receipt(42, "b")], [repeated, distinct_owner]),
        )
        self.assertEqual(prepared["assets"], [])
        self.assertEqual(
            prepared["aliases"], [repeated, distinct_owner, self.receipt(42, "b")]
        )
        publication.validate_staged_fragment(self.data, prepared)

    def test_new_model_build_materializes_identical_owned_companion_path(self):
        payload = b"SKIN same authenticated companion"
        previous = self.receipt(42, "a", payload)
        requested = self.receipt(42, "b", payload)
        index = self.write("model-asset-index.json", self.document([previous]))
        original_index = index.read_bytes()
        old_path = self.data / previous["path"]
        old_path.parent.mkdir(parents=True)
        old_path.write_bytes(payload)

        prepared = publication.deduplicate_staged_fragment(
            self.data, self.document([requested])
        )
        publication.validate_staged_fragment(self.data, prepared)
        for row in prepared["assets"] + prepared["aliases"]:
            destination = self.data / row["path"]
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(payload)
        self.write("model-asset-index-new.json", prepared)

        self.assertTrue((self.data / requested["path"]).is_file())
        self.assertEqual((self.data / requested["path"]).read_bytes(), payload)
        self.assertEqual(old_path.read_bytes(), payload)
        self.assertEqual(index.read_bytes(), original_index)
        owners = publication.index_asset_receipts(publication.read_indexes(self.data))
        self.assertEqual(len(owners), 1)
        self.assertEqual(
            next(iter(owners.values()))[1]["build_key"], previous["build_key"]
        )

    def test_alias_colocation_keeps_distinct_builds_and_dedupes_same_path(self):
        previous = self.receipt(42, "a")
        earlier_copy = self.receipt(42, "b")
        requested = self.receipt(42, "c")
        self.write("model-asset-index.json", self.document([previous], [earlier_copy]))
        prepared = publication.deduplicate_staged_fragment(
            self.data, self.document([requested], [requested, requested])
        )
        self.assertEqual(prepared["assets"], [])
        self.assertEqual(prepared["aliases"], [requested])
        publication.validate_staged_fragment(self.data, prepared)

    def test_incoming_internal_duplicates_fail_preflight(self):
        with self.assertRaisesRegex(ValueError, "Duplicate.*receipt"):
            publication.validate_staged_fragment(
                self.data, self.document([self.receipt(42), self.receipt(42, "b")])
            )

    def test_repeated_legacy_alias_metadata_does_not_invalidate_unique_assets(self):
        alias = dict(self.receipt(42), path=f"products/wow/{'a' * 32}/models/4200.skin")
        del alias["content_key"]
        self.write("model-asset-index.json", self.document([self.receipt(42)], [alias]))
        self.write(
            "model-asset-index-other.json", self.document([self.receipt(43)], [alias])
        )
        prepared = publication.deduplicate_staged_fragment(
            self.data, self.document([self.receipt(44)])
        )
        self.assertEqual([row["fdid"] for row in prepared["assets"]], [44])
        publication.validate_staged_fragment(self.data, prepared)

    def test_existing_cross_fragment_duplicates_fail_before_staging(self):
        self.write("model-asset-index.json", self.document([self.receipt(42)]))
        self.write(
            "model-asset-index-broken.json", self.document([self.receipt(42, "b")])
        )
        with self.assertRaisesRegex(ValueError, "Duplicate.*receipt"):
            publication.deduplicate_staged_fragment(
                self.data, self.document([self.receipt(44)])
            )


if __name__ == "__main__":
    unittest.main()
