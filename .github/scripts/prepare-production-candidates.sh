#!/usr/bin/env bash
set -euo pipefail

required=(AWS_REGION ECS_CLUSTER RESOURCE_PREFIX RELEASE_MANIFEST_BUCKET HEAD_SHA AUTH_IMAGE_URI PROFILE_IMAGE_URI TODO_IMAGE_URI)
for name in "${required[@]}"; do
  [[ -n "${!name:-}" ]] || { echo "Missing required value: ${name}" >&2; exit 1; }
done

STACK_NAME="${RESOURCE_PREFIX}-prod"
RELEASE_ID="${HEAD_SHA:0:12}-$(date -u +%Y%m%dT%H%M%SZ)"
expires_at="$(date -u -v+24H +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u -d '+24 hours' +%Y-%m-%dT%H:%M:%SZ)"

stack_output() {
  aws cloudformation describe-stacks --stack-name "${STACK_NAME}" \
    --query "Stacks[0].Outputs[?OutputKey=='$1'].OutputValue" --output text
}

declare -A blue_service green_service blue_tg green_tg rule_arn image_uri container repository changed stable_slot candidate_slot in_progress
blue_service[auth]="$(stack_output AuthServiceName)"
green_service[auth]="$(stack_output AuthGreenServiceName)"
blue_service[profile]="$(stack_output ProfileServiceName)"
green_service[profile]="$(stack_output ProfileGreenServiceName)"
blue_service[todo]="$(stack_output TodoServiceName)"
green_service[todo]="$(stack_output TodoGreenServiceName)"
blue_tg[auth]="$(stack_output AuthBlueTargetGroupArn)"
green_tg[auth]="$(stack_output AuthGreenTargetGroupArn)"
blue_tg[profile]="$(stack_output ProfileBlueTargetGroupArn)"
green_tg[profile]="$(stack_output ProfileGreenTargetGroupArn)"
blue_tg[todo]="$(stack_output TodoBlueTargetGroupArn)"
green_tg[todo]="$(stack_output TodoGreenTargetGroupArn)"
rule_arn[auth]="$(stack_output AuthListenerRuleArn)"
rule_arn[profile]="$(stack_output ProfileListenerRuleArn)"
rule_arn[todo]="$(stack_output TodoListenerRuleArn)"
image_uri[auth]="${AUTH_IMAGE_URI}"; image_uri[profile]="${PROFILE_IMAGE_URI}"; image_uri[todo]="${TODO_IMAGE_URI}"
container[auth]=auth; container[profile]=profile; container[todo]=todo
repository[auth]="${AUTH_ECR_REPOSITORY}"; repository[profile]="${PROFILE_ECR_REPOSITORY}"; repository[todo]="${TODO_ECR_REPOSITORY}"

service_for_slot() { local service="$1" slot="$2"; [[ "${slot}" == blue ]] && echo "${blue_service[$service]}" || echo "${green_service[$service]}"; }
tg_for_slot() { local service="$1" slot="$2"; [[ "${slot}" == blue ]] && echo "${blue_tg[$service]}" || echo "${green_tg[$service]}"; }
profile_endpoint_for_slot() { [[ "$1" == blue ]] && echo 'profile-prod:50051' || echo 'profile-prod-green:50051'; }

