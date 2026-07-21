from typing import List

_model = None

def get_embedder():
    global _model
    if _model is None:
        try:
            from sentence_transformers import SentenceTransformer
            # all-MiniLM-L6-v2 produces 384-dimensional embeddings
            _model = SentenceTransformer("all-MiniLM-L6-v2")
        except ImportError:
            raise RuntimeError("sentence-transformers is not installed.")
    return _model

def embed_text(text: str) -> List[float]:
    """Embeds the given text using the global SentenceTransformer model."""
    model = get_embedder()
    # SentenceTransformer outputs a numpy array. We need a list of floats for LanceDB/PyArrow.
    vector = model.encode(text)
    return vector.tolist()
