#!/bin/bash
#
# build.sh(1)
#

[[ -n $DEBUG ]] && set -x
set -eu -o pipefail

# build parameters
readonly REGION=${AWS_DEFAULT_REGION:-"eu-central-1"}
readonly IMAGE_NAME='eks-version-exporter'
readonly BUILD_NUMBER=${1:-"N/A"}
readonly BUILD_SOURCES_DIRECTORY=${2:-${PWD}}

build_container_image() {
    echo "Building container image..."
    ls
    docker build -t ${IMAGE_NAME} -f Dockerfile .
}

push_container_image() {
    echo "Login to docker..."
    $(aws ecr get-login --no-include-email)

    account_id=$(aws sts get-caller-identity --output text --query 'Account')
    image_name="${account_id}.dkr.ecr.${REGION}.amazonaws.com/ded/${IMAGE_NAME}:${BUILD_NUMBER}"

    echo "Tagging container image..."
    docker tag ${IMAGE_NAME}:latest ${image_name}

    echo "Pushing container image to ECR..."
    docker push ${image_name}
}


build_container_image

if [[ "${BUILD_NUMBER}" != "N/A" ]]; then
    push_container_image
fi
