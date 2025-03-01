repomix --no-file-summary --no-security-check \
  --include "src/**" \
  --output "repopack.yml"


docker build -t dublok/proxma:latest -f src/Dockerfile src

