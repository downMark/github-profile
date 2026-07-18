#!/usr/bin/env bash
set -euo pipefail

required=(AWS_REGION AWS_ROLE_ARN SHARED_CONFIG_PARAMETER)
for name in "${required[@]}"; do
  if [[ -z "${!name:-}" ]]; then
    echo "Missing GitHub repository variable: ${name}" >&2
    exit 1
  fi
done
