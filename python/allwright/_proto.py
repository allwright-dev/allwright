from __future__ import annotations

import importlib
import os
import sys
from contextlib import contextmanager
from pathlib import Path
from typing import Iterator

import grpc

DEFAULT_SERVER_ADDR = "127.0.0.1:50051"
SERVER_ADDR_ENV_VAR = "ALLWRIGHT_SERVER_ADDR"
PROTO_ROOT = Path(__file__).resolve().parent / "proto"
PROTO_RELATIVE_PATH = Path("engine") / "v1" / "engine.proto"


@contextmanager
def _proto_cwd(path: Path) -> Iterator[None]:
    previous = Path.cwd()
    os.chdir(path)
    sys.path.insert(0, str(path))
    try:
        yield
    finally:
        sys.path.remove(str(path))
        os.chdir(previous)


with _proto_cwd(PROTO_ROOT):
    engine_pb2, engine_pb2_grpc = grpc.protos_and_services(str(PROTO_RELATIVE_PATH))


# The service proto imports contracts from the core and surface layers. Python
# protobuf modules do not re-export ordinary imports, so assemble the internal
# facade used by the clients from the descriptors, without duplicating contracts.
def _export_contracts(descriptor, visited: set[str]) -> None:
    if descriptor.name in visited:
        return
    visited.add(descriptor.name)
    for dependency in descriptor.dependencies:
        _export_contracts(dependency, visited)
    module = importlib.import_module(descriptor.name.removesuffix(".proto").replace("/", ".") + "_pb2")
    for name in descriptor.message_types_by_name:
        setattr(engine_pb2, name, getattr(module, name))
    for name, enum in descriptor.enum_types_by_name.items():
        setattr(engine_pb2, name, getattr(module, name))
        for value in enum.values:
            setattr(engine_pb2, value.name, value.number)


_export_contracts(engine_pb2.DESCRIPTOR, set())
