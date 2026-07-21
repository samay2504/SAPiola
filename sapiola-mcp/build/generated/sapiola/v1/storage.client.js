import { StorageService } from "./storage";
import { stackIntercept } from "@protobuf-ts/runtime-rpc";
/**
 * @generated from protobuf service sapiola.v1.StorageService
 */
export class StorageServiceClient {
    _transport;
    typeName = StorageService.typeName;
    methods = StorageService.methods;
    options = StorageService.options;
    constructor(_transport) {
        this._transport = _transport;
    }
    /**
     * @generated from protobuf rpc: PutEdge
     */
    putEdge(input, options) {
        const method = this.methods[0], opt = this._transport.mergeOptions(options);
        return stackIntercept("unary", this._transport, method, opt, input);
    }
    /**
     * @generated from protobuf rpc: GetNeighbors
     */
    getNeighbors(input, options) {
        const method = this.methods[1], opt = this._transport.mergeOptions(options);
        return stackIntercept("unary", this._transport, method, opt, input);
    }
}