detect_slots() {
  local service="$1" groups blue_weight green_weight
  groups="$(aws elbv2 describe-rules --rule-arns "${rule_arn[$service]}" --query 'Rules[0].Actions[0].ForwardConfig.TargetGroups' --output json)"
  blue_weight="$(jq -r --arg arn "${blue_tg[$service]}" '.[] | select(.TargetGroupArn==$arn) | .Weight' <<<"${groups}")"
  green_weight="$(jq -r --arg arn "${green_tg[$service]}" '.[] | select(.TargetGroupArn==$arn) | .Weight' <<<"${groups}")"
  if [[ "${blue_weight}" == 100 && "${green_weight}" == 0 ]]; then
    stable_slot[$service]=blue; candidate_slot[$service]=green; in_progress[$service]=false
  elif [[ "${blue_weight}" == 0 && "${green_weight}" == 100 ]]; then
    stable_slot[$service]=green; candidate_slot[$service]=blue; in_progress[$service]=false
  else
    tagged_candidate="$(aws elbv2 describe-tags --resource-arns "${rule_arn[$service]}" --query 'TagDescriptions[0].Tags[?Key==`CandidateSlot`].Value | [0]' --output text)"
    [[ "${tagged_candidate}" == blue || "${tagged_candidate}" == green ]] || { echo "${service} has mixed weights without a CandidateSlot tag" >&2; exit 1; }
    candidate_slot[$service]="${tagged_candidate}"
    [[ "${tagged_candidate}" == blue ]] && stable_slot[$service]=green || stable_slot[$service]=blue
    in_progress[$service]=true
  fi
}

stable_revision() {
  local service="$1" stable_service task_definition
  stable_service="$(service_for_slot "${service}" "${stable_slot[$service]}")"
  task_definition="$(aws ecs describe-services --cluster "${ECS_CLUSTER}" --services "${stable_service}" --query 'services[0].taskDefinition' --output text)"
  aws ecs describe-task-definition --task-definition "${task_definition}" \
    --query "taskDefinition.containerDefinitions[?name=='${container[$service]}'].environment[?name=='SERVICE_REVISION'].value | [0]" --output text
}

service_changed() {
  local service="$1" revision="$2"
  if [[ ! "${revision}" =~ ^[0-9a-f]{40}$ ]] || ! git cat-file -e "${revision}^{commit}" 2>/dev/null; then return 0; fi
  case "${service}" in
    auth) git diff --quiet "${revision}..${HEAD_SHA}" -- app/auth-service .codebuild/auth.yml app/compose.yaml || return 0 ;;
    profile) git diff --quiet "${revision}..${HEAD_SHA}" -- app/backend app/contracts .codebuild/profile.yml app/compose.yaml || return 0 ;;
    todo) git diff --quiet "${revision}..${HEAD_SHA}" -- app/todo-service app/contracts .codebuild/todo.yml app/compose.yaml || return 0 ;;
  esac
  return 1
}

for service in auth profile todo; do
  detect_slots "${service}"
  revision="$(stable_revision "${service}")"
  if service_changed "${service}" "${revision}"; then
    changed[$service]=true
    [[ "${in_progress[$service]}" == false ]] || { echo "${service} already has a canary in progress; finish or roll it back first" >&2; exit 1; }
  else changed[$service]=false; fi
done

if [[ "${changed[todo]}" == true && "${in_progress[profile]}" == true ]]; then
  echo "Todo cannot prepare a new candidate while Profile is mid-canary" >&2
  exit 1
fi

manifest_services='{}'
prepared=0
profile_target_slot="${stable_slot[profile]}"
[[ "${changed[profile]}" == true ]] && profile_target_slot="${candidate_slot[profile]}"

