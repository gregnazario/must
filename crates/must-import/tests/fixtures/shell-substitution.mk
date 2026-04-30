GIT_HASH := $(shell git rev-parse --short HEAD)

version:
	@echo $(GIT_HASH)
