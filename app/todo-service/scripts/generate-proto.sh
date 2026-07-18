#!/usr/bin/env bash
set -euo pipefail

service_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
contracts_dir="$(cd "${service_dir}/../contracts" && pwd)"

export PATH="$(go env GOPATH)/bin:${PATH}"

if ! command -v protoc >/dev/null; then
  echo "protoc is required to generate the Go gRPC code" >&2
  exit 1
fi
if ! command -v protoc-gen-go >/dev/null; then
  go install google.golang.org/protobuf/cmd/protoc-gen-go@v1.36.6
fi
if ! command -v protoc-gen-go-grpc >/dev/null; then
  go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@v1.5.1
fi

rm -rf "${service_dir}/internal/gen"
protoc -I "${contracts_dir}" \
  --go_out="${service_dir}" \
  --go_opt=module=github.com/downMark/github-profile/app/todo-service \
  --go-grpc_out="${service_dir}" \
  --go-grpc_opt=module=github.com/downMark/github-profile/app/todo-service \
  "${contracts_dir}/profile/v1/profile.proto"
