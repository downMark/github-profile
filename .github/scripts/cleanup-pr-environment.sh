#!/usr/bin/env bash
set -euo pipefail

required=(AWS_REGION PR_NUMBER ECS_CLUSTER PRIVATE_SUBNETS ECS_SECURITY_GROUP ECR_REPOSITORY RESOURCE_PREFIX)
for name in "${required[@]}"; do
  if [[ -z "${!name:-}" ]]; then
    echo "Missing GitHub repository variable: ${name}" >&2
    exit 1
  fi
done

if [[ ! "${PR_NUMBER}" =~ ^[0-9]+$ ]]; then
  echo "PR_NUMBER must contain digits only" >&2
  exit 1
fi

STACK_NAME="${RESOURCE_PREFIX}-pr-${PR_NUMBER}"

stack_status="$(aws cloudformation describe-stacks \
  --stack-name "${STACK_NAME}" \
  --query 'Stacks[0].StackStatus' \
  --output text 2>/dev/null || true)"

if [[ -n "${stack_status}" ]]; then
  task_definition="$(aws cloudformation describe-stacks \
    --stack-name "${STACK_NAME}" \
    --query "Stacks[0].Outputs[?OutputKey=='TaskDefinitionArn'].OutputValue" \
    --output text)"

  if [[ -n "${task_definition}" ]]; then
    cleanup_task="$(aws ecs run-task \
      --cluster "${ECS_CLUSTER}" \
      --task-definition "${task_definition}" \
      --launch-type FARGATE \
      --network-configuration "awsvpcConfiguration={subnets=[${PRIVATE_SUBNETS}],securityGroups=[${ECS_SECURITY_GROUP}],assignPublicIp=DISABLED}" \
      --overrides '{"containerOverrides":[{"name":"backend","environment":[{"name":"DB_SCHEMA_ACTION","value":"drop"}]}]}' \
      --query 'tasks[0].taskArn' \
      --output text)"

    if [[ -z "${cleanup_task}" || "${cleanup_task}" == "None" ]]; then
      echo "Failed to start database schema cleanup task" >&2
      exit 1
    fi

    aws ecs wait tasks-stopped --cluster "${ECS_CLUSTER}" --tasks "${cleanup_task}"
    cleanup_exit_code="$(aws ecs describe-tasks \
      --cluster "${ECS_CLUSTER}" \
      --tasks "${cleanup_task}" \
      --query 'tasks[0].containers[?name==`backend`].exitCode | [0]' \
      --output text)"
    if [[ "${cleanup_exit_code}" != "0" ]]; then
      echo "Schema cleanup task failed with exit code ${cleanup_exit_code}; stack deletion stopped" >&2
      exit 1
    fi
  fi

  aws cloudformation delete-stack --stack-name "${STACK_NAME}"
  aws cloudformation wait stack-delete-complete --stack-name "${STACK_NAME}"
fi

image_ids="$(aws ecr describe-images \
  --repository-name "${ECR_REPOSITORY}" \
  --query "imageDetails[?imageTags[?starts_with(@, 'pr-${PR_NUMBER}-')]].imageDigest" \
  --output text)"

for digest in ${image_ids}; do
  aws ecr batch-delete-image \
    --repository-name "${ECR_REPOSITORY}" \
    --image-ids imageDigest="${digest}" >/dev/null
done

echo "PR ${PR_NUMBER} database schema, CloudFormation stack, and ECR images were cleaned up"
