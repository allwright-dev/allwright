#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bin_dir="$repo_root/go/.bin"
proto_root="$repo_root/proto"
go_root="$repo_root/go"
go_output_root="$go_root"

mkdir -p "$bin_dir"

if [[ ! -x "$bin_dir/protoc-gen-go" ]]; then
  GOBIN="$bin_dir" go install google.golang.org/protobuf/cmd/protoc-gen-go@v1.36.10
fi
if [[ ! -x "$bin_dir/protoc-gen-go-grpc" ]]; then
  GOBIN="$bin_dir" go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@v1.5.1
fi

PATH="$bin_dir:$PATH" protoc \
  -I "$proto_root" \
  --go_out="$go_output_root" \
  --go_opt=paths=import \
  --go_opt=module=allwright.dev \
  --go-grpc_out="$go_output_root" \
  --go-grpc_opt=paths=import \
  --go-grpc_opt=module=allwright.dev \
  "$proto_root/core/v1/browser.proto" \
  "$proto_root/core/v1/common.proto" \
  "$proto_root/core/v1/tab.proto" \
  "$proto_root/engine/v1/engine.proto" \
  "$proto_root/surfaces/mobile/v1/mobile.proto" \
  "$proto_root/surfaces/web/v1/web.proto"
