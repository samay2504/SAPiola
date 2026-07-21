import time
import random
from neo4j import GraphDatabase

NUM_VERTICES = 10_000
NUM_EDGES = 100_000

def bench_db(uri, db_name):
    print(f"--- Benchmarking {db_name} ---")
    driver = GraphDatabase.driver(uri, auth=None)
    
    with driver.session() as session:
        # Clear DB and create index
        session.run("MATCH (n) DETACH DELETE n")
        try:
            session.run("CREATE INDEX ON :Node(id)")
        except:
            pass # Index might already exist
            
        try:
            session.run("CREATE INDEX FOR (n:Node) ON (n.id)")
        except:
            pass
        
        print("Inserting vertices...")
        t0 = time.time()
        vertices = list(range(1, NUM_VERTICES + 1))
        for i in range(0, len(vertices), 1000):
            batch = vertices[i:i+1000]
            session.run("UNWIND $batch AS id CREATE (n:Node {id: id})", batch=batch)
        v_time = time.time() - t0
        print(f"  {NUM_VERTICES} vertices in {v_time:.2f}s ({NUM_VERTICES/v_time:.0f} ops/sec)")
        
        print("Inserting skewed edges...")
        t0 = time.time()
        # Skewed workload: 10% of nodes get 90% of edges
        edges = []
        for _ in range(NUM_EDGES):
            if random.random() < 0.9:
                src = random.randint(1, max(2, NUM_VERTICES // 10))
            else:
                src = random.randint(1, NUM_VERTICES)
            dst = random.randint(1, NUM_VERTICES)
            edges.append((src, dst))
            
        t1 = time.time()
        
        # Batch insert for edges
        batch_size = 1000
        for i in range(0, len(edges), batch_size):
            batch = edges[i:i+batch_size]
            query = """
            UNWIND $batch AS pair
            MATCH (s:Node {id: pair[0]}), (t:Node {id: pair[1]})
            CREATE (s)-[:EDGE]->(t)
            """
            session.run(query, batch=batch)
            
        e_time = time.time() - t1
        print(f"  {NUM_EDGES} edges in {e_time:.2f}s ({NUM_EDGES/e_time:.0f} ops/sec)")
        
        print("Reads...")
        t0 = time.time()
        for _ in range(100):
            node_id = random.randint(1, NUM_VERTICES)
            session.run("MATCH (n:Node {id: $id})-[:EDGE]->(t) RETURN t.id", id=node_id)
        r_time = time.time() - t0
        print(f"  100 reads in {r_time:.2f}s ({(r_time/100)*1000:.2f} ms/query)")
        
    driver.close()

if __name__ == "__main__":
    try:
        bench_db("bolt://localhost:7687", "Memgraph")
    except Exception as e:
        print(f"Memgraph error: {e}")
        
    try:
        bench_db("bolt://localhost:7688", "Neo4j")
    except Exception as e:
        print(f"Neo4j error: {e}")
