PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
DESTDIR ?=
CARGO ?= cargo
NAME = wmailor
BIN = target/release/$(NAME)
INSTALL ?= install
INSTALL_PROGRAM ?= $(INSTALL) -m 755
INSTALL_DIR ?= $(INSTALL) -d

.PHONY: all build install uninstall clean test

all: build

build: $(BIN)

$(BIN):
	env -u DESTDIR $(CARGO) build --release

install: $(BIN)
	$(INSTALL_DIR) $(DESTDIR)$(BINDIR)
	$(INSTALL_PROGRAM) $(BIN) $(DESTDIR)$(BINDIR)/$(NAME)

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/$(NAME)

clean:
	$(CARGO) clean

test:
	$(CARGO) test
