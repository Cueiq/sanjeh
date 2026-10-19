import csv
import json
import random
import argparse
from pathlib import Path

def generate_benchmark(tsv_file, output_jsonl, sample_size=500):
    items = []

    with open(tsv_file, mode='r', encoding='utf-8') as f:
        reader = csv.DictReader(f, delimiter='\t')

        for row in reader:
            accent = row.get('accents', '').lower()
            if 'us' in accent or 'united states' in accent:
                clean_accent = 'us'
            elif 'gb' in accent or 'united kingdom' in accent or 'england' in accent:
                clean_accent = 'gb'
            else:
                continue

            items.append({
                "path": row['path'],
                "text": row['sentence'],
                "accent": clean_accent
            })

    if len(items) > sample_size:
        items = random.sample(items, sample_size)

    split_index = int(len(items) * 0.8)

    benchmark_items = []
    for i, data in enumerate(items):
        split = "fit" if i < split_index else "test"

        item = {
            "id": f"cv-{i}",
            "group": "common_voice",
            "split": split,
            "input": {
                "text": data["text"],
                "duration_ms": 0,
                "accent": data["accent"],
                "voices": 1,
                "entries": [],
                "names": []
            },
            "labels": []
        }
        benchmark_items.append(item)


    output_path = Path(output_jsonl)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, 'w', encoding='utf-8') as out_f:
        for item in benchmark_items:
            out_f.write(json.dumps(item) + '\n')

    print(f"✅ Benchmark with {len(benchmark_items)} lines saved to {output_jsonl}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Prepare open-lines benchmark dataset")
    parser.add_argument("tsv_file", help="Path to the Common Voice TSV file")
    parser.add_argument("output", help="Path to output the items.jsonl file")

    args = parser.parse_args()
    generate_benchmark(args.tsv_file, args.output)