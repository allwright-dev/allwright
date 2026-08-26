from __future__ import annotations

import os
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
    try:
        yield
    finally:
        os.chdir(previous)


with _proto_cwd(PROTO_ROOT):
    engine_pb2, engine_pb2_grpc = grpc.protos_and_services(str(PROTO_RELATIVE_PATH))
