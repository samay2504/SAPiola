import lancedb
import pyarrow as pa
from typing import Any

from sapiola_ai.orchestrator import GraphCandidate, EmbeddingClient


class LanceDbEmbeddingClient:
    """Concrete EmbeddingClient backed by an embedded, in-process LanceDB table.
    No server, no Docker — this opens a local directory as its storage.
    """
    def __init__(self, db_path: str, embedding_fn: Any, table_name: str = "entity_embeddings"):
        self._db = lancedb.connect(db_path)
        self._table = self._db.open_table(table_name)
        self._embedding_fn = embedding_fn
        
    async def verify_dimensions(self) -> None:
        """Verify that the embedding model's dimensions match the LanceDB schema."""
        model_dim = await self._embedding_fn.get_dimensions()
        schema = self._table.schema
        vector_field = schema.field("vector")
        table_dim = vector_field.type.list_size
        
        if model_dim != table_dim:
            raise ValueError(
                f"Embedding dimension mismatch: Model produces {model_dim} dims, "
                f"but LanceDB table '{self._table.name}' expects {table_dim} dims."
            )

    async def recall(self, domain: str, query: str) -> list[GraphCandidate]:
        query_vec = await self._embedding_fn.embed(query)
        rows = (
            self._table.search(query_vec)
            .where(f"domain = '{domain}'")   # pre-filter by domain before RBAC even sees it
            .limit(20)
            .to_list()
        )
        
        return [
            GraphCandidate(
                node_id=r["node_id"],
                label=r["label"],
                domain=r["domain"],
                sap_key=r["sap_key"],
                pointer_path=tuple(r["pointer_path"])
            )
            for r in rows
        ]
