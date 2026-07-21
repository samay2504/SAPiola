import { CdcIngest } from "./source";
import { stackIntercept } from "@protobuf-ts/runtime-rpc";
/**
 * @generated from protobuf service sapiola.v1.CdcIngest
 */
export class CdcIngestClient {
    _transport;
    typeName = CdcIngest.typeName;
    methods = CdcIngest.methods;
    options = CdcIngest.options;
    constructor(_transport) {
        this._transport = _transport;
    }
    /**
     * Unary publish for simple clients
     *
     * @generated from protobuf rpc: PublishEvent
     */
    publishEvent(input, options) {
        const method = this.methods[0], opt = this._transport.mergeOptions(options);
        return stackIntercept("unary", this._transport, method, opt, input);
    }
    /**
     * Client streaming for high-throughput batch ingestion
     *
     * @generated from protobuf rpc: PublishEventsStream
     */
    publishEventsStream(options) {
        const method = this.methods[1], opt = this._transport.mergeOptions(options);
        return stackIntercept("clientStreaming", this._transport, method, opt);
    }
}
