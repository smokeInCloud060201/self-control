DOCKER_BIN=docker
DOCKER_COMPOSE_BIN=docker compose
PROJECT_NAME=self-control

build-proxy-image:
	$(DOCKER_BIN) build --build-arg APP_NAME=proxy -t $(PROJECT_NAME)-proxy:latest -f dockerfile/proxy.Dockerfile ../proxy

build-agent-image:
	$(DOCKER_BIN) build --build-arg APP_NAME=agent -t $(PROJECT_NAME)-agent:latest -f dockerfile/agent.Dockerfile ../agent

build-web-image:
	$(DOCKER_BIN) build -t $(PROJECT_NAME)-web:latest -f dockerfile/web.Dockerfile ../web

build-all-images:
	$(MAKE) build-proxy-image
	$(MAKE) build-agent-image
	$(MAKE) build-web-image

local-up:
	$(DOCKER_COMPOSE_BIN) -f docker-compose.local.yml -p $(PROJECT_NAME) up -d

local-down:
	$(DOCKER_COMPOSE_BIN) -f docker-compose.local.yml -p $(PROJECT_NAME) down
