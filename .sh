repomix --no-file-summary --no-security-check \
  --include "src/**,docker-compose.yml" \
  --ignore "src/proxy-manager/go.mod,src/proxy-manager/go.sum" \
  --output "repopack.yml"


docker build -t dublok/proxma:latest -f src/Dockerfile src

docker build -t dublok/proxma:latest -f src/Dockerfile src && docker compose up