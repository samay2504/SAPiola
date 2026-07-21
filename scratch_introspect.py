import sys
import subprocess

try:
    from datasets import load_dataset
except ImportError:
    subprocess.check_call([sys.executable, "-m", "pip", "install", "datasets", "pyarrow", "pandas"])
    from datasets import load_dataset

dataset_name = "sap-ai-research/SALT"
tables = ["salesdocuments", "salesdocument_items", "customers", "addresses"]

import os
token = os.environ.get("HF_TOKEN")

print(f"Introspecting {dataset_name}...\n")
for table in tables:
    try:
        ds = load_dataset(dataset_name, table, split="train", streaming=True, token=token)
        # Get just the features
        print(f"Table: {table}")
        for col, feature in ds.features.items():
            print(f"  - {col}: {feature}")
        print()
    except Exception as e:
        print(f"Error loading {table}: {e}")
