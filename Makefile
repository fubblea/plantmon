.PHONY: build-firmware bf

build-firmware:
	$(MAKE) -C firmware build

# Alias for build-firmware
bf: build-firmware