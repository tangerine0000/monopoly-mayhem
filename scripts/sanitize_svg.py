#!/usr/bin/env python3
"""
SVG asset sanitizer for Cờ Tỷ Phú Sài Gòn.
Strips Inkscape / Sodipodi namespaces and metadata, cleans empty nodes,
and formats optimized SVG assets.
"""

import os
import sys
import xml.etree.ElementTree as ET

NAMESPACES_TO_REMOVE = [
    "{http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd}",
    "{http://www.inkscape.org/namespaces/inkscape}",
]

def sanitize_element(elem):
    # Remove unwanted attributes with inkscape/sodipodi namespaces
    keys_to_remove = []
    for key in elem.attrib:
        for ns in NAMESPACES_TO_REMOVE:
            if key.startswith(ns):
                keys_to_remove.append(key)
                break
    for key in keys_to_remove:
        del elem.attrib[key]

    # Recursively clean children
    children_to_remove = []
    for child in elem:
        tag = child.tag
        if any(tag.startswith(ns) for ns in NAMESPACES_TO_REMOVE) or tag.endswith("metadata"):
            children_to_remove.append(child)
        else:
            sanitize_element(child)

    for child in children_to_remove:
        elem.remove(child)

def sanitize_file(file_path):
    try:
        tree = ET.parse(file_path)
        root = tree.getroot()
        sanitize_element(root)
        tree.write(file_path, encoding="utf-8", xml_declaration=True)
        print(f"Sanitized: {file_path}")
    except Exception as e:
        print(f"Error sanitizing {file_path}: {e}", file=sys.stderr)

def main():
    target_dir = os.path.join(os.path.dirname(__file__), "..", "crates", "monopoly-client", "assets")
    if not os.path.exists(target_dir):
        print(f"Assets directory not found at {target_dir}, nothing to sanitize.")
        return 0

    for root, _, files in os.walk(target_dir):
        for f in files:
            if f.lower().endswith(".svg"):
                sanitize_file(os.path.join(root, f))
    return 0

if __name__ == "__main__":
    sys.exit(main())
