.PHONY: dev build release clean compress install-tools help

# Default target
help:
	@echo "Available commands:"
	@echo "  make dev           - Run development server with hot reload"
	@echo "  make build         - Build for development"
	@echo "  make release       - Build optimized release (includes compression)"
	@echo "  make compress      - Compress static assets (run after release build)"
	@echo "  make clean         - Clean build artifacts"
	@echo "  make install-tools - Install required tools (brotli, zstd, binaryen)"
	@echo "  make check         - Run cargo check"
	@echo "  make test          - Run tests"
	@echo "  make fmt           - Format code"
	@echo "  make clippy        - Run clippy lints"

# Development server with hot reload
dev:
	cargo leptos watch

# Development build
build:
	cargo leptos build

# Optimized release build (compression runs via end-build-command)
release:
	cargo leptos build --release
	bash scripts/compress-assets.sh

# Compress static assets manually
compress:
	bash scripts/compress-assets.sh

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/site/pkg/*.br
	rm -rf target/site/pkg/*.gz
	rm -rf target/site/pkg/*.zst

# Install required tools
install-tools:
	@echo "Installing binaryen (wasm-opt)..."
	sudo pacman -S -y binaryen
	@echo "Installing compression tools..."
	sudo pacman -S -y brotli gzip
	@echo "Done!"

# Cargo check (SSR only - wasm32 dependencies like mio don't support this target)
check:
	cargo check --features ssr

# Run tests
test:
	cargo test --features ssr

# Format code
fmt:
	cargo fmt

# Run clippy (SSR only - wasm32 dependencies like mio don't support this target)
clippy:
	cargo clippy --features ssr -- -D warnings

# Build and run release locally
run-release: release
	./target/release/archischema

# Docker build (if needed)
docker-build:
	docker build -t archischema .

# Show sizes of WASM files
sizes:
	@echo "WASM file sizes:"
	@ls -lh target/site/pkg/*.wasm 2>/dev/null || echo "No WASM files found"
	@echo ""
	@echo "Compressed versions:"
	@ls -lh target/site/pkg/*.wasm.br 2>/dev/null || echo "No .br files"
	@ls -lh target/site/pkg/*.wasm.gz 2>/dev/null || echo "No .gz files"
	@ls -lh target/site/pkg/*.wasm.zst 2>/dev/null || echo "No .zst files"
