// sap-streaming-gateway — Kafka/Redpanda consumer bridging CDC events to graph layer.
// Phase 0: consumer, deserializer, dispatcher skeletons.
// Note: Rust protobuf code generation (protoc-gen-prost/protoc-gen-tonic) requires
// the host to have the protoc plugins installed. Until that is resolved, we
// use prost::Message derive macros directly to decode the wire format.
// The go_package generated Go protos are the canonical generated source; the
// Rust side decodes the same wire format with manually-defined structs for now.

pub mod consumer;
pub mod deserializer;
pub mod dispatcher;

pub mod gen;
