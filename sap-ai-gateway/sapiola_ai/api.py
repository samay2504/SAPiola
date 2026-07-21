from fastapi import FastAPI, HTTPException
from sapiola_ai.quickstart import Sapiola
from sapiola_ai.orchestrator import RagOrchestrator, Principal
from pydantic import BaseModel
from typing import Optional
import uvicorn

app = FastAPI(title="SAPiola AI Gateway")

class AnswerRequest(BaseModel):
    query: str
    principal: str
    enable_embeddings: Optional[bool] = None

# Global orchestrator instance initialized on startup
orchestrator: RagOrchestrator = None

@app.on_event("startup")
async def startup_event():
    global orchestrator
    # Note: connect() sets up LLM, Embeddings, and Graph clients automatically
    orchestrator = await Sapiola.connect()

@app.post("/answer")
async def answer_endpoint(request: AnswerRequest):
    global orchestrator
    if not orchestrator:
        raise HTTPException(status_code=503, detail="Orchestrator not ready")
        
    try:
        result = await orchestrator.answer(
            query=request.query, 
            principal=Principal(name=request.principal, domains=frozenset([request.principal])), 
            enable_embeddings=request.enable_embeddings or False
        )
        return {"answer": result.answer, "degraded": result.degraded}
    except PermissionError as e:
        # RBAC failures should return 403 explicitly
        raise HTTPException(status_code=403, detail=str(e))
    except Exception as e:
        # If the breaker trips and degrades the response, it's caught in the orchestrator and returned as degraded=True.
        # So we only catch actual uncontrolled crashes here.
        raise HTTPException(status_code=500, detail=str(e))

if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=8000)
