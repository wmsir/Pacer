#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

SERVER_IP="${SERVER_IP:-}"
SERVER_PORT="${SERVER_PORT:-22}"
SERVER_USER="${SERVER_USER:-root}"
SERVER_PASS="${SERVER_PASS:-}"
DB_URL="${DB_URL:-}"
DB_USER="${DB_USER:-}"
DB_PASS="${DB_PASS:-}"

if [ -z "$SERVER_IP" ] || [ -z "$SERVER_PASS" ] || [ -z "$DB_URL" ] || [ -z "$DB_USER" ] || [ -z "$DB_PASS" ]; then
  echo "Error: Missing required environment variables."
  echo "Please set SERVER_IP, SERVER_PASS, DB_URL, DB_USER, and DB_PASS before running this script."
  exit 1
fi

cd "${ROOT_DIR}"

echo "[1/4] Building backend..."
mvn clean package -DskipTests -f backend/pom.xml

echo "[2/4] Building frontend..."
(cd frontend && npm ci && npm run build)

echo "[3/4] Uploading artifacts to ${SERVER_USER}@${SERVER_IP}:${SERVER_PORT}..."
sshpass -p "${SERVER_PASS}" ssh -o StrictHostKeyChecking=no -p "${SERVER_PORT}" "${SERVER_USER}@${SERVER_IP}" "mkdir -p /opt/wmdb/backend /opt/wmdb/frontend"

sshpass -p "${SERVER_PASS}" scp -o StrictHostKeyChecking=no -P "${SERVER_PORT}" backend/target/wmdb-backend-1.0.0-SNAPSHOT.jar "${SERVER_USER}@${SERVER_IP}:/opt/wmdb/backend/"

tar -czf frontend-dist.tar.gz -C frontend/dist .
sshpass -p "${SERVER_PASS}" scp -o StrictHostKeyChecking=no -P "${SERVER_PORT}" frontend-dist.tar.gz "${SERVER_USER}@${SERVER_IP}:/opt/wmdb/frontend/"
rm -f frontend-dist.tar.gz

echo "[4/4] Restarting service on remote server..."
sshpass -p "${SERVER_PASS}" ssh -o StrictHostKeyChecking=no -p "${SERVER_PORT}" "${SERVER_USER}@${SERVER_IP}" << EOF
  pkill -f wmdb-backend-1.0.0-SNAPSHOT.jar || true

  cd /opt/wmdb/frontend
  tar -xzf frontend-dist.tar.gz
  rm -f frontend-dist.tar.gz

  cd /opt/wmdb/backend
  nohup java -Dspring.datasource.dynamic.datasource.master.url="${DB_URL}" \
       -Dspring.datasource.dynamic.datasource.master.username="${DB_USER}" \
       -Dspring.datasource.dynamic.datasource.master.password="${DB_PASS}" \
       -jar wmdb-backend-1.0.0-SNAPSHOT.jar > backend.log 2>&1 &
EOF

echo "Deployment complete."
