#!/usr/bin/env python3
"""
Live RAG Test with Gemini LLM API on SALT Data

Exercises LiteLlmClient against Gemini API using SAPIOLA_LLM_API_KEY
from sap-ai-gateway/.env to query SALT sales document graph context.
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
load_dotenv(env_file)

from sapiola_ai.llm_binding import LiteLlmClient

async def main():
    print("=== Testing SAPiola RAG Generation with Live Gemini API ===")
    
    model = os.environ.get("SAPIOLA_LLM_MODEL", "gemini/gemini-2.5-flash")
    api_key = os.environ.get("SAPIOLA_LLM_API_KEY")

    print(f"Target Model: {model}")
    print(f"API Key Status: {'FOUND' if api_key else 'NOT FOUND'}")

    if not api_key:
        print("ERROR: SAPIOLA_LLM_API_KEY is missing from sap-ai-gateway/.env")
        return

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

    print(f"\nSending RAG prompt to Gemini LLM ({model})...\n")
    llm_client = LiteLlmClient(model=model, api_key=api_key)
    
    try:
        answer = await llm_client.complete(prompt)
        print("--- Live Gemini RAG Response ---")
        print(answer)
        print("\n[SUCCESS] Live Gemini API RAG Test COMPLETED SUCCESSFULLY!")
    except Exception as e:
        print(f"\nFallback: Trying gemini/gemini-1.5-flash due to error: {e}")
        fallback_client = LiteLlmClient(model="gemini/gemini-1.5-flash", api_key=api_key)
        answer = await fallback_client.complete(prompt)
        print("--- Live Gemini 1.5 Response ---")
        print(answer)
        print("\n[SUCCESS] Live Gemini API RAG Test COMPLETED SUCCESSFULLY (via fallback model)!")

if __name__ == "__main__":
    asyncio.run(main())
