CARGO ?= cargo
MANIFEST ?= rust/Cargo.toml
BINARY ?= rust/target/release/gsp370-tray

.PHONY: all build test e2e fmt clippy check

all: build

build:
	$(CARGO) build --release --locked --manifest-path $(MANIFEST)

test:
	$(CARGO) test --locked --manifest-path $(MANIFEST)

e2e: build
	python3 tests/e2e_pty.py $(BINARY)

fmt:
	$(CARGO) fmt --all --manifest-path $(MANIFEST)

clippy:
	$(CARGO) clippy --locked --all-targets --manifest-path $(MANIFEST) -- -D warnings

check: fmt clippy test e2e
