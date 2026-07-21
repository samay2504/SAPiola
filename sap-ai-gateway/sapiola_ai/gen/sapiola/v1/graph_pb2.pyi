from google.protobuf import struct_pb2 as _struct_pb2
from sapiola.v1 import common_pb2 as _common_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class GraphNode(_message.Message):
    __slots__ = ("id", "label", "properties")
    class PropertiesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    ID_FIELD_NUMBER: _ClassVar[int]
    LABEL_FIELD_NUMBER: _ClassVar[int]
    PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    id: _common_pb2.NodeId
    label: str
    properties: _containers.ScalarMap[str, str]
    def __init__(self, id: _Optional[_Union[_common_pb2.NodeId, _Mapping]] = ..., label: _Optional[str] = ..., properties: _Optional[_Mapping[str, str]] = ...) -> None: ...

class GraphEdge(_message.Message):
    __slots__ = ("from_id", "to_id", "label", "properties")
    class PropertiesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    FROM_ID_FIELD_NUMBER: _ClassVar[int]
    TO_ID_FIELD_NUMBER: _ClassVar[int]
    LABEL_FIELD_NUMBER: _ClassVar[int]
    PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    from_id: _common_pb2.NodeId
    to_id: _common_pb2.NodeId
    label: str
    properties: _containers.ScalarMap[str, str]
    def __init__(self, from_id: _Optional[_Union[_common_pb2.NodeId, _Mapping]] = ..., to_id: _Optional[_Union[_common_pb2.NodeId, _Mapping]] = ..., label: _Optional[str] = ..., properties: _Optional[_Mapping[str, str]] = ...) -> None: ...

class QueryRow(_message.Message):
    __slots__ = ("columns",)
    class ColumnsEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    COLUMNS_FIELD_NUMBER: _ClassVar[int]
    columns: _containers.ScalarMap[str, str]
    def __init__(self, columns: _Optional[_Mapping[str, str]] = ...) -> None: ...

class CypherQueryRequest(_message.Message):
    __slots__ = ("cypher", "limit")
    CYPHER_FIELD_NUMBER: _ClassVar[int]
    LIMIT_FIELD_NUMBER: _ClassVar[int]
    cypher: str
    limit: int
    def __init__(self, cypher: _Optional[str] = ..., limit: _Optional[int] = ...) -> None: ...

class CypherQueryResponse(_message.Message):
    __slots__ = ("rows", "execution_ms", "path", "error")
    ROWS_FIELD_NUMBER: _ClassVar[int]
    EXECUTION_MS_FIELD_NUMBER: _ClassVar[int]
    PATH_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    rows: _containers.RepeatedCompositeFieldContainer[QueryRow]
    execution_ms: int
    path: str
    error: _common_pb2.Error
    def __init__(self, rows: _Optional[_Iterable[_Union[QueryRow, _Mapping]]] = ..., execution_ms: _Optional[int] = ..., path: _Optional[str] = ..., error: _Optional[_Union[_common_pb2.Error, _Mapping]] = ...) -> None: ...

class ListLabelsRequest(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class ListLabelsResponse(_message.Message):
    __slots__ = ("node_labels", "edge_labels")
    NODE_LABELS_FIELD_NUMBER: _ClassVar[int]
    EDGE_LABELS_FIELD_NUMBER: _ClassVar[int]
    node_labels: _containers.RepeatedScalarFieldContainer[str]
    edge_labels: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, node_labels: _Optional[_Iterable[str]] = ..., edge_labels: _Optional[_Iterable[str]] = ...) -> None: ...

class QueryCandidatesRequest(_message.Message):
    __slots__ = ("domain", "query")
    DOMAIN_FIELD_NUMBER: _ClassVar[int]
    QUERY_FIELD_NUMBER: _ClassVar[int]
    domain: str
    query: str
    def __init__(self, domain: _Optional[str] = ..., query: _Optional[str] = ...) -> None: ...

class CandidateInfo(_message.Message):
    __slots__ = ("node_id", "label", "sap_key", "pointer_path")
    NODE_ID_FIELD_NUMBER: _ClassVar[int]
    LABEL_FIELD_NUMBER: _ClassVar[int]
    SAP_KEY_FIELD_NUMBER: _ClassVar[int]
    POINTER_PATH_FIELD_NUMBER: _ClassVar[int]
    node_id: int
    label: str
    sap_key: str
    pointer_path: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, node_id: _Optional[int] = ..., label: _Optional[str] = ..., sap_key: _Optional[str] = ..., pointer_path: _Optional[_Iterable[str]] = ...) -> None: ...

class QueryCandidatesResponse(_message.Message):
    __slots__ = ("candidates", "error")
    CANDIDATES_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    candidates: _containers.RepeatedCompositeFieldContainer[CandidateInfo]
    error: _common_pb2.Error
    def __init__(self, candidates: _Optional[_Iterable[_Union[CandidateInfo, _Mapping]]] = ..., error: _Optional[_Union[_common_pb2.Error, _Mapping]] = ...) -> None: ...
