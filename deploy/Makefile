DOCKER_BIN=docker
DOCKER_COMPOSE_BIN=docker compose
PROJECT_NAME=self-control

build-proxy-image:
	$(DOCKER_BIN) build --build-arg APP_NAME=proxy -t $(PROJECT_NAME)-proxy:latest -f dockerfile/proxy.Dockerfile ../proxy

build-web-image:
	$(DOCKER_BIN) build -t $(PROJECT_NAME)-web:latest -f dockerfile/web.Dockerfile ../web

build-all-images:
	$(MAKE) build-proxy-image
	$(MAKE) build-web-image

local-up:
	$(DOCKER_COMPOSE_BIN) -f docker-compose.local.yml -p $(PROJECT_NAME) up -d

local-down:
	$(DOCKER_COMPOSE_BIN) -f docker-compose.local.yml -p $(PROJECT_NAME) down

macos-service-install:
	cd ../agent && cargo build --release --features macos_service
	sudo cp ../agent/target/release/agent /usr/local/bin/self-control-agent
	sudo cp ../agent/com.selfcontrol.agent.plist /Library/LaunchDaemons/
	sudo launchctl load /Library/LaunchDaemons/com.selfcontrol.agent.plist

macos-service-uninstall:
	sudo launchctl unload /Library/LaunchDaemons/com.selfcontrol.agent.plist
	sudo rm /Library/LaunchDaemons/com.selfcontrol.agent.plist
	sudo rm /usr/local/bin/self-control-agent
