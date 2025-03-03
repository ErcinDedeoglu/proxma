repomix --no-file-summary --no-security-check \
  --include "src/**,Cargo.toml" \
  --output "repopack.yml"

# nginx
repomix --no-file-summary --no-security-check \
  --include "src/main.rs,src/nginx/**" \
  --output "repopack.yml"

# nginx
repomix --no-file-summary --no-security-check \
  --include "src/main.rs,src/certbot.rs" \
  --output "repopack.yml"


docker build -t dublok/proxma:latest -f src/Dockerfile .
docker buildx build --platform linux/amd64,linux/arm64 -t dublok/proxma:latest -f src/Dockerfile .
docker run -it --rm --name proxma -v /var/run/docker.sock:/var/run/docker.sock dublok/proxma:latest

docker run -it --rm --rm --name -v /var/run/docker.sock:/var/run/docker.sock  -p 80:80 -p 443:443 dublok/proxma:latest

# BUILD
docker build -t dublok/proxma:latest -f src/Dockerfile .

# NO-CACHE BUILD
docker build -t dublok/proxma:latest -f src/Dockerfile . --no-cache

# Run with shell to debug
docker run -it --rm -v /var/run/docker.sock:/var/run/docker.sock --entrypoint /bin/sh dublok/proxma:latest
ls -la /usr/local/bin/

###################
### DEBUG:
docker run -it --rm --rm --name proxma2 -v /var/run/docker.sock:/var/run/docker.sock  -p 80:80 -p 443:443 dublok/proxma:latest
# Connect to the running container
docker exec -it proxma2 /bin/sh
# Check if the proxy rules file exists
ls -la /etc/nginx/conf.d/
# View the contents of the proxy rules file
cat /etc/nginx/conf.d/proxma-proxy-rules.conf

##################


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