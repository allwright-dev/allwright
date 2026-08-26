from __future__ import annotations

import queue
from typing import Any, Callable, Iterator

import grpc

from ._types import AllwrightError

_STREAM_SENTINEL = object()


class StreamHandle:
    def __init__(self, stream_factory: Callable[[Iterator[Any]], Iterator[Any]]) -> None:
        self._queue: queue.Queue[Any] = queue.Queue()
        self._responses = stream_factory(self._request_iterator())
        self._send_closed = False

    def send(self, message: Any) -> None:
        if self._send_closed:
            raise AllwrightError("cannot send on a closed stream")
        self._queue.put(message)

    def recv(self, action: str) -> Any:
        try:
            return next(self._responses)
        except StopIteration as exc:
            raise AllwrightError(f"{action}: stream ended unexpectedly") from exc
        except grpc.RpcError as exc:
            raise AllwrightError(f"{action}: {exc}") from exc

    def close_send(self) -> None:
        if self._send_closed:
            return
        self._send_closed = True
        self._queue.put(_STREAM_SENTINEL)

    def _request_iterator(self) -> Iterator[Any]:
        while True:
            item = self._queue.get()
            if item is _STREAM_SENTINEL:
                return
            yield item


class RuntimeClient:
    def __init__(self, server_addr: str, stub_factory: Any) -> None:
        self.channel = grpc.insecure_channel(server_addr)
        self.stub = stub_factory(self.channel)

    def close(self) -> None:
        self.channel.close()
