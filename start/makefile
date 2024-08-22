# Makefile for building the "start" app for different platforms and architectures

# Common build variables
OUTPUT_DIR=..
APP_NAME=start_node

# Ensure a target is provided
ifeq ($(filter linux macos windows,$(MAKECMDGOALS)),)
$(error No target provided. Usage: make <target> ARCH=<arch>)
endif

# Ensure ARCH is set
ifndef ARCH
$(error ARCH is not set. Usage: make <target> ARCH=<arch>)
endif

# Build for macOS
macos:
	GOOS=darwin GOARCH=$(ARCH) go build -o $(OUTPUT_DIR)/$(APP_NAME)

# Build for Linux
linux:
	GOOS=linux GOARCH=$(ARCH) go build -o $(OUTPUT_DIR)/$(APP_NAME)

# Build for Windows
windows:
	GOOS=windows GOARCH=$(ARCH) go build -o $(OUTPUT_DIR)/$(APP_NAME).exe

.PHONY: macos linux windows
