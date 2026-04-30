.PHONY: all clean install

all: app

clean:
	rm -f app

install: app
	cp app $(PREFIX)/bin/
