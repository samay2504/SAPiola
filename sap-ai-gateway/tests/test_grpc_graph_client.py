import pytest
from unittest.mock import AsyncMock, patch
import grpc

from sapiola_ai.graph_client_grpc import GrpcGraphClient
from sapiola.v1 import graph_pb2

@pytest.mark.asyncio
async def test_grpc_graph_client_success():
    client = GrpcGraphClient()
    
    mock_stub = AsyncMock()
    # Create a mock response
    mock_response = graph_pb2.QueryCandidatesResponse()
    cand = mock_response.candidates.add()
    cand.node_id = 42
    cand.label = "Material"
    cand.sap_key = "MAT-1"
    cand.pointer_path.extend(["Company", "BU", "Plant", "Material"])
    
    mock_stub.QueryCandidates.return_value = mock_response
    client._stub = mock_stub
    
    candidates = await client.search("inventory", "query")
    
    assert len(candidates) == 1
    assert candidates[0].node_id == 42
    assert candidates[0].sap_key == "MAT-1"
    assert candidates[0].domain == "inventory"
    assert candidates[0].pointer_path == ("Company", "BU", "Plant", "Material")

@pytest.mark.asyncio
async def test_grpc_graph_client_transient_error_retries():
    client = GrpcGraphClient()
    
    mock_stub = AsyncMock()
    
    class FakeRpcError(grpc.aio.AioRpcError):
        def __init__(self, code):
            self._code = code
        def code(self):
            return self._code
        def details(self):
            return "Fake error"

    # Fail 3 times, then succeed
    mock_response = graph_pb2.QueryCandidatesResponse()
    mock_stub.QueryCandidates.side_effect = [
        FakeRpcError(grpc.StatusCode.UNAVAILABLE),
        FakeRpcError(grpc.StatusCode.DEADLINE_EXCEEDED),
        FakeRpcError(grpc.StatusCode.UNAVAILABLE),
        mock_response
    ]
    client._stub = mock_stub
    
    # Should succeed after retries
    candidates = await client.search("inventory", "query")
    assert len(candidates) == 0
    assert mock_stub.QueryCandidates.call_count == 4
