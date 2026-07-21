import dotenv from "dotenv";

dotenv.config();

export interface SapiolaConfig {
  graphGrpcTarget: string;
}

export function loadSapiolaConfig(): SapiolaConfig {
  const target = process.env.SAPIOLA_GRAPH_TARGET || "localhost:50051";
  return {
    graphGrpcTarget: target,
  };
}
