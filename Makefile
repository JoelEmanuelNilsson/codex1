BIN := codex1
PROJECT_ROOT := $(shell pwd)
INSTALL_DIR ?= $(HOME)/.local/bin

.PHONY: help build test fmt clippy install-local verify-installed verify-contract clean

help:
	@echo "Codex1 — make targets:"
	@echo "  build              cargo build --release"
	@echo "  test               cargo test"
	@echo "  fmt                cargo fmt --check"
	@echo "  clippy             cargo clippy -- -D warnings"
	@echo "  install-local      build + copy target/release/$(BIN) to $(INSTALL_DIR)/"
	@echo "  verify-installed   command -v $(BIN) && $(BIN) --help && $(BIN) --json doctor"
	@echo "  verify-contract    run the full contract suite (fmt + clippy + test + install + smoke)"
	@echo "  clean              cargo clean"

build:
	cargo build --release

test:
	cargo test

fmt:
	cargo fmt --check

clippy:
	cargo clippy -- -D warnings

install-local: build
	@mkdir -p $(INSTALL_DIR)
	cp target/release/$(BIN) $(INSTALL_DIR)/$(BIN)
	@echo "Installed $(BIN) to $(INSTALL_DIR)/$(BIN)"

verify-installed:
	@command -v $(BIN) > /dev/null || { echo "$(BIN) not on PATH"; exit 1; }
	@echo "$(BIN) is installed at: $$(command -v $(BIN))"
	$(BIN) --help > /dev/null
	$(BIN) doctor

verify-contract: fmt clippy test install-local verify-installed
	@echo "Contract suite passed."

clean:
	cargo clean
