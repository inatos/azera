#!/bin/bash
set -e

# Wait for CockroachDB to be ready
echo "Waiting for CockroachDB..."
for i in {1..30}; do
    if curl -f http://cockroach:8080/health > /dev/null 2>&1; then
        echo "CockroachDB is ready!"
        break
    fi
    echo "Waiting for CockroachDB... ($i/30)"
    sleep 1
done

echo "Starting azera-core..."
exec ./azera_core
