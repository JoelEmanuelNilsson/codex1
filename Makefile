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
	@mkdir -p "$(INSTALL_DIR)"
	install -m 0755 "target/release/$(BIN)" "$(INSTALL_DIR)/$(BIN)"
	@echo "Installed $(BIN) to $(INSTALL_DIR)/$(BIN)"

verify-installed:
	@installed="$(INSTALL_DIR)/$(BIN)"; \
	  test -x "$$installed" || { echo "$$installed is not executable"; exit 1; }; \
	  echo "$(BIN) is installed at: $$installed"; \
	  tmp="$$(mktemp -d /tmp/codex1-verify.XXXXXX)"; \
	  PATH="$(INSTALL_DIR):$$PATH"; export PATH; \
	  cd "$$tmp" && \
	  resolved="$$(command -v $(BIN))" && \
	  test "$$resolved" = "$$installed" && \
	  $(BIN) --help > /dev/null && \
	  $(BIN) doctor > /dev/null && \
	  $(BIN) init --mission verify-smoke > /dev/null && \
	  test -f PLANS/verify-smoke/STATE.json && \
	  $(BIN) status --mission verify-smoke > /dev/null; \
	  rc=$$?; rm -rf "$$tmp"; exit $$rc

verify-contract: fmt clippy test install-local verify-installed
	@echo "Contract suite passed."

clean:
	cargo clean
