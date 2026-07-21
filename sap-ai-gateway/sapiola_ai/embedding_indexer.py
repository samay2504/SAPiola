import lancedb
import pyarrow as pa
from typing import Any

class EmbeddingIndexer:
    """Populates an embedded LanceDB table from the structural pointer index.
    Embeddings are a derived, rebuildable cache over the pointer index.
    """
    
    def __init__(self, db_path: str, embedding_fn: Any, table_name: str = "entity_embeddings"):
        self._db_path = db_path
        self._table_name = table_name
        self._embedding_fn = embedding_fn
        self._db = lancedb.connect(db_path)
        
    async def index(self, entities: list[dict]):
        """
        Populate the LanceDB table. 
        `entities` should be a list of dictionaries with keys:
        - node_id (int)
        - label (str)
        - domain (str)
        - sap_key (str)
        - pointer_path (list of str)
        - text (str)
        """
        model_dim = await self._embedding_fn.get_dimensions()
        
        schema = pa.schema([
            pa.field("node_id", pa.int64()),
            pa.field("label", pa.string()),
            pa.field("domain", pa.string()),
            pa.field("sap_key", pa.string()),
            pa.field("pointer_path", pa.list_(pa.string())),
            pa.field("text", pa.string()),
            pa.field("vector", pa.list_(pa.float32(), model_dim))
        ])
        
        if self._table_name in self._db.table_names():
            table = self._db.open_table(self._table_name)
            table_dim = table.schema.field("vector").type.list_size
            if model_dim != table_dim:
                raise ValueError(
                    f"Embedding dimension mismatch: Model produces {model_dim} dims, "
                    f"but LanceDB table '{self._table_name}' expects {table_dim} dims."
                )
        else:
            table = self._db.create_table(self._table_name, schema=schema)
            
        data = []
        for e in entities:
            data.append({
                "node_id": e["node_id"],
                "label": e["label"],
                "domain": e["domain"],
                "sap_key": e["sap_key"],
                "pointer_path": e["pointer_path"],
                "text": e["text"],
                "vector": await self._embedding_fn.embed(e["text"])
            })
            
        if data:
            table.add(data)
