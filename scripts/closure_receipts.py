"""Receipt-selected auditor model IO, separate from raw producer byte discovery.

Native references: assets/creature.rs authored_asset_resolver/cache_model_files/
cache_model_textures. Model children use the owning model's namespace; native
terrain WMO/ADT readers still use flat files and remain unverified here.
"""

import hashlib
from pathlib import Path

from asset_closure import RawByteGraph
from model_asset_publication import index_asset_receipts, read_indexes

MODEL_KINDS = {"m2", "skin", "skel", "anim", "bone", "blp"}
PRODUCTS = {"wow", "wow_classic_beta"}


def asset_path(product, build_key, fdid, kind):
    directory = "textures" if kind == "blp" else "models"
    return f"products/{product}/{build_key}/{directory}/{fdid}.{kind}"


def validate_receipt(row, alias=False):
    product, key, kind = row["product"], row["build_key"], row["kind"]
    hashes = [(key, 32), (row["sha256"], 64)]
    if not alias or row.get("content_key"):
        hashes.append((row["content_key"], 32))
    if product not in PRODUCTS or not row.get("build") or row["bytes"] <= 0:
        raise ValueError(f"Incomplete verified receipt: {row['path']}")
    if any(
        len(value) != size or any(c not in "0123456789abcdefABCDEF" for c in value)
        for value, size in hashes
    ):
        raise ValueError(f"Invalid verified receipt hashes: {row['path']}")
    expected = asset_path(product, key, row["fdid"], kind)
    prefix = Path("products") / product / key
    path = Path(row["path"])
    if path.is_absolute() or ".." in path.parts or not path.is_relative_to(prefix):
        raise ValueError(f"Receipt outside product/build namespace: {row['path']}")
    if not alias and row["path"] != expected:
        raise ValueError(f"Asset has wrong namespace; expected {expected}")


class ReceiptClosure(RawByteGraph):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.active_asset = None
        self.adding_product = None
        documents = read_indexes(self.data)
        self.receipts = {
            key: row for key, (_, row) in index_asset_receipts(documents).items()
        }
        self.scoped_receipts = {}
        for path, document in documents.items():
            self.input_file(path)
            for alias, rows in [
                (False, document["assets"]),
                (True, document.get("aliases", [])),
            ]:
                for row in rows:
                    validate_receipt(row, alias)
                    previous = self.scoped_receipts.get(row["path"])
                    if previous and any(
                        previous[field] != row[field]
                        for field in ["product", "fdid", "kind", "bytes", "sha256"]
                    ):
                        raise ValueError(f"Conflicting scoped receipt: {row['path']}")
                    self.scoped_receipts[row["path"]] = row

    def request(self, fdid, kind):
        parent = self.active_asset
        product = self.adding_product or (
            parent["requested_product"] if parent else self.product
        )
        if product not in PRODUCTS:
            raise ValueError(f"Unsupported requested product {product}")
        # WMO/ADT children have no model resolver: their textures stay flat.
        scoped = bool(
            kind in MODEL_KINDS
            and (not parent or parent["asset_build_key"] or kind == "m2")
        )
        receipt = self.receipts.get((product, int(fdid), kind)) if scoped else None
        build_key = (
            parent["asset_build_key"]
            if scoped and parent and parent["asset_build_key"]
            else (receipt["build_key"] if receipt else "")
        )
        path = asset_path(product, build_key, fdid, kind) if build_key else None
        return product, build_key, scoped, path

    def asset_key(self, fdid, kind):
        product, key, scoped, _ = self.request(fdid, kind)
        namespace = "model:" + key if scoped else "flat"
        return product, namespace, int(fdid), kind

    def wmo_context_key(self, fdid):
        return self.asset_key(fdid, "wmo")

    def add(self, fdid, kind, reason, parent=None, alias=None, *, product=None):
        if not fdid:
            return
        previous = self.adding_product
        self.adding_product = product
        try:
            requested, build_key, scoped, path = self.request(fdid, kind)
            key = self.asset_key(fdid, kind)
            super().add(fdid, kind, reason, parent, alias if not scoped else None)
        finally:
            self.adding_product = previous
        asset = self.assets[key]
        asset.update(
            requested_product=requested,
            asset_build_key=build_key,
            receipt_selected=scoped,
        )
        if scoped:
            asset["locations"] = {path} if path else set()
        source_edge = {
            "parent_product": self.active_asset["requested_product"]
            if self.active_asset
            else None,
            "parent_build_key": self.active_asset["asset_build_key"]
            if self.active_asset
            else None,
            "parent_fdid": parent,
            "reason": reason,
        }
        edges = asset.setdefault("source_edges", [])
        if source_edge not in edges:
            edges.append(source_edge)

    def inspect(self, asset):
        if not asset["receipt_selected"]:
            return super().inspect(asset)
        asset.update(
            source_product=None, actual_build=None, identity_status="unverified"
        )
        path = next(iter(asset["locations"]), None)
        receipt = self.scoped_receipts.get(path)
        if receipt is None:
            code = "missing_scoped_receipt" if path else "missing_asset_receipt"
            self.issue(
                code,
                f"{asset['requested_product']} {asset['fdid']}.{asset['type']} expected {path or 'matching product receipt'}",
                asset["fdid"],
            )
            asset.update(present=False, present_files=[], size=None, sha256=None)
            return None
        found = super().inspect(asset)
        if found is None:
            return None
        owner = self.receipts.get(
            (asset["requested_product"], asset["fdid"], asset["type"])
        )
        if owner is None or any(
            receipt[field] != owner[field] for field in ["bytes", "sha256"]
        ):
            self.issue(
                "alias_receipt_mismatch",
                f"{path}: alias has no matching same-product receipt owner",
                asset["fdid"],
            )
            return None
        with found.open("rb") as stream:
            md5 = hashlib.file_digest(stream, "md5").hexdigest()
        valid = (
            asset["size"] == receipt["bytes"]
            and asset["sha256"] == receipt["sha256"]
            and md5 == owner["content_key"]
        )
        if receipt.get("content_key"):
            valid = valid and md5 == receipt["content_key"]
        if not valid:
            self.issue(
                "receipt_payload_mismatch",
                f"{asset['requested_product']} {path}: size/SHA256/content-key mismatch",
                asset["fdid"],
            )
            return None
        asset.update(
            source_product=receipt["product"],
            actual_build=receipt["build"],
            identity_status="verified_receipt",
        )
        return found

    def expand(self, asset, data):
        previous = self.active_asset
        self.active_asset = asset
        try:
            super().expand(asset, data)
        finally:
            self.active_asset = previous

    def run(self):
        result = super().run()
        result["schema_version"] = 2
        result["resolver_tool_sha256"]["closure_receipts.py"] = hashlib.sha256(
            Path(__file__).read_bytes()
        ).hexdigest()
        result["summary"]["unverified_identity"] = sum(
            row["identity_status"] != "verified_receipt" for row in result["assets"]
        )
        result["io_policy"] = (
            "Receipt-selected model roots; model-owned companions/textures use the same namespace; native flat WMO/ADT inputs remain unverified. RawByteGraph is producer discovery, not audit."
        )
        return result
