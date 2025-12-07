.PHONY: dev build release clean compress install-tools help check test fmt clippy run-release docker-build sizes

# Directories
SITE_DIR := target/site
PKG_DIR := $(SITE_DIR)/pkg

# Default target
help:
	@echo "Available commands:"
	@echo "  make dev           - Run development server with hot reload"
	@echo "  make build         - Build for development"
	@echo "  make release       - Build optimized release (includes compression)"
	@echo "  make compress      - Compress static assets (run after release build)"
	@echo "  make clean         - Clean build artifacts"
	@echo "  make install-tools - Install required tools (brotli, binaryen)"
	@echo "  make check         - Run cargo check"
	@echo "  make test          - Run tests"
	@echo "  make fmt           - Format code"
	@echo "  make clippy        - Run clippy lints"
	@echo "  make sizes         - Show sizes of WASM files"
	@echo "  make git-check     - Run pre-commit checks"

# Development server with hot reload
dev:
	cargo leptos watch

# Development build
build:
	cargo leptos build

# Optimized release build (includes compression)
release: build-release compress
	@echo "✅ Release build complete!"

build-release:
	cargo leptos build --release

# Compress static assets
compress:
	@echo "🗜️  Pre-compressing static assets..."
	@if [ ! -d "$(PKG_DIR)" ]; then \
		echo "❌ Package directory not found: $(PKG_DIR)"; \
		echo "   Run 'make build-release' first"; \
		exit 1; \
	fi
	@# Compress WASM, JS, CSS files with Brotli
	@if command -v brotli >/dev/null 2>&1; then \
		echo "📦 Compressing with Brotli..."; \
		find $(PKG_DIR) -type f \( -name "*.wasm" -o -name "*.js" -o -name "*.css" \) | while read f; do \
			brotli -f -k -q 11 "$$f"; \
			echo "   ✓ $$(basename $$f).br"; \
		done; \
		find $(SITE_DIR) -maxdepth 1 -name "*.html" -type f | while read f; do \
			brotli -f -k -q 11 "$$f"; \
			echo "   ✓ $$(basename $$f).br"; \
		done; \
	else \
		echo "⚠️  brotli not found. Install with: sudo pacman -S brotli"; \
	fi
	@# Compress with Gzip
	@if command -v gzip >/dev/null 2>&1; then \
		echo "📦 Compressing with Gzip..."; \
		find $(PKG_DIR) -type f \( -name "*.wasm" -o -name "*.js" -o -name "*.css" \) | while read f; do \
			gzip -f -k -9 "$$f"; \
			echo "   ✓ $$(basename $$f).gz"; \
		done; \
		find $(SITE_DIR) -maxdepth 1 -name "*.html" -type f | while read f; do \
			gzip -f -k -9 "$$f"; \
			echo "   ✓ $$(basename $$f).gz"; \
		done; \
	else \
		echo "⚠️  gzip not found. Install with: sudo pacman -S gzip"; \
	fi
	@echo "✅ Compression complete!"
	@$(MAKE) --no-print-directory sizes

# Clean build artifacts
clean:
	cargo clean
	rm -rf $(PKG_DIR)/*.br
	rm -rf $(PKG_DIR)/*.gz

# Install required tools
install-tools:
	@echo "Installing binaryen (wasm-opt)..."
	sudo pacman -S --noconfirm binaryen
	@echo "Installing compression tools..."
	sudo pacman -S --noconfirm brotli gzip
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
	@echo ""
	@echo "📊 File sizes:"
	@for f in $(PKG_DIR)/*.wasm; do \
		if [ -f "$$f" ]; then \
			orig_size=$$(stat -c%s "$$f" 2>/dev/null || stat -f%z "$$f"); \
			echo "   Original: $$(basename $$f) - $$((orig_size / 1024)) KB"; \
			if [ -f "$$f.br" ]; then \
				br_size=$$(stat -c%s "$$f.br" 2>/dev/null || stat -f%z "$$f.br"); \
				ratio=$$((100 - (br_size * 100 / orig_size))); \
				echo "   Brotli:   $$((br_size / 1024)) KB ($${ratio}% smaller)"; \
			fi; \
			if [ -f "$$f.gz" ]; then \
				gz_size=$$(stat -c%s "$$f.gz" 2>/dev/null || stat -f%z "$$f.gz"); \
				ratio=$$((100 - (gz_size * 100 / orig_size))); \
				echo "   Gzip:     $$((gz_size / 1024)) KB ($${ratio}% smaller)"; \
			fi; \
		fi; \
	done || echo "   No WASM files found"

# Pre-commit checks
git-check: check fmt clippy test
	@echo "✅ All checks passed!"
