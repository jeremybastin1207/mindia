#!/usr/bin/env bash

set -euo pipefail

echo "=== Mindia Docker Build Wizard ==="
echo

if ! command -v docker >/dev/null 2>&1; then
  echo "Error: docker is not installed or not on PATH."
  echo "Please install Docker and try again."
  exit 1
fi

DRY_RUN="${DRY_RUN:-0}"

echo "Select Dockerfile variant:"
echo "  [1] Standard image (Dockerfile)"
echo "  [2] ClamAV-enabled image (Dockerfile.with-clamav)"
read -rp "Choice [1/2, default 1]: " dockerfile_choice
dockerfile_choice="${dockerfile_choice:-1}"

case "$dockerfile_choice" in
  1)
    dockerfile="Dockerfile"
    default_tag="mindia"
    ;;
  2)
    dockerfile="Dockerfile.with-clamav"
    default_tag="mindia:clamav"
    ;;
  *)
    echo "Invalid choice, defaulting to standard image (Dockerfile)."
    dockerfile="Dockerfile"
    default_tag="mindia"
    ;;
esac

read -rp "Image tag/name [default: ${default_tag}]: " image_tag
image_tag="${image_tag:-$default_tag}"

echo
echo "Select cargo feature profile for mindia-api:"
echo "  [1] default  (full features, recommended)"
echo "  [2] minimal  (uses Dockerfile's minimal profile)"
read -rp "Choice [1/2, default 1]: " feature_choice
feature_choice="${feature_choice:-1}"

case "$feature_choice" in
  1)
    mindia_features="default"
    ;;
  2)
    mindia_features="minimal"
    ;;
  *)
    echo "Invalid choice, defaulting to 'default' features."
    mindia_features="default"
    ;;
esac

echo
read -rp "Use an env file with docker run? [y/N, default N]: " use_env_file
use_env_file="${use_env_file:-N}"
env_file_flag=""

if [[ "$use_env_file" =~ ^[Yy]$ ]]; then
  read -rp "Path to env file [default: .env]: " env_file_path
  env_file_path="${env_file_path:-.env}"
  env_file_flag="--env-file ${env_file_path}"
fi

echo
read -rp "Host port to map to container port 3000 [default: 3000]: " host_port_3000
host_port_3000="${host_port_3000:-3000}"

echo
read -rp "Also expose container port 8080? [y/N, default N]: " expose_8080
expose_8080="${expose_8080:-N}"
port_8080_flag=""

if [[ "$expose_8080" =~ ^[Yy]$ ]]; then
  read -rp "Host port to map to container port 8080 [default: 8080]: " host_port_8080
  host_port_8080="${host_port_8080:-8080}"
  port_8080_flag="-p ${host_port_8080}:8080"
fi

echo
echo "Summary:"
echo "  Dockerfile:        ${dockerfile}"
echo "  Image tag:         ${image_tag}"
echo "  MINDIA_FEATURES:   ${mindia_features}"
echo "  Host port -> 3000: ${host_port_3000}"
if [[ -n "$port_8080_flag" ]]; then
  echo "  Host port -> 8080: ${host_port_8080}"
else
  echo "  Port 8080:         not exposed"
fi
if [[ -n "$env_file_flag" ]]; then
  echo "  Env file:          ${env_file_path}"
else
  echo "  Env file:          none"
fi

echo
read -rp "Proceed with docker build? [Y/n, default Y]: " confirm
confirm="${confirm:-Y}"

if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
  echo "Aborted by user."
  exit 0
fi

build_cmd=(docker build -f "$dockerfile" -t "$image_tag" --build-arg "MINDIA_FEATURES=${mindia_features}" .)

echo
echo "Running docker build:"
printf '  %q ' "${build_cmd[@]}"
echo

if [[ "$DRY_RUN" == "1" ]]; then
  echo
  echo "DRY_RUN=1 set – not executing docker build."
else
  "${build_cmd[@]}"
fi

echo
echo "Build step completed."
echo
echo "Example docker run command:"
run_cmd=(docker run -d -p "${host_port_3000}:3000")
if [[ -n "$port_8080_flag" ]]; then
  run_cmd+=($port_8080_flag)
fi
if [[ -n "$env_file_flag" ]]; then
  run_cmd+=($env_file_flag)
fi
run_cmd+=("$image_tag")

printf '  %q ' "${run_cmd[@]}"
echo

echo
echo "You can copy and adjust this command as needed."

