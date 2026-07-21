import dotenv from "dotenv";
dotenv.config();
export function loadSapiolaConfig() {
    const target = process.env.SAPIOLA_GRAPH_TARGET || "localhost:50051";
    return {
        graphGrpcTarget: target,
    };
}
