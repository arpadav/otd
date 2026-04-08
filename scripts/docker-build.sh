#!/bin/bash
# Build Docker image and optionally save as tarball
set -euo pipefail

docker build -t otd .

if [[ "${1:-}" == "--save" ]]; then
    docker save -o otd.tar otd:latest
    echo "Saved otd.tar"
fi
