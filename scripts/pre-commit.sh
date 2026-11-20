#!/usr/bin/env bash
# Pre-commit hook: run naga validation on all WGSL shader files
# Usage: placed in .git/hooks/pre-commit and made executable
#
# This ensures shaders compile via Naga before any commit.
# Install: cp scripts/pre-commit.sh .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit

set -euo pipefail

# Find all WGSL files in the repo
WGSL_FILES=$(find . -name '*.wgsl' -not -path './target/*' -not -path './.git/*')

if [ -z "$WGSL_FILES" ]; then
    echo "✅ No WGSL files to check"
    exit 0
fi

echo "🔍 Validating WGSL shaders with Naga..."

ERRORS=0
for file in $WGSL_FILES; do
    if naga "$file" > /dev/null 2>&1; then
        echo "  ✅ $file"
    else
        echo "  ❌ $file"
        naga "$file" 2>&1 || true
        ERRORS=$((ERRORS + 1))
    fi
done

if [ $ERRORS -gt 0 ]; then
    echo ""
    echo "❌ $ERRORS WGSL file(s) failed Naga validation"
    echo "Fix the errors above, then try again."
    exit 1
fi

echo ""
echo "✅ All WGSL files passed Naga validation"
exit 0
