import pytest
from fastapi.testclient import TestClient
from sapiola_ai.api import app, AnswerRequest
import sapiola_ai.api as api_module
from sapiola_ai.orchestrator import RagResult
from unittest.mock import AsyncMock

client = TestClient(app)

@pytest.fixture
def mock_orchestrator():
    mock = AsyncMock()
    api_module.orchestrator = mock
    yield mock
    api_module.orchestrator = None

def test_answer_endpoint_success(mock_orchestrator):
    mock_orchestrator.answer.return_value = RagResult(domain="sales", prompt="q", answer="SAP Hana answer", degraded=False)
    
    response = client.post("/answer", json={
        "query": "What is the vendor name?",
        "principal": "tenant123"
    })
    
    assert response.status_code == 200
    assert response.json() == {"answer": "SAP Hana answer", "degraded": False}
    mock_orchestrator.answer.assert_called_once()
    call_args = mock_orchestrator.answer.call_args
    assert call_args.kwargs["query"] == "What is the vendor name?"
    assert call_args.kwargs["principal"].name == "tenant123"
    assert call_args.kwargs["enable_embeddings"] is False

def test_answer_endpoint_degraded(mock_orchestrator):
    mock_orchestrator.answer.return_value = RagResult(domain="sales", prompt="q", answer="I couldn't contact the LLM. Relevant data is: ...", degraded=True)
    
    response = client.post("/answer", json={
        "query": "What is the vendor name?",
        "principal": "tenant123"
    })
    
    # Degraded mode should STILL return a 200 OK because the system successfully fell back
    assert response.status_code == 200
    assert response.json()["degraded"] is True
    assert "I couldn't contact the LLM" in response.json()["answer"]

def test_answer_endpoint_rbac_denial(mock_orchestrator):
    mock_orchestrator.answer.side_effect = PermissionError("RBAC Denied: Principal tenant123 does not have access")
    
    response = client.post("/answer", json={
        "query": "What is the vendor name?",
        "principal": "tenant123"
    })
    
    # An RBAC denial should return 403
    assert response.status_code == 403
    assert "RBAC Denied" in response.json()["detail"]
