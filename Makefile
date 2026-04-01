setup:
	$(MAKE) -C deploy build-all-images
	$(MAKE) -C deploy local-up


clean:
	$(MAKE) -C deploy local-down

agent-install:
	$(MAKE) -C deploy macos-service-install

agent-uninstall:
	$(MAKE) -C deploy macos-service-uninstall

agent-start:
	$(MAKE) -C deploy agent-start

agent-stop:
	$(MAKE) -C deploy agent-stop

agent-restart:
	$(MAKE) -C deploy agent-restart

agent-logs:
	$(MAKE) -C deploy agent-logs