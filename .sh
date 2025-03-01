repomix --no-file-summary --no-security-check \
  --include "src/**,docker-compose.yml" \
  --ignore "src/proxy-manager/go.mod,src/proxy-manager/go.sum" \
  --output "repopack.yml"


docker build -t dublok/proxma:latest -f src/Dockerfile src

docker build -t dublok/proxma:latest -f src/Dockerfile src && docker compose up

docker run --rm \
  --name test-app \
  --network proxma_default \
  --label "proxma.hosts=test.localhost" \
  --label "proxma.port=80" \
  nginx:alpine

docker run --rm \
  --name test-app-ssl \
  --network proxma_default \
  --label "proxma.hosts=test1.localhost,test2.localhost" \
  --label "proxma.port=80" \
  --label "proxma.ssl=true" \
  --label "proxma.ssl.email=test@localhost" \
  nginx:alpine

docker run --rm -d \
  --name test-app-redirect \
  --network proxma_default \
  --label "proxma.hosts=www.test.localhost,test.localhost" \
  --label "proxma.port=80" \
  --label "proxma.redirects=test.localhost>www.test.localhost" \
  nginx:alpine

docker run --rm \
  --name test-app-ssl \
  --network proxma_default \
  --label "proxma.hosts=test1.localhost,test2.localhost" \
  --label "proxma.port=80" \
  --label "proxma.ssl=true" \
  --label "proxma.ssl.email=test@localhost" \
  --label "proxma.ssl.provider=letsencrypt" \
  nginx:alpine

docker run --rm \
  --name test-app-ssl \
  --network proxma_default \
  --label "proxma.hosts=test1.localhost,test2.localhost" \
  --label "proxma.port=80" \
  --label "proxma.ssl=true" \
  --label "proxma.ssl.email=test@localhost" \
  --label "proxma.ssl.provider=development" \
  nginx:alpine