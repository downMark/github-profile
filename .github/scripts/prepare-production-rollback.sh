#!/usr/bin/env bash
set -euo pipefail

required=(AWS_REGION ECS_CLUSTER RESOURCE_PREFIX RELEASE_MANIFEST_BUCKET ROLLBACK_RELEASE_ID ROLLBACK_SERVICES)
for name in "${required[@]}"; do [[ -n "${!name:-}" ]] || { echo "Missing required value: ${name}" >&2; exit 1; }; done

STACK_NAME="${RESOURCE_PREFIX}-prod"
tmp_manifest="$(mktemp)"
trap 'rm -f "${tmp_manifest}"' EXIT
aws s3 cp "s3://${RELEASE_MANIFEST_BUCKET}/releases/${ROLLBACK_RELEASE_ID}.json" "${tmp_manifest}"
manifest="$(<"${tmp_manifest}")"
jq -e '.schemaVersion==1 and .rollbackCompatible==true' <<<"${manifest}" >/dev/null || { echo "Release is not rollback compatible" >&2; exit 1; }
expires_epoch="$(date -u -d "$(jq -r .expiresAt <<<"${manifest}")" +%s)"
(( $(date -u +%s) <= expires_epoch )) || { echo "The 24-hour rollback window has expired" >&2; exit 1; }

stack_output() { aws cloudformation describe-stacks --stack-name "${STACK_NAME}" --query "Stacks[0].Outputs[?OutputKey=='$1'].OutputValue" --output text; }
declare -A blue_service green_service blue_tg green_tg rule_arn container selected stable_slot candidate_slot
for service in auth profile todo; do selected[$service]=false; done
if [[ "${ROLLBACK_SERVICES}" == all ]]; then
  for service in auth profile todo; do jq -e --arg s "${service}" '.services[$s] != null' <<<"${manifest}" >/dev/null && selected[$service]=true; done
else
  IFS=',' read -ra requested <<<"${ROLLBACK_SERVICES}"
  for service in "${requested[@]}"; do
    [[ -n "${selected[$service]+x}" ]] || { echo "Unsupported service: ${service}" >&2; exit 1; }
    jq -e --arg s "${service}" '.services[$s] != null' <<<"${manifest}" >/dev/null || { echo "Release has no ${service} artifact" >&2; exit 1; }
    selected[$service]=true
  done
fi

blue_service[auth]="$(stack_output AuthServiceName)"; green_service[auth]="$(stack_output AuthGreenServiceName)"
blue_service[profile]="$(stack_output ProfileServiceName)"; green_service[profile]="$(stack_output ProfileGreenServiceName)"
blue_service[todo]="$(stack_output TodoServiceName)"; green_service[todo]="$(stack_output TodoGreenServiceName)"
blue_tg[auth]="$(stack_output AuthBlueTargetGroupArn)"; green_tg[auth]="$(stack_output AuthGreenTargetGroupArn)"
blue_tg[profile]="$(stack_output ProfileBlueTargetGroupArn)"; green_tg[profile]="$(stack_output ProfileGreenTargetGroupArn)"
blue_tg[todo]="$(stack_output TodoBlueTargetGroupArn)"; green_tg[todo]="$(stack_output TodoGreenTargetGroupArn)"
rule_arn[auth]="$(stack_output AuthListenerRuleArn)"; rule_arn[profile]="$(stack_output ProfileListenerRuleArn)"; rule_arn[todo]="$(stack_output TodoListenerRuleArn)"
container[auth]=auth; container[profile]=profile; container[todo]=todo

service_for_slot() { [[ "$2" == blue ]] && echo "${blue_service[$1]}" || echo "${green_service[$1]}"; }
tg_for_slot() { [[ "$2" == blue ]] && echo "${blue_tg[$1]}" || echo "${green_tg[$1]}"; }
profile_endpoint_for_slot() { [[ "$1" == blue ]] && echo 'profile-prod:50051' || echo 'profile-prod-green:50051'; }

for service in auth profile todo; do
  groups="$(aws elbv2 describe-rules --rule-arns "${rule_arn[$service]}" --query 'Rules[0].Actions[0].ForwardConfig.TargetGroups' --output json)"
  bw="$(jq -r --arg arn "${blue_tg[$service]}" '.[]|select(.TargetGroupArn==$arn)|.Weight' <<<"${groups}")"
  gw="$(jq -r --arg arn "${green_tg[$service]}" '.[]|select(.TargetGroupArn==$arn)|.Weight' <<<"${groups}")"
  if [[ "${bw}" == 100 && "${gw}" == 0 ]]; then stable_slot[$service]=blue; candidate_slot[$service]=green
  elif [[ "${bw}" == 0 && "${gw}" == 100 ]]; then stable_slot[$service]=green; candidate_slot[$service]=blue
  else echo "${service} has a canary in progress; finish it before preparing a historical rollback" >&2; exit 1; fi
