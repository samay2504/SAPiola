#!/usr/bin/env python3
import requests

key = "freellmapi-da07d76e216773d4faf337ea7383258af5530f20706680da"

candidate_bases = [
    "http://localhost:3001/v1",
    "https://api.freellm.ai/v1",
    "https://freellmapi.com/v1",
    "https://api.freellmapi.com/v1",
    "https://generativelanguage.googleapis.com/v1beta/openai",
]

for base in candidate_bases:
    url = f"{base}/chat/completions"
    headers = {"Authorization": f"Bearer {key}", "Content-Type": "application/json"}
    payload = {
        "model": "gemini-2.5-flash",
        "messages": [{"role": "user", "content": "Hello"}]
    }
    print(f"\nTesting POST to: {url} ...")
    try:
        r = requests.post(url, headers=headers, json=payload, timeout=5)
        print(f"  Status: {r.status_code}")
        print(f"  Response: {r.text[:200]}")
    except Exception as e:
        print(f"  Error: {e}")
