#!/usr/bin/env bash
set -euo pipefail

required=(AWS_REGION CODEBUILD_PROJECT HEAD_SHA ENVIRONMENT_ID PROFILE_ECR_REPOSITORY TODO_ECR_REPOSITORY AUTH_ECR_REPOSITORY)
for name in "${required[@]}"; do
  if [[ -z "${!name:-}" ]]; then
    echo "Missing required value: ${name}" >&2
    exit 1
  fi
done

if [[ ! "${ENVIRONMENT_ID}" =~ ^(prod|pr-[0-9]+)$ || ! "${HEAD_SHA}" =~ ^[0-9a-f]{40}$ ]]; then
  echo "ENVIRONMENT_ID or HEAD_SHA has an invalid format" >&2
  exit 1
fi

AWS_ACCOUNT_ID="$(aws sts get-caller-identity --query Account --output text)"
IMAGE_TAG="${ENVIRONMENT_ID}-${HEAD_SHA:0:12}"
REGISTRY="${AWS_ACCOUNT_ID}.dkr.ecr.${AWS_REGION}.amazonaws.com"
overrides="$(jq -cn \
  --arg account "${AWS_ACCOUNT_ID}" \
  --arg tag "${IMAGE_TAG}" \
  --arg auth "${AUTH_ECR_REPOSITORY}" \
  --arg profile "${PROFILE_ECR_REPOSITORY}" \
  --arg todo "${TODO_ECR_REPOSITORY}" \
  '[
    {name:"AWS_ACCOUNT_ID",value:$account,type:"PLAINTEXT"},
    {name:"IMAGE_TAG",value:$tag,type:"PLAINTEXT"},
    {name:"AUTH_ECR_REPOSITORY",value:$auth,type:"PLAINTEXT"},
    {name:"PROFILE_ECR_REPOSITORY",value:$profile,type:"PLAINTEXT"},
    {name:"TODO_ECR_REPOSITORY",value:$todo,type:"PLAINTEXT"}
  ]')"

batch_id="$(aws codebuild start-build-batch \
  --project-name "${CODEBUILD_PROJECT}" \
  --source-version "${HEAD_SHA}" \
  --environment-variables-override "${overrides}" \
  --query 'buildBatch.id' \
  --output text)"

echo "Started CodeBuild batch ${batch_id}"
while true; do
  status="$(aws codebuild batch-get-build-batches \
    --ids "${batch_id}" \
    --query 'buildBatches[0].buildBatchStatus' \
    --output text)"
  case "${status}" in
    SUCCEEDED) break ;;
    FAILED|FAULT|STOPPED|TIMED_OUT)
      aws codebuild batch-get-build-batches --ids "${batch_id}"
      echo "CodeBuild batch failed: ${status}" >&2
      exit 1
      ;;
    IN_PROGRESS) sleep 15 ;;
    *) echo "Unexpected CodeBuild batch status: ${status}" >&2; exit 1 ;;
  esac
done

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  {
    echo "auth-image-uri=${REGISTRY}/${AUTH_ECR_REPOSITORY}:${IMAGE_TAG}"
    echo "profile-image-uri=${REGISTRY}/${PROFILE_ECR_REPOSITORY}:${IMAGE_TAG}"
    echo "todo-image-uri=${REGISTRY}/${TODO_ECR_REPOSITORY}:${IMAGE_TAG}"
    echo "batch-id=${batch_id}"
  } >> "${GITHUB_OUTPUT}"
fi
