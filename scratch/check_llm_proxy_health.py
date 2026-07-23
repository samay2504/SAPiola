#!/usr/bin/env python3
"""
Step 3: Pre-Flight LLM Proxy Connectivity Health Check
Verifies model completion against Unified LLM Proxy (:31415) independently of RAG logic.
"""

import os
import sys
import asyncio
import pathlib

gateway_path = pathlib.Path(__file__).parent.parent / "sap-ai-gateway"
sys.path.insert(0, str(gateway_path))

from sapiola_ai.llm_binding import LiteLlmClient

async def main():
    api_key = os.environ.get("SAPIOLA_LLM_API_KEY", "freellmapi-da07d76e216773d4faf337ea7383258af5530f20706680da")
    api_base = os.environ.get("SAPIOLA_LLM_API_BASE", "http://127.0.0.1:31415/v1")
    model = os.environ.get("SAPIOLA_LLM_MODEL", "openai/gemini-2.5-flash")

    print(f"[Step 3] Checking LLM Proxy Health at {api_base} (Model: {model})...")
    client = LiteLlmClient(model=model, api_key=api_key, api_base=api_base)
    
    prompt = "Hello! Please respond with 'OK' if you are online and healthy."
    response = await client.complete(prompt)
    
    print(f"  -> Proxy Response: {response.strip()}")
    assert len(response.strip()) > 0, "Empty response from LLM proxy"
    print("\n[SUCCESS] Step 3 LLM Proxy Health Check PASSED 100%!")

if __name__ == "__main__":
    asyncio.run(main())
