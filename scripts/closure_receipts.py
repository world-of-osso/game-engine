"""Receipt-selected auditor model IO, separate from raw producer byte discovery.

Native references: assets/creature.rs authored_asset_resolver/cache_model_files/
cache_model_textures. Model children use the owning model's namespace; native
terrain WMO/ADT readers still use flat files with separate provenance checks.
"""

from collections import Counter
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
    if (
        product not in PRODUCTS
        or (not alias and not row.get("build"))
        or row["bytes"] <= 0
    ):
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


def receipt_rows(document):
    yield from ((row, False) for row in document["assets"])
    yield from ((row, True) for row in document.get("aliases", []))


class ReceiptClosure(RawByteGraph):
    receipt_audit = True

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.active_asset = None
        self.adding_product = None
        self.adding_flat = False
        self.source_appearances = {}
        self.source_catalogs = {}
        self.db2_policy_exclusions = {}
        documents = read_indexes(self.data)
        self.receipts = {
            key: row for key, (_, row) in index_asset_receipts(documents).items()
        }
        self.scoped_receipts = {}
        for path, document in documents.items():
            self.input_file(path)
            for row, alias in receipt_rows(document):
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
            not self.adding_flat
            and kind in MODEL_KINDS
            and (not parent or parent["receipt_selected"] or kind == "m2")
        )
        receipt = self.receipts.get((product, int(fdid), kind)) if scoped else None
        build_key = (
            parent["asset_build_key"]
            if scoped and parent and parent["receipt_selected"]
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
            metadata_build=self.active_asset["metadata_build"]
            if self.active_asset
            else self.metadata_build,
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

    def add_flat(self, fdid, kind, reason, parent=None):
        previous = self.adding_flat
        self.adding_flat = True
        try:
            self.add(fdid, kind, reason, parent)
        finally:
            self.adding_flat = previous

    def verify_metadata(self, path, name):
        from closure_nonmodel_receipts import verify_table

        return verify_table(self, path, name)

    def add_model_texture(self, model_fdid, texture_fdid, reason):
        model_asset = self.assets.get(self.asset_key(model_fdid, "m2"))
        if model_asset is None:
            self.issue(
                "missing_product_source",
                f"texture {texture_fdid}: model {model_fdid} namespace unavailable",
                texture_fdid,
            )
            return
        previous = self.active_asset
        self.active_asset = model_asset
        try:
            self.add(texture_fdid, "blp", reason, model_fdid)
        finally:
            self.active_asset = previous

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
            actual_build=owner["build"],
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
        from closure_nonmodel_receipts import qualify_flat_assets

        result = super().run()
        qualify_flat_assets(self, result)
        result["inputs"] = sorted(self.inputs.values(), key=lambda row: row["path"])
        result["unresolved"] = self.unresolved_records()
        result["summary"].update(
            unresolved=len(result["unresolved"]),
            unresolved_by_code=dict(
                sorted(Counter(row["code"] for row in result["unresolved"]).items())
            ),
        )
        result["schema_version"] = 2
        result["db2_policy_exclusions"] = [
            self.db2_policy_exclusions[path]
            for path in sorted(self.db2_policy_exclusions)
        ]
        result["summary"]["db2_policy_excluded_records"] = sum(
            row["excluded_records"] for row in result["db2_policy_exclusions"]
        )
        for name in [
            "closure_receipts.py",
            "closure_root_products.py",
            "closure_nonmodel_receipts.py",
            "closure_db2_policy.py",
            "closure_player_roots.py",
        ]:
            result["resolver_tool_sha256"][name] = hashlib.sha256(
                Path(__file__).with_name(name).read_bytes()
            ).hexdigest()
        result["summary"]["unverified_identity"] = sum(
            row["identity_status"] != "verified_receipt" for row in result["assets"]
        )
        result["io_policy"] = (
            "Receipt-selected model roots and owning namespaces; actual flat texture/sound bytes require separate matching source receipts, never scoped substitution. DB2 exports require matching table receipts. RawByteGraph is producer discovery, not audit."
        )
        return result
