#!/bin/bash
# Test script that outputs markers in a specific order to verify stream ordering
# Usage: ordered-output.sh <prefix>

PREFIX="${1:-test}"

echo "STDOUT-START:$PREFIX"
echo "STDOUT-1:$PREFIX" >&2
echo "STDOUT-2:$PREFIX"
echo "STDERR-1:$PREFIX" >&2
echo "STDOUT-3:$PREFIX"
echo "STDERR-2:$PREFIX" >&2
echo "STDOUT-END:$PREFIX"
echo "STDERR-END:$PREFIX" >&2
