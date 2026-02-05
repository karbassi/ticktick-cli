.PHONY: release check

VERSION := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')

check:
	cargo fmt -- --check
	cargo clippy -- -D warnings
	cargo test

release: check
	@if [ -z "$(VERSION)" ]; then echo "Could not read version from Cargo.toml"; exit 1; fi
	@if git tag | grep -q "^v$(VERSION)$$"; then echo "Tag v$(VERSION) already exists"; exit 1; fi
	@echo "Releasing v$(VERSION)..."
	git add -A
	git commit -m "release: v$(VERSION)" --allow-empty
	git tag "v$(VERSION)"
	git push origin main "v$(VERSION)"
	@echo "Release v$(VERSION) pushed. GitHub Actions will handle the rest."
