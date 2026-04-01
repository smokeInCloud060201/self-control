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
	sudo rm -rf /Applications/SelfControlAgent.app
	sudo mkdir -p /Applications/SelfControlAgent.app/Contents/MacOS
	sudo cp ../agent/target/release/agent /Applications/SelfControlAgent.app/Contents/MacOS/self-control-agent
	sudo cp ../agent/Info.plist /Applications/SelfControlAgent.app/Contents/Info.plist
	sudo codesign --force --deep --sign - /Applications/SelfControlAgent.app
	sudo cp ../agent/com.selfcontrol.agent.plist /Library/LaunchAgents/
	sudo chown root:wheel /Library/LaunchAgents/com.selfcontrol.agent.plist
	sudo launchctl bootout gui/$$(id -u) /Library/LaunchAgents/com.selfcontrol.agent.plist 2>/dev/null || true
	sudo launchctl bootstrap gui/$$(id -u) /Library/LaunchAgents/com.selfcontrol.agent.plist || true

macos-service-uninstall:
	sudo launchctl bootout gui/$$(id -u) /Library/LaunchAgents/com.selfcontrol.agent.plist || true
	sudo rm /Library/LaunchAgents/com.selfcontrol.agent.plist
	sudo rm -rf /Applications/SelfControlAgent.app

agent-start:
	sudo launchctl kickstart -k gui/$$(id -u)/com.selfcontrol.agent

agent-stop:
	sudo launchctl kill SIGTERM gui/$$(id -u)/com.selfcontrol.agent

agent-restart:
	sudo launchctl kickstart -k gui/$$(id -u)/com.selfcontrol.agent

agent-logs:
	tail -f /tmp/self-control-agent.log
