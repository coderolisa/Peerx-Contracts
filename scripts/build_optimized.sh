#!/bin/bash

# Build script for optimizing Soroban contract size

set -e

echo "Building Soroban contracts with size optimizations..."

# Function to get file size
get_size() {
    if [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
        # Windows
        stat -c%s "$1" 2>/dev/null || echo "0"
    else
        # Unix-like
        stat -f%z "$1" 2>/dev/null || stat -c%s "$1" 2>/dev/null || echo "0"
    fi
}

# Build the contracts using release profile
soroban contract build

echo "Contracts built successfully."

# Check sizes
echo "Contract sizes:"
for wasm_file in target/wasm32-unknown-unknown/release/*.wasm; do
    if [ -f "$wasm_file" ]; then
        size=$(get_size "$wasm_file")
        echo "$wasm_file: $size bytes"
    fi
done

# Optional: If wasm-opt is available, further optimize
if command -v wasm-opt &> /dev/null; then
    echo "Running wasm-opt for additional size reduction..."
    for wasm_file in target/wasm32-unknown-unknown/release/*.wasm; do
        if [ -f "$wasm_file" ]; then
            original_size=$(get_size "$wasm_file")
            echo "Optimizing $wasm_file (original: $original_size bytes)..."
            wasm-opt -Oz "$wasm_file" -o "${wasm_file}.tmp" && mv "${wasm_file}.tmp" "$wasm_file"
            new_size=$(get_size "$wasm_file")
            reduction=$((original_size - new_size))
            percent=$((reduction * 100 / original_size))
            echo "$wasm_file: $new_size bytes (reduced by $reduction bytes, $percent%)"
        fi
    done
    echo "WASM optimization completed."
else
    echo "wasm-opt not found. Install binaryen for additional optimizations."
fi

echo "Build complete. Check target/wasm32-unknown-unknown/release/ for optimized contracts."