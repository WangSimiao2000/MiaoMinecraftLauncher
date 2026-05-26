#!/bin/sh
# Run once after cloning to enable pre-commit and pre-push hooks.
git config core.hooksPath .githooks
echo "Git hooks enabled (.githooks/)"
