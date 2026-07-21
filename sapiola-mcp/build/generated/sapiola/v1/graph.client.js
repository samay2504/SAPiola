import { GraphService } from "./graph";
import { stackIntercept } from "@protobuf-ts/runtime-rpc";
// ─── Graph API service ───────────────────────────────────────────────────────
/**
 * @generated from protobuf service sapiola.v1.GraphService
 */
export class GraphServiceClient {
    _transport;
    typeName = GraphService.typeName;
    methods = GraphService.methods;
    options = GraphService.options;
    constructor(_transport) {
        this._transport = _transport;
    }
    /**
     * Execute a Cypher-subset query (INV-8).
     *
     * @generated from protobuf rpc: Query
     */
    query(input, options) {
        const method = this.methods[0], opt = this._transport.mergeOptions(options);
        return stackIntercept("unary", this._transport, method, opt, input);
    }
    /**
     * Introspect available node and edge labels.
     *
     * @generated from protobuf rpc: ListLabels
     */
    listLabels(input, options) {
        const method = this.methods[1], opt = this._transport.mergeOptions(options);
        return stackIntercept("unary", this._transport, method, opt, input);
    }
    /**
     * Fast-path RAG candidate retrieval (Phase 3).
     *
     * @generated from protobuf rpc: QueryCandidates
     */
    queryCandidates(input, options) {
        const method = this.methods[2], opt = this._transport.mergeOptions(options);
        return stackIntercept("unary", this._transport, method, opt, input);
    }
}