for service in auth profile todo; do
  [[ "${changed[$service]}" == true ]] || continue
  prepared=$((prepared + 1))
  stable_service="$(service_for_slot "${service}" "${stable_slot[$service]}")"
  candidate_service="$(service_for_slot "${service}" "${candidate_slot[$service]}")"
  stable_td="$(aws ecs describe-services --cluster "${ECS_CLUSTER}" --services "${stable_service}" --query 'services[0].taskDefinition' --output text)"
  definition="$(aws ecs describe-task-definition --task-definition "${stable_td}" --query taskDefinition --output json)"
  profile_address="$(profile_endpoint_for_slot "${profile_target_slot}")"
  registration="$(jq --arg name "${container[$service]}" --arg image "${image_uri[$service]}" --arg revision "${HEAD_SHA}" --arg profile "${profile_address}" '
    (.containerDefinitions[] | select(.name==$name) | .image)=$image |
    (.containerDefinitions[] | select(.name==$name) | .environment[] | select(.name=="SERVICE_REVISION") | .value)=$revision |
    if $name=="todo" then (.containerDefinitions[] | select(.name==$name) | .environment[] | select(.name=="PROFILE_GRPC_ADDR") | .value)=$profile else . end |
    {family,taskRoleArn,executionRoleArn,networkMode,containerDefinitions,volumes,placementConstraints,requiresCompatibilities,cpu,memory,runtimePlatform,ephemeralStorage} |
    with_entries(select(.value != null))' <<<"${definition}")"
  tmp_definition="$(mktemp)"
  printf '%s' "${registration}" > "${tmp_definition}"
  new_td="$(aws ecs register-task-definition --cli-input-json "file://${tmp_definition}" --query 'taskDefinition.taskDefinitionArn' --output text)"
  rm -f "${tmp_definition}"
  aws ecs update-service --cluster "${ECS_CLUSTER}" --service "${candidate_service}" --task-definition "${new_td}" --desired-count 1 >/dev/null
  aws ecs wait services-stable --cluster "${ECS_CLUSTER}" --services "${candidate_service}"
  target_health="$(aws elbv2 describe-target-health --target-group-arn "$(tg_for_slot "${service}" "${candidate_slot[$service]}")" --query 'TargetHealthDescriptions[].TargetHealth.State' --output json)"
  jq -e 'length > 0 and all(. == "healthy")' <<<"${target_health}" >/dev/null || { echo "${service} candidate targets are not healthy" >&2; exit 1; }
  aws elbv2 add-tags --resource-arns "${rule_arn[$service]}" --tags Key=CandidateSlot,Value="${candidate_slot[$service]}" Key=ReleaseId,Value="${RELEASE_ID}"
  image_digest="$(aws ecr describe-images --repository-name "${repository[$service]}" --image-ids imageTag="prod-${HEAD_SHA:0:12}" --query 'imageDetails[0].imageDigest' --output text)"
  digest_uri="${image_uri[$service]%:*}@${image_digest}"
  manifest_services="$(jq --arg service "${service}" --arg stableSlot "${stable_slot[$service]}" --arg candidateSlot "${candidate_slot[$service]}" \
    --arg stableTaskDefinitionArn "${stable_td}" --arg candidateTaskDefinitionArn "${new_td}" --arg imageDigestUri "${digest_uri}" \
    '. + {($service): {stableSlot:$stableSlot,candidateSlot:$candidateSlot,stableTaskDefinitionArn:$stableTaskDefinitionArn,candidateTaskDefinitionArn:$candidateTaskDefinitionArn,imageDigestUri:$imageDigestUri}}' <<<"${manifest_services}")"
done

if (( prepared == 0 )); then echo "No backend service changes relative to deployed revisions"; exit 0; fi
manifest="$(jq -n --arg releaseId "${RELEASE_ID}" --arg commit "${HEAD_SHA}" --arg createdAt "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg expiresAt "${expires_at}" --argjson services "${manifest_services}" \
  '{schemaVersion:1,releaseId:$releaseId,commit:$commit,createdAt:$createdAt,expiresAt:$expiresAt,databaseSchemaVersion:3,rollbackCompatible:true,status:"CANDIDATE_PREPARED",services:$services}')"
tmp_manifest="$(mktemp)"; printf '%s' "${manifest}" > "${tmp_manifest}"
aws s3 cp "${tmp_manifest}" "s3://${RELEASE_MANIFEST_BUCKET}/releases/${RELEASE_ID}.json" --sse AES256
rm -f "${tmp_manifest}"

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  echo "release-id=${RELEASE_ID}" >> "${GITHUB_OUTPUT}"
  echo "services=$(jq -r '.services | keys | join(",")' <<<"${manifest}")" >> "${GITHUB_OUTPUT}"
fi
echo "Prepared release ${RELEASE_ID}: $(jq -r '.services | keys | join(", ")' <<<"${manifest}")"
