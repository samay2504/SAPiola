import { WireType } from "@protobuf-ts/runtime";
import { UnknownFieldHandler } from "@protobuf-ts/runtime";
import { reflectionMergePartial } from "@protobuf-ts/runtime";
import { MessageType } from "@protobuf-ts/runtime";
import { Timestamp } from "../../google/protobuf/timestamp";
import { Value } from "./common";
/**
 * @generated from protobuf enum sapiola.v1.Operation
 */
export var Operation;
(function (Operation) {
    /**
     * @generated from protobuf enum value: OPERATION_UNSPECIFIED = 0;
     */
    Operation[Operation["UNSPECIFIED"] = 0] = "UNSPECIFIED";
    /**
     * @generated from protobuf enum value: OPERATION_INSERT = 1;
     */
    Operation[Operation["INSERT"] = 1] = "INSERT";
    /**
     * @generated from protobuf enum value: OPERATION_UPDATE = 2;
     */
    Operation[Operation["UPDATE"] = 2] = "UPDATE";
    /**
     * @generated from protobuf enum value: OPERATION_DELETE = 3;
     */
    Operation[Operation["DELETE"] = 3] = "DELETE";
})(Operation || (Operation = {}));
// @generated message type with reflection information, may provide speed optimized methods
class CdcEvent$Type extends MessageType {
    constructor() {
        super("sapiola.v1.CdcEvent", [
            { no: 1, name: "table", kind: "scalar", T: 9 /*ScalarType.STRING*/ },
            { no: 2, name: "primary_key", kind: "scalar", T: 9 /*ScalarType.STRING*/ },
            { no: 3, name: "operation", kind: "enum", T: () => ["sapiola.v1.Operation", Operation, "OPERATION_"] },
            { no: 4, name: "before", kind: "map", K: 9 /*ScalarType.STRING*/, V: { kind: "message", T: () => Value } },
            { no: 5, name: "after", kind: "map", K: 9 /*ScalarType.STRING*/, V: { kind: "message", T: () => Value } },
            { no: 6, name: "timestamp", kind: "message", T: () => Timestamp },
            { no: 7, name: "txid", kind: "scalar", T: 9 /*ScalarType.STRING*/ },
            { no: 8, name: "log_sequence_number", kind: "scalar", T: 3 /*ScalarType.INT64*/, L: 0 /*LongType.BIGINT*/ }
        ]);
    }
    create(value) {
        const message = globalThis.Object.create((this.messagePrototype));
        message.table = "";
        message.primaryKey = "";
        message.operation = 0;
        message.before = {};
        message.after = {};
        message.txid = "";
        message.logSequenceNumber = 0n;
        if (value !== undefined)
            reflectionMergePartial(this, message, value);
        return message;
    }
    internalBinaryRead(reader, length, options, target) {
        let message = target ?? this.create(), end = reader.pos + length;
        while (reader.pos < end) {
            let [fieldNo, wireType] = reader.tag();
            switch (fieldNo) {
                case /* string table */ 1:
                    message.table = reader.string();
                    break;
                case /* string primary_key */ 2:
                    message.primaryKey = reader.string();
                    break;
                case /* sapiola.v1.Operation operation */ 3:
                    message.operation = reader.int32();
                    break;
                case /* map<string, sapiola.v1.Value> before */ 4:
                    this.binaryReadMap4(message.before, reader, options);
                    break;
                case /* map<string, sapiola.v1.Value> after */ 5:
                    this.binaryReadMap5(message.after, reader, options);
                    break;
                case /* google.protobuf.Timestamp timestamp */ 6:
                    message.timestamp = Timestamp.internalBinaryRead(reader, reader.uint32(), options, message.timestamp);
                    break;
                case /* string txid */ 7:
                    message.txid = reader.string();
                    break;
                case /* int64 log_sequence_number */ 8:
                    message.logSequenceNumber = reader.int64().toBigInt();
                    break;
                default:
                    let u = options.readUnknownField;
                    if (u === "throw")
                        throw new globalThis.Error(`Unknown field ${fieldNo} (wire type ${wireType}) for ${this.typeName}`);
                    let d = reader.skip(wireType);
                    if (u !== false)
                        (u === true ? UnknownFieldHandler.onRead : u)(this.typeName, message, fieldNo, wireType, d);
            }
        }
        return message;
    }
    binaryReadMap4(map, reader, options) {
        let len = reader.uint32(), end = reader.pos + len, key, val;
        while (reader.pos < end) {
            let [fieldNo, wireType] = reader.tag();
            switch (fieldNo) {
                case 1:
                    key = reader.string();
                    break;
                case 2:
                    val = Value.internalBinaryRead(reader, reader.uint32(), options);
                    break;
                default: throw new globalThis.Error("unknown map entry field for sapiola.v1.CdcEvent.before");
            }
        }
        map[key ?? ""] = val ?? Value.create();
    }
    binaryReadMap5(map, reader, options) {
        let len = reader.uint32(), end = reader.pos + len, key, val;
        while (reader.pos < end) {
            let [fieldNo, wireType] = reader.tag();
            switch (fieldNo) {
                case 1:
                    key = reader.string();
                    break;
                case 2:
                    val = Value.internalBinaryRead(reader, reader.uint32(), options);
                    break;
                default: throw new globalThis.Error("unknown map entry field for sapiola.v1.CdcEvent.after");
            }
        }
        map[key ?? ""] = val ?? Value.create();
    }
    internalBinaryWrite(message, writer, options) {
        /* string table = 1; */
        if (message.table !== "")
            writer.tag(1, WireType.LengthDelimited).string(message.table);
        /* string primary_key = 2; */
        if (message.primaryKey !== "")
            writer.tag(2, WireType.LengthDelimited).string(message.primaryKey);
        /* sapiola.v1.Operation operation = 3; */
        if (message.operation !== 0)
            writer.tag(3, WireType.Varint).int32(message.operation);
        /* map<string, sapiola.v1.Value> before = 4; */
        for (let k of globalThis.Object.keys(message.before)) {
            writer.tag(4, WireType.LengthDelimited).fork().tag(1, WireType.LengthDelimited).string(k);
            writer.tag(2, WireType.LengthDelimited).fork();
            Value.internalBinaryWrite(message.before[k], writer, options);
            writer.join().join();
        }
        /* map<string, sapiola.v1.Value> after = 5; */
        for (let k of globalThis.Object.keys(message.after)) {
            writer.tag(5, WireType.LengthDelimited).fork().tag(1, WireType.LengthDelimited).string(k);
            writer.tag(2, WireType.LengthDelimited).fork();
            Value.internalBinaryWrite(message.after[k], writer, options);
            writer.join().join();
        }
        /* google.protobuf.Timestamp timestamp = 6; */
        if (message.timestamp)
            Timestamp.internalBinaryWrite(message.timestamp, writer.tag(6, WireType.LengthDelimited).fork(), options).join();
        /* string txid = 7; */
        if (message.txid !== "")
            writer.tag(7, WireType.LengthDelimited).string(message.txid);
        /* int64 log_sequence_number = 8; */
        if (message.logSequenceNumber !== 0n)
            writer.tag(8, WireType.Varint).int64(message.logSequenceNumber);
        let u = options.writeUnknownFields;
        if (u !== false)
            (u == true ? UnknownFieldHandler.onWrite : u)(this.typeName, message, writer);
        return writer;
    }
}
/**
 * @generated MessageType for protobuf message sapiola.v1.CdcEvent
 */
export const CdcEvent = new CdcEvent$Type();
