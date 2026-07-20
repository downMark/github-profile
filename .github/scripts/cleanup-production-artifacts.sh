#!/usr/bin/env bash
set -euo pipefail

required=(AWS_REGION ECS_CLUSTER RESOURCE_PREFIX AUTH_ECR_REPOSITORY PROFILE_ECR_REPOSITORY TODO_ECR_REPOSITORY)
for name in "${required[@]}"; do [[ -n "${!name:-}" ]] || { echo "Missing required value: ${name}" >&2; exit 1; }; done

stack="${RESOURCE_PREFIX}-prod"
output() { aws cloudformation describe-stacks --stack-name "${stack}" --query "Stacks[0].Outputs[?OutputKey=='$1'].OutputValue" --output text; }
services=("$(output AuthServiceName)" "$(output AuthGreenServiceName)" "$(output ProfileServiceName)" "$(output ProfileGreenServiceName)" "$(output TodoServiceName)" "$(output TodoGreenServiceName)")
protected_digests="$(mktemp)"
trap 'rm -f "${protected_digests}"' EXIT

for service in "${services[@]}"; do
  td="$(aws ecs describe-services --cluster "${ECS_CLUSTER}" --services "${service}" --query 'services[0].taskDefinition' --output text)"
  while read -r image; do
    [[ -n "${image}" ]] || continue
    path="${image#*.amazonaws.com/}"
    repository="${path%%[:@]*}"
    if [[ "${image}" == *@sha256:* ]]; then
      digest="${image##*@}"
    else
      tag="${image##*:}"
      digest="$(aws ecr describe-images --repository-name "${repository}" --image-ids imageTag="${tag}" --query 'imageDetails[0].imageDigest' --output text)"
    fi
    printf '%s %s\n' "${repository}" "${digest}" >>"${protected_digests}"
  done < <(aws ecs describe-task-definition --task-definition "${td}" --query 'taskDefinition.containerDefinitions[].image' --output text | tr '\t' '\n')
done

cutoff="$(date -u -d '-24 hours' +%s)"
for repository in "${AUTH_ECR_REPOSITORY}" "${PROFILE_ECR_REPOSITORY}" "${TODO_ECR_REPOSITORY}"; do
  delete_ids='[]'
  while IFS=$'\t' read -r digest pushed_at; do
    [[ -n "${digest}" && -n "${pushed_at}" ]] || continue
    pushed_epoch="$(date -u -d "${pushed_at}" +%s)"
    if (( pushed_epoch < cutoff )) && ! grep -Fqx "${repository} ${digest}" "${protected_digests}"; then
      delete_ids="$(jq --arg digest "${digest}" '. + [{imageDigest:$digest}]' <<<"${delete_ids}")"
    fi
  done < <(aws ecr describe-images --repository-name "${repository}" --filter tagStatus=ANY --output json | jq -r '.imageDetails[] | [.imageDigest,.imagePushedAt] | @tsv')
  if [[ "$(jq length <<<"${delete_ids}")" -gt 0 ]]; then
    tmp_ids="$(mktemp)"; printf '%s' "${delete_ids}" >"${tmp_ids}"
    aws ecr batch-delete-image --repository-name "${repository}" --image-ids "file://${tmp_ids}"
    rm -f "${tmp_ids}"
  fi
done
