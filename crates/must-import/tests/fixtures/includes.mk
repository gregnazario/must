include common.mk
include $(PLATFORM).mk

build:
	make -f subdir/Makefile
