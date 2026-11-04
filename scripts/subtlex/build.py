#!/usr/bin/env python3

import hashlib
import json
import sys
from pathlib import Path

# Pin the source after the first successful download.
SOURCE_SHA256 = "271c5a5fbf332f60762cfa34b11394427c220099d96c589751b6bc77e5b32c1a"


def main():
    if len(sys.argv) != 2:
        raise SystemExit("Usage: build.py <source.json>")

    source_path = Path(sys.argv[1])

    if not source_path.is_file():
        raise SystemExit(f"Source file not found: {source_path}")

    source_bytes = source_path.read_bytes()
    source_hash = hashlib.sha256(source_bytes).hexdigest()

    print(f"Source SHA-256: {source_hash}")

    if SOURCE_SHA256 is not None and source_hash != SOURCE_SHA256:
        raise SystemExit("Source file does not match the pinned version.")

    entries = json.loads(source_bytes.decode("utf-8"))

    if not isinstance(entries, list):
        raise SystemExit("Expected a JSON array of word entries.")

    project_root = Path(__file__).resolve().parents[2]

    output_path = (
        project_root
        / "data"
        / "line"
        / "subtlex-us-words.txt"
    )

    words = set()

    for entry in entries:
        if not isinstance(entry, dict):
            continue

        word = entry.get("word")

        if not isinstance(word, str) or not word:
            continue

        if not word[0].islower():
            continue

        if not word.isascii() or not word.isalpha():
            continue

        words.add(word.lower())

    sorted_words = sorted(words)

    output_path.parent.mkdir(parents=True, exist_ok=True)

    output_path.write_text(
        "\n".join(sorted_words) + "\n",
        encoding="utf-8"
    )

    print(f"Generated {len(sorted_words)} words.")
    print(f"Output: {output_path}")


if __name__ == "__main__":
    main()
