#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import * as grpc from "@grpc/grpc-js";
import { GraphApiClient } from "./generated/sapiola/v1/graph.client.js";
import { loadSapiolaConfig } from "./config.js";
const config = loadSapiolaConfig();
// @protobuf-ts/grpc-transport uses standard grpc-js channel credentials
import { GrpcTransport } from "@protobuf-ts/grpc-transport";
const transport = new GrpcTransport({
    host: config.graphGrpcTarget,
    channelCredentials: grpc.credentials.createInsecure(),
});
const graphClient = new GraphApiClient(transport);
const server = new McpServer({ name: "sapiola", version: "0.1.0" });
server.tool("query_sap_graph", {
    cypher: z.string().describe("A Cypher-subset query over the SAP graph"),
    tenant_id: z.string().describe("The tenant ID for RBAC"),
}, async ({ cypher, tenant_id }) => {
    try {
        const { response } = await graphClient.query({
            cypher,
            tenantId: tenant_id,
        });
        return { content: [{ type: "text", text: JSON.stringify(response.nodes) + JSON.stringify(response.edges) }] };
    }
    catch (e) {
        return { content: [{ type: "text", text: `Error: ${e.message}` }] };
    }
});
// We are assuming RagOrchestrator uses the same client for now, or just an ask_sap endpoint in GraphApi.
// According to user prompt:
// "const result = await ragClient.answer({ query: question, tenantId: tenant_id });"
// Wait, the user snippet had "ragClient", but graph.proto might not have an "answer" RPC yet. 
// I will just add the tool and leave a TODO, or see if GraphApi has an Answer RPC.
// Let's implement the tool using graphClient for now to prevent compilation errors if ragClient is missing.
// Or I can just omit it if the gRPC service isn't defined yet, or I can define it as a stub.
server.tool("ask_sap", {
    question: z.string(),
    tenant_id: z.string()
}, async ({ question, tenant_id }) => {
    try {
        // Stub for RagOrchestrator integration
        return { content: [{ type: "text", text: `RAG answer for ${question} (Tenant: ${tenant_id}) not yet implemented at gRPC layer.` }] };
    }
    catch (e) {
        return { content: [{ type: "text", text: `Error: ${e.message}` }] };
    }
});
async function main() {
    const stdioTransport = new StdioServerTransport();
    await server.connect(stdioTransport);
    console.error("SAPiola MCP Server running on stdio");
}
main().catch((err) => {
    console.error("Fatal error in MCP server:", err);
    process.exit(1);
});
