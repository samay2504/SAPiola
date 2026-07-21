import sys
import pathlib
import grpc
import structlog

# Add the gen directory to sys.path so the generated stubs can import each other
gen_path = pathlib.Path(__file__).parent / "gen"
if str(gen_path) not in sys.path:
    sys.path.insert(0, str(gen_path))

from sapiola.v1 import graph_pb2
from sapiola.v1 import graph_pb2_grpc

from sapiola_ai.orchestrator import GraphCandidate
from sapiola_ai.resiliency import resilient_call

log = structlog.get_logger()

class GrpcGraphClient:
    def __init__(self, target: str = "localhost:50051"):
        self.target = target
        # Connect to the grpc server
        self._channel = grpc.aio.insecure_channel(self.target)
        self._stub = graph_pb2_grpc.GraphServiceStub(self._channel)

        from grpc_health.v1 import health_pb2
        from grpc_health.v1 import health_pb2_grpc
        self._health_stub = health_pb2_grpc.HealthStub(self._channel)

    async def check_health(self):
        from grpc_health.v1 import health_pb2
        try:
            req = health_pb2.HealthCheckRequest(service="sapiola.v1.GraphService")
            resp = await self._health_stub.Check(req, timeout=5.0)
            if resp.status != health_pb2.HealthCheckResponse.SERVING:
                raise ConnectionError(f"Graph server health check failed: {resp.status}")
        except grpc.aio.AioRpcError as e:
            if e.code() == grpc.StatusCode.UNIMPLEMENTED:
                raise ConnectionError(f"Wrong service or unreachable at configured target {self.target}: HealthCheck UNIMPLEMENTED")
            raise ConnectionError(f"Failed to connect to graph server at {self.target}: {e.code().name} {e.details()}")

    @resilient_call()
    async def search(self, domain: str, query: str) -> list[GraphCandidate]:
        request = graph_pb2.QueryCandidatesRequest(domain=domain, query=query)
        try:
            response = await self._stub.QueryCandidates(request)
        except grpc.aio.AioRpcError as e:
            # We want to catch specific grpc errors to retry them.
            # grpc.StatusCode.UNAVAILABLE, DEADLINE_EXCEEDED are transient.
            if e.code() in (grpc.StatusCode.UNAVAILABLE, grpc.StatusCode.DEADLINE_EXCEEDED):
                log.warning("grpc_transient_error", target=self.target, code=e.code().name, details=e.details())
                raise ConnectionError(f"gRPC connection error: {e.code().name}") from e
            raise  # Not a transient error, propagate immediately (e.g. UNIMPLEMENTED, PERMISSION_DENIED)
            
        candidates = []
        for c in response.candidates:
            candidates.append(
                GraphCandidate(
                    node_id=c.node_id,
                    label=c.label,
                    domain=domain,
                    sap_key=c.sap_key,
                    pointer_path=tuple(c.pointer_path)
                )
            )
        return candidates

    async def close(self):
        await self._channel.close()
