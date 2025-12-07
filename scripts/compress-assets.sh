#!/bin/bash
# Pre-compress static assets for faster serving
# Run this after `cargo leptos build --release`

set -e

SITE_DIR="target/site"
PKG_DIR="$SITE_DIR/pkg"

echo "🗜️  Pre-compressing static assets..."

# Check if required tools are installed
check_tool() {
    if ! command -v "$1" &> /dev/null; then
        echo "⚠️  $1 not found. Install with: $2"
        return 1
    fi
    return 0
}

HAS_BROTLI=false
HAS_GZIP=false

check_tool "brotli" "sudo apt install brotli" && HAS_BROTLI=true
check_tool "gzip" "sudo apt install gzip" && HAS_GZIP=true

if [ ! -d "$PKG_DIR" ]; then
    echo "❌ Package directory not found: $PKG_DIR"
    echo "   Run 'cargo leptos build --release' first"
    exit 1
fi

# Compress function
compress_file() {
    local file="$1"
    local size_before=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file")

    # Brotli compression (best for web)
    if [ "$HAS_BROTLI" = true ]; then
        brotli -f -k -q 11 "$file"
        local br_size=$(stat -f%z "$file.br" 2>/dev/null || stat -c%s "$file.br")
        echo "  📦 Brotli: $(basename "$file") ($size_before → $br_size bytes)"
    fi

    # Gzip compression (wide support)
    if [ "$HAS_GZIP" = true ]; then
        gzip -f -k -9 "$file"
        local gz_size=$(stat -f%z "$file.gz" 2>/dev/null || stat -c%s "$file.gz")
        echo "  📦 Gzip:   $(basename "$file") ($size_before → $gz_size bytes)"
    fi

}

# Find and compress WASM, JS, and CSS files
echo ""
echo "🔍 Processing files in $PKG_DIR..."
echo ""

for ext in wasm js css; do
    find "$PKG_DIR" -name "*.$ext" -type f | while read -r file; do
        # Skip already compressed files
        if [[ "$file" == *.br ]] || [[ "$file" == *.gz ]]; then
            continue
        fi
        compress_file "$file"
    done
done

# Also compress HTML files in site root
echo ""
echo "🔍 Processing HTML files in $SITE_DIR..."
echo ""

find "$SITE_DIR" -maxdepth 1 -name "*.html" -type f | while read -r file; do
    compress_file "$file"
done

echo ""
echo "✅ Compression complete!"
echo ""
echo "📊 Summary:"
find "$PKG_DIR" -name "*.wasm" -type f | while read -r file; do
    original=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file")
    echo "   Original WASM: $(basename "$file") - $((original / 1024)) KB"

    if [ -f "$file.br" ]; then
        br_size=$(stat -f%z "$file.br" 2>/dev/null || stat -c%s "$file.br")
        ratio=$((100 - (br_size * 100 / original)))
        echo "   Brotli:        $((br_size / 1024)) KB (${ratio}% smaller)"
    fi

    if [ -f "$file.gz" ]; then
        gz_size=$(stat -f%z "$file.gz" 2>/dev/null || stat -c%s "$file.gz")
        ratio=$((100 - (gz_size * 100 / original)))
        echo "   Gzip:          $((gz_size / 1024)) KB (${ratio}% smaller)"
    fi
done
