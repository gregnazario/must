all: build test
	@echo done

build:
	gcc -o app main.c

test:
	./run_tests.sh
