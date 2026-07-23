#!/usr/bin/env python3
"""
Live RAG Test with Custom Unified LLM API Proxy on SALT Data

Exercises LiteLlmClient against the local Unified API proxy (http://localhost:3001/v1)
using API Key: freellmapi-52f2a828232d1a24a854f5cc66f76527d9a8e13bbc83605d
to query SALT sales document graph context.
"""

import sys
import pathlib
import asyncio
import os
from dotenv import load_dotenv

# Add sap-ai-gateway to sys.path
gateway_path = pathlib.Path(__file__).parent.parent / "sap-ai-gateway"
sys.path.insert(0, str(gateway_path))

# Load .env from sap-ai-gateway/.env
env_file = gateway_path / ".env"
load_dotenv(env_file, override=True)

from sapiola_ai.llm_binding import LiteLlmClient

async def main():
    print("=== Testing SAPiola RAG Generation with Unified LLM API Proxy ===")
    
    # Priority: SAPIOLA_LLM_API_KEY / SAPIOLA_LLM_API_BASE or User's Unified Proxy Defaults
    api_key = os.environ.get("SAPIOLA_LLM_API_KEY")
    if not api_key or api_key.startswith("AIzaSy"):
        api_key = "freellmapi-52f2a828232d1a24a854f5cc66f76527d9a8e13bbc83605d"

    api_base = os.environ.get("SAPIOLA_LLM_API_BASE")
    if not api_base:
        api_base = "http://localhost:3001/v1"

    raw_model = os.environ.get("SAPIOLA_LLM_MODEL", "openai/gemini-2.5-flash")
    if not raw_model.startswith("openai/"):
        model = f"openai/{raw_model.split('/')[-1]}"
    else:
        model = raw_model

    # Set OpenAI environment variables for litellm compatibility
    os.environ["OPENAI_API_KEY"] = api_key
    os.environ["OPENAI_API_BASE"] = api_base

    print(f"Target Model: {model}")
    print(f"API Base URL: {api_base}")
    print(f"API Key: {api_key[:12]}...{api_key[-6:]}")

    # Sample SALT graph context retrieved by sap-graph-layer
    salt_context = """
    [SAP RAG GRAPH CONTEXT]
    - Tenant: salt_production
    - Sales Document ID: 1000 (Type: OR - Standard Order)
    - Net Amount: 4,500.00 EUR
    - Created By: SYSTEM
    - Document Status: COMPLETED
    - Customer Name: Acme Industrial SAP GmbH (Customer ID: CUST-7740)
    - Billing Address: Friedrichstraße 100, 10117 Berlin, Germany
    - Line Items:
      * Item 10: Material MAT-8890 (Industrial Valves, Qty: 50 EA, Price: 2,500.00 EUR)
      * Item 20: Material MAT-9941 (Seal Rings, Qty: 200 EA, Price: 2,000.00 EUR)
    """

    user_query = "What is the net amount, customer name, and item breakdown for Sales Document 1000 in SALT?"

    prompt = f"""You are SAPiola AI Gateway. Answer the user question based ONLY on the provided SAP Graph Context below.

Context:
{salt_context}

User Question: {user_query}
"""

    print(f"\nSending RAG prompt via LiteLlmClient to {model} at {api_base}...\n")
    llm_client = LiteLlmClient(model=model, api_key=api_key, api_base=api_base)
    
    try:
        answer = await llm_client.complete(prompt)
        print("--- Live Unified API Proxy RAG Response ---")
        print(answer)
        print("\n[SUCCESS] Live Unified API Proxy RAG Test COMPLETED SUCCESSFULLY!")
    except Exception as e:
        print(f"\nFallback: Trying model 'openai/auto' due to error: {e}")
        fallback_client = LiteLlmClient(model="openai/auto", api_key=api_key, api_base=api_base)
        answer = await fallback_client.complete(prompt)
        print("--- Live Unified API Proxy (auto) Response ---")
        print(answer)
        print("\n[SUCCESS] Live Unified API Proxy RAG Test COMPLETED SUCCESSFULLY (via auto model)!")

if __name__ == "__main__":
    asyncio.run(main())
