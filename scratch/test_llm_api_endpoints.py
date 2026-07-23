#!/usr/bin/env python3
import requests

key = "freellmapi-52f2a828232d1a24a854f5cc66f76527d9a8e13bbc83605d"

candidate_bases = [
    "http://localhost:3001/v1",
    "http://127.0.0.1:3001/v1",
    "https://api.freellm.ai/v1",
    "https://freellmapi.com/v1",
    "https://api.freellmapi.com/v1",
    "https://generativelanguage.googleapis.com/v1beta/openai",
]

for base in candidate_bases:
    url = f"{base}/models"
    headers = {"Authorization": f"Bearer {key}"}
    print(f"Testing base: {base} ...")
    try:
        r = requests.get(url, headers=headers, timeout=5)
        print(f"  -> HTTP {r.status_code}: {r.text[:150]}")
    except Exception as e:
        print(f"  -> Error: {e}")
