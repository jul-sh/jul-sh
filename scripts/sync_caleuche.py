#!/usr/bin/env python3
"""Refresh Caleuche from the exact Kerberos fonts pinned by this site.

Run with Python 3.10+ and fontTools[woff]. The sibling iosevka-charon checkout
supplies the existing Caleuche statics and the shared height generator.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import tempfile
from datetime import date
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont


ROOT = Path(__file__).resolve().parents[1]


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def pinned_fonts() -> tuple[dict, dict[str, Path]]:
    manifest = json.loads((ROOT / "static/Kerberos-source.json").read_text())
    fonts = {}
    for source in manifest["sources"]:
        match source["source"]:
            case "bundled":
                path = (ROOT / source["path"]).resolve()
                if not path.is_relative_to(ROOT):
                    raise ValueError("Bundled font must be inside this repository")
            case _:
                raise ValueError("Bundle the pinned Kerberos fonts before refreshing Caleuche")
        if path.stat().st_size != source["bytes"] or digest(path) != source["sha256"]:
            raise ValueError(f"Pinned font integrity check failed: {path}")
        fonts[source["style"]] = path
    if set(fonts) != {"roman", "italic"}:
        raise ValueError("Expected one Roman and one Italic Kerberos source")
    return manifest, fonts


def refresh(caleuche: Path) -> None:
    manifest, fonts = pinned_fonts()
    exporter = caleuche / "scripts/sync_caleuche_kerberos.py"
    generator = caleuche / "scripts/caleuche_variable.py"
    metrics = ROOT / "target/caleuche/metrics.json"
    variable_dir = ROOT / "target/caleuche/variable"
    subprocess.run([
        sys.executable, str(exporter), "--roman", str(fonts["roman"]),
        "--italic", str(fonts["italic"]), "--output", str(metrics),
    ], cwd=caleuche, check=True)
    styles = ("Regular", "Medium", "Bold")
    source_dir = caleuche / "fonts/iosevkacaleuche"
    subprocess.run([
        sys.executable, str(generator),
        *(str(source_dir / f"IosevkaCaleuche-{style}.ttf") for style in styles),
        "--metrics", str(metrics), "--out", str(variable_dir),
    ], cwd=caleuche, check=True)
    publish(caleuche, manifest, metrics, variable_dir)


def publish(caleuche: Path, manifest: dict, metrics: Path, variable_dir: Path) -> None:
    """Publish already generated fonts after checking their height targets."""
    generator = caleuche / "scripts/caleuche_variable.py"
    styles = ("Regular", "Medium", "Bold")

    metadata_path = ROOT / "static/IosevkaCaleuche-source.json"
    metadata = json.loads(metadata_path.read_text())
    snapshot = json.loads(metrics.read_text())
    for entry in manifest["sources"]:
        measured = snapshot["font_sources"][entry["style"]]["sha256"]
        if measured not in {entry["sha256"], entry.get("source_font_sha256")}:
            raise ValueError("Caleuche was measured against a different Kerberos font")
    entries = {entry["file"]: entry for entry in metadata["fonts"]}
    with tempfile.TemporaryDirectory(prefix="caleuche-sync-") as temporary:
        staging = Path(temporary)
        outputs = []
        for style in styles:
            name = f"IosevkaCaleuche-{style}[YTLC,opsz].woff2"
            current = TTFont(ROOT / "static" / name)
            coverage = set(current.getBestCmap())
            current.close()
            source = variable_dir / name.replace(".woff2", ".ttf")
            verify_profile(source, snapshot["profiles"][style], snapshot["axes"]["YTLC"])
            options = subset.Options()
            options.flavor = "woff2"
            options.layout_features = ["*"]
            options.name_IDs = ["*"]
            options.name_legacy = True
            options.name_languages = ["*"]
            options.notdef_outline = True
            options.recommended_glyphs = True
            font = subset.load_font(source, options)
            cutter = subset.Subsetter(options)
            cutter.populate(unicodes=coverage)
            cutter.subset(font)
            output = staging / name
            subset.save_font(font, output, options)
            result = TTFont(output)
            if set(result.getBestCmap()) != coverage:
                raise ValueError(f"Character coverage changed: {name}")
            entries[name].pop("source_ttf_path", None)
            entries[name].update({
                "sha256": digest(output), "bytes": output.stat().st_size,
                "font_version": result["name"].getDebugName(5),
                "glyph_count": len(result.getGlyphOrder()),
                "unicode_codepoint_count": len(coverage),
                "source_ttf_file": source.name,
                "source_ttf_sha256": digest(source),
                "axes": [{"tag": axis.axisTag, "minimum": axis.minValue,
                          "default": axis.defaultValue, "maximum": axis.maxValue}
                         for axis in result["fvar"].axes],
                "notes": "Lowercase heights follow the measured Kerberos optical-size and YTLC profile; original drawing and advances are preserved.",
            })
            result.close()
            outputs.append((output, ROOT / "static" / name))
        metadata.update({
            "checked_on": date.today().isoformat(),
            "paired_kerberos_version": manifest["release"],
            "paired_kerberos_sources": {entry["style"]: entry["sha256"]
                                        for entry in manifest["sources"]},
            "kerberos_metrics_sha256": digest(metrics),
            "kerberos_font_sources": snapshot["font_sources"],
            "generator": {"path": "scripts/caleuche_variable.py", "sha256": digest(generator)},
            "derivation": "Existing Caleuche static drawings with lowercase heights derived automatically from the site's pinned Kerberos fonts. Webfonts retain the previous exact Unicode coverage and OpenType features, with matching opsz and YTLC axes.",
        })
        # All outputs are checked before replacing the site's pinned assets.
        for source, destination in outputs:
            destination.write_bytes(source.read_bytes())
        metadata_path.write_text(json.dumps(metadata, indent=2) + "\n")
    print("Updated Caleuche to match the site's pinned Kerberos fonts.")


def verify_profile(path: Path, profile: dict, heights: list[float]) -> None:
    from fontTools.pens.boundsPen import BoundsPen

    with TTFont(path) as font:
        name = font.getBestCmap()[ord("x")]
        for optical, targets in profile["targets"].items():
            for height, target in zip(sorted(set(heights)), targets, strict=True):
                glyphs = font.getGlyphSet(location={"opsz": float(optical), "YTLC": height})
                pen = BoundsPen(glyphs)
                glyphs[name].draw(pen)
                if pen.bounds is None or abs(pen.bounds[3] - target) > 1:
                    raise ValueError(f"Stale Caleuche height profile: {path.name}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--caleuche-dir", type=Path, default=ROOT.parent / "iosevka-charon")
    args = parser.parse_args()
    refresh(args.caleuche_dir.resolve())


if __name__ == "__main__":
    main()
