#!/bin/bash

# Setup script for git hooks
# This enables pre-commit test hooks for the project

echo "🔧 Configuring git hooks..."

# Configure git to use the .githooks directory
git config core.hooksPath .githooks

# Make the pre-commit hook executable
chmod +x .githooks/pre-commit

echo "✅ Git hooks configured successfully!"
echo ""
echo "The following hooks are now active:"
echo "  - pre-commit: Runs all tests before allowing commits"
echo ""
echo "Note: To bypass hooks (not recommended), use: git commit --no-verify"
