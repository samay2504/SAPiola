import { WireType } from "@protobuf-ts/runtime";
import { UnknownFieldHandler } from "@protobuf-ts/runtime";
import { reflectionMergePartial } from "@protobuf-ts/runtime";
import { MessageType } from "@protobuf-ts/runtime";
// @generated message type with reflection information, may provide speed optimized methods
class Value$Type extends MessageType {
    constructor() {
        super("sapiola.v1.Value", [
            { no: 1, name: "string_value", kind: "scalar", oneof: "kind", T: 9 /*ScalarType.STRING*/ },
            { no: 2, name: "int_value", kind: "scalar", oneof: "kind", T: 3 /*ScalarType.INT64*/, L: 0 /*LongType.BIGINT*/ },
            { no: 3, name: "double_value", kind: "scalar", oneof: "kind", T: 1 /*ScalarType.DOUBLE*/ },
            { no: 4, name: "bool_value", kind: "scalar", oneof: "kind", T: 8 /*ScalarType.BOOL*/ },
            { no: 5, name: "bytes_value", kind: "scalar", oneof: "kind", T: 12 /*ScalarType.BYTES*/ }
        ]);
    }
    create(value) {
        const message = globalThis.Object.create((this.messagePrototype));
        message.kind = { oneofKind: undefined };
        if (value !== undefined)
            reflectionMergePartial(this, message, value);
        return message;
    }
    internalBinaryRead(reader, length, options, target) {
        let message = target ?? this.create(), end = reader.pos + length;
        while (reader.pos < end) {
            let [fieldNo, wireType] = reader.tag();
            switch (fieldNo) {
                case /* string string_value */ 1:
                    message.kind = {
                        oneofKind: "stringValue",
                        stringValue: reader.string()
                    };
                    break;
                case /* int64 int_value */ 2:
                    message.kind = {
                        oneofKind: "intValue",
                        intValue: reader.int64().toBigInt()
                    };
                    break;
                case /* double double_value */ 3:
                    message.kind = {
                        oneofKind: "doubleValue",
                        doubleValue: reader.double()
                    };
                    break;
                case /* bool bool_value */ 4:
                    message.kind = {
                        oneofKind: "boolValue",
                        boolValue: reader.bool()
                    };
                    break;
                case /* bytes bytes_value */ 5:
                    message.kind = {
                        oneofKind: "bytesValue",
                        bytesValue: reader.bytes()
                    };
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
    internalBinaryWrite(message, writer, options) {
        /* string string_value = 1; */
        if (message.kind.oneofKind === "stringValue")
            writer.tag(1, WireType.LengthDelimited).string(message.kind.stringValue);
        /* int64 int_value = 2; */
        if (message.kind.oneofKind === "intValue")
            writer.tag(2, WireType.Varint).int64(message.kind.intValue);
        /* double double_value = 3; */
        if (message.kind.oneofKind === "doubleValue")
            writer.tag(3, WireType.Bit64).double(message.kind.doubleValue);
        /* bool bool_value = 4; */
        if (message.kind.oneofKind === "boolValue")
            writer.tag(4, WireType.Varint).bool(message.kind.boolValue);
        /* bytes bytes_value = 5; */
        if (message.kind.oneofKind === "bytesValue")
            writer.tag(5, WireType.LengthDelimited).bytes(message.kind.bytesValue);
        let u = options.writeUnknownFields;
        if (u !== false)
            (u == true ? UnknownFieldHandler.onWrite : u)(this.typeName, message, writer);
        return writer;
    }
}
/**
 * @generated MessageType for protobuf message sapiola.v1.Value
 */
export const Value = new Value$Type();
// @generated message type with reflection information, may provide speed optimized methods
class Error$Type extends MessageType {
    constructor() {
        super("sapiola.v1.Error", [
            { no: 1, name: "code", kind: "scalar", T: 5 /*ScalarType.INT32*/ },
            { no: 2, name: "message", kind: "scalar", T: 9 /*ScalarType.STRING*/ },
            { no: 3, name: "details", kind: "map", K: 9 /*ScalarType.STRING*/, V: { kind: "scalar", T: 9 /*ScalarType.STRING*/ } }
        ]);
    }
    create(value) {
        const message = globalThis.Object.create((this.messagePrototype));
        message.code = 0;
        message.message = "";
        message.details = {};
        if (value !== undefined)
            reflectionMergePartial(this, message, value);
        return message;
    }
    internalBinaryRead(reader, length, options, target) {
        let message = target ?? this.create(), end = reader.pos + length;
        while (reader.pos < end) {
            let [fieldNo, wireType] = reader.tag();
            switch (fieldNo) {
                case /* int32 code */ 1:
                    message.code = reader.int32();
                    break;
                case /* string message */ 2:
                    message.message = reader.string();
                    break;
                case /* map<string, string> details */ 3:
                    this.binaryReadMap3(message.details, reader, options);
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
    binaryReadMap3(map, reader, options) {
        let len = reader.uint32(), end = reader.pos + len, key, val;
        while (reader.pos < end) {
            let [fieldNo, wireType] = reader.tag();
            switch (fieldNo) {
                case 1:
                    key = reader.string();
                    break;
                case 2:
                    val = reader.string();
                    break;
                default: throw new globalThis.Error("unknown map entry field for sapiola.v1.Error.details");
            }
        }
        map[key ?? ""] = val ?? "";
    }
    internalBinaryWrite(message, writer, options) {
        /* int32 code = 1; */
        if (message.code !== 0)
            writer.tag(1, WireType.Varint).int32(message.code);
        /* string message = 2; */
        if (message.message !== "")
            writer.tag(2, WireType.LengthDelimited).string(message.message);
        /* map<string, string> details = 3; */
        for (let k of globalThis.Object.keys(message.details))
            writer.tag(3, WireType.LengthDelimited).fork().tag(1, WireType.LengthDelimited).string(k).tag(2, WireType.LengthDelimited).string(message.details[k]).join();
        let u = options.writeUnknownFields;
        if (u !== false)
            (u == true ? UnknownFieldHandler.onWrite : u)(this.typeName, message, writer);
        return writer;
    }
}
/**
 * @generated MessageType for protobuf message sapiola.v1.Error
 */
export const Error = new Error$Type();
// @generated message type with reflection information, may provide speed optimized methods
class NodeId$Type extends MessageType {
    constructor() {
        super("sapiola.v1.NodeId", [
            { no: 1, name: "id", kind: "scalar", T: 3 /*ScalarType.INT64*/, L: 0 /*LongType.BIGINT*/ }
        ]);
    }
    create(value) {
        const message = globalThis.Object.create((this.messagePrototype));
        message.id = 0n;
        if (value !== undefined)
            reflectionMergePartial(this, message, value);
        return message;
    }
    internalBinaryRead(reader, length, options, target) {
        let message = target ?? this.create(), end = reader.pos + length;
        while (reader.pos < end) {
            let [fieldNo, wireType] = reader.tag();
            switch (fieldNo) {
                case /* int64 id */ 1:
                    message.id = reader.int64().toBigInt();
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
    internalBinaryWrite(message, writer, options) {
        /* int64 id = 1; */
        if (message.id !== 0n)
            writer.tag(1, WireType.Varint).int64(message.id);
        let u = options.writeUnknownFields;
        if (u !== false)
            (u == true ? UnknownFieldHandler.onWrite : u)(this.typeName, message, writer);
        return writer;
    }
}
/**
 * @generated MessageType for protobuf message sapiola.v1.NodeId
 */
export const NodeId = new NodeId$Type();
