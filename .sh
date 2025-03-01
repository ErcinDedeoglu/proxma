repomix --no-file-summary --no-security-check \
  --include "src/**" \
  --ignore "src/proxy-manager/go.mod,src/proxy-manager/go.sum" \
  --output "repopack.yml"


docker build -t dublok/proxma:latest -f src/Dockerfile src

