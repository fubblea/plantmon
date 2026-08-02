.PHONY: build-firmware bf flash-firmware f

build-firmware:
	$(MAKE) -C firmware build

# Alias for build-firmware
bf: build-firmware

flash-firmware:
	$(MAKE) -C firmware flash

# Alias for flash-firmware
f: flash-firmware