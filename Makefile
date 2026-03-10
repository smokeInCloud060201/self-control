setup:
	$(MAKE) -C deploy build-all-images
	$(MAKE) -C deploy local-up


clean:
	$(MAKE) -C deploy local-down