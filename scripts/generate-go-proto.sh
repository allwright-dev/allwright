#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bin_dir="$repo_root/go/.bin"
proto_root="$repo_root/proto"
go_root="$repo_root/go"
go_output_root="$go_root/gen/allwright"

mkdir -p "$bin_dir"

GOBIN="$bin_dir" go install google.golang.org/protobuf/cmd/protoc-gen-go@v1.36.10
GOBIN="$bin_dir" go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@v1.5.1

PATH="$bin_dir:$PATH" protoc \
  -I "$proto_root" \
  --go_out="$go_output_root" \
  --go_opt=paths=source_relative \
  --go-grpc_out="$go_output_root" \
  --go-grpc_opt=paths=source_relative \
  "$proto_root/engine/v1/engine.proto"
