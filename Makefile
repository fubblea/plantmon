.PHONY: check-firmware cf build-firmware bf flash-firmware f

check-firmware:
	$(MAKE) -C firmware check

# Alias for check-firmware
cf: check-firmware

build-firmware:
	$(MAKE) -C firmware build

# Alias for build-firmware
bf: build-firmware

flash-firmware:
	$(MAKE) -C firmware flash

# Alias for flash-firmware
f: flash-firmware