done

profile_target_slot="${stable_slot[profile]}"
[[ "${selected[profile]}" == true ]] && profile_target_slot="${candidate_slot[profile]}"
prepared='{}'
rollback_id="rollback-${ROLLBACK_RELEASE_ID}-$(date -u +%Y%m%dT%H%M%SZ)"
for service in todo profile auth; do
  [[ "${selected[$service]}" == true ]] || continue
  digest_uri="$(jq -er --arg s "${service}" '.services[$s].imageDigestUri' <<<"${manifest}")"
  repository_path="${digest_uri#*.amazonaws.com/}"; repository="${repository_path%@*}"; digest="${digest_uri##*@}"
  aws ecr batch-get-image --repository-name "${repository}" --image-ids imageDigest="${digest}" --query 'images[0].imageId.imageDigest' --output text | grep -q '^sha256:' || { echo "Historical ${service} image is unavailable" >&2; exit 1; }
  stable_service="$(service_for_slot "${service}" "${stable_slot[$service]}")"
  candidate_service="$(service_for_slot "${service}" "${candidate_slot[$service]}")"
  stable_td="$(aws ecs describe-services --cluster "${ECS_CLUSTER}" --services "${stable_service}" --query 'services[0].taskDefinition' --output text)"
  definition="$(aws ecs describe-task-definition --task-definition "${stable_td}" --query taskDefinition --output json)"
  target_revision="$(jq -r .commit <<<"${manifest}")"
  profile_address="$(profile_endpoint_for_slot "${profile_target_slot}")"
  registration="$(jq --arg name "${container[$service]}" --arg image "${digest_uri}" --arg revision "${target_revision}" --arg profile "${profile_address}" '
    (.containerDefinitions[]|select(.name==$name)|.image)=$image |
    (.containerDefinitions[]|select(.name==$name)|.environment[]|select(.name=="SERVICE_REVISION")|.value)=$revision |
    if $name=="todo" then (.containerDefinitions[]|select(.name==$name)|.environment[]|select(.name=="PROFILE_GRPC_ADDR")|.value)=$profile else . end |
    {family,taskRoleArn,executionRoleArn,networkMode,containerDefinitions,volumes,placementConstraints,requiresCompatibilities,cpu,memory,runtimePlatform,ephemeralStorage} | with_entries(select(.value!=null))' <<<"${definition}")"
  tmp_td="$(mktemp)"; printf '%s' "${registration}" >"${tmp_td}"
  new_td="$(aws ecs register-task-definition --cli-input-json "file://${tmp_td}" --query 'taskDefinition.taskDefinitionArn' --output text)"; rm -f "${tmp_td}"
  aws ecs update-service --cluster "${ECS_CLUSTER}" --service "${candidate_service}" --task-definition "${new_td}" --desired-count 1 >/dev/null
  aws ecs wait services-stable --cluster "${ECS_CLUSTER}" --services "${candidate_service}"
  health="$(aws elbv2 describe-target-health --target-group-arn "$(tg_for_slot "${service}" "${candidate_slot[$service]}")" --query 'TargetHealthDescriptions[].TargetHealth.State' --output json)"
  jq -e 'length>0 and all(.=="healthy")' <<<"${health}" >/dev/null || { echo "Rollback candidate for ${service} is unhealthy" >&2; exit 1; }
  aws elbv2 add-tags --resource-arns "${rule_arn[$service]}" --tags Key=CandidateSlot,Value="${candidate_slot[$service]}" Key=ReleaseId,Value="${rollback_id}"
  prepared="$(jq --arg s "${service}" --arg slot "${candidate_slot[$service]}" --arg td "${new_td}" --arg image "${digest_uri}" '.+{($s):{candidateSlot:$slot,candidateTaskDefinitionArn:$td,imageDigestUri:$image}}' <<<"${prepared}")"
done

rollback_manifest="$(jq -n --arg id "${rollback_id}" --arg source "${ROLLBACK_RELEASE_ID}" --arg created "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --argjson services "${prepared}" '{schemaVersion:1,releaseId:$id,sourceReleaseId:$source,createdAt:$created,status:"ROLLBACK_CANDIDATE_PREPARED",services:$services}')"
printf '%s' "${rollback_manifest}" >"${tmp_manifest}"
aws s3 cp "${tmp_manifest}" "s3://${RELEASE_MANIFEST_BUCKET}/rollbacks/${rollback_id}.json" --sse AES256
echo "Prepared rollback ${rollback_id}; traffic remains on the current stable slots"
