.PHONY: all fmt lint test bench build clean check clippy install install-completions install-manpage manpage completions readme

all: fmt flint test lint build manpage completions readme

fmt:
	cargo fmt --all

flint:
	@EXIT_CODE=0; \
	cargo fix --broken-code --allow-dirty || EXIT_CODE=$$?; \
	cargo clippy --fix --allow-dirty || EXIT_CODE=$$?; \
	rg -i --no-heading --no-line-number -F '#[expect' | grep -v Makefile || true; \
	rg -i --no-heading --no-line-number -F '#[allow' | grep -v Makefile || true; \
	rg -i --no-heading --no-line-number -F '#![expect' | grep -v Makefile || true; \
	rg -i --no-heading --no-line-number -F '#![allow' | grep -v Makefile | grep -v syncweb-core/benches/ || true; \
	exit $$EXIT_CODE

lint:
	@bash -c 'set -o pipefail; \
	cargo clippy --all-targets --all-features --color always 2>&1 | tee clippy.log || EXIT_CODE=$$?; \
	EXIT_CODE=$${EXIT_CODE:-0}; \
	echo ""; echo "Error Summary:"; \
	cat clippy.log | sed "s/\x1B\[[0-9;]*[a-zA-Z]//g" | grep -E -i "^error(\[[^]]+\])?:" | grep -v "could not compile" | sort | uniq -c | sort -g || true; \
	rm -f clippy.log; \
	exit $$EXIT_CODE'

clippy:
	cargo clippy --all-targets --all-features

test:
	cargo nextest run --show-progress only --no-fail-fast

test0:
	cargo test --all-targets --all-features --quiet
	cargo test --doc

bench:
	cargo bench --all-features

build:
	cargo build --all-targets --all-features

check:
	cargo check --all-targets --all-features

clean:
	cargo clean

install:
	cargo install --path syncweb-cli

completions: build
	mkdir -p completions
	./target/debug/syncweb completions bash > completions/syncweb.bash
	./target/debug/syncweb completions zsh > completions/syncweb.zsh
	./target/debug/syncweb completions fish > completions/syncweb.fish
	./target/debug/syncweb completions elvish > completions/syncweb.elvish
	./target/debug/syncweb completions powershell > completions/syncweb.ps1

manpage: build
	mkdir -p man
	./target/debug/syncweb manpages

readme:
	cd syncweb-core && cargo doc2readme

install-completions:
	install -d $(DESTDIR)/usr/share/bash-completion/completions
	install -m 644 completions/syncweb.bash $(DESTDIR)/usr/share/bash-completion/completions/syncweb
	install -d $(DESTDIR)/usr/share/zsh/site-functions
	install -m 644 completions/syncweb.zsh $(DESTDIR)/usr/share/zsh/site-functions/_syncweb
	install -d $(DESTDIR)/usr/share/fish/vendor_completions.d
	install -m 644 completions/syncweb.fish $(DESTDIR)/usr/share/fish/vendor_completions.d/syncweb.fish

install-manpage:
	install -d $(DESTDIR)/usr/share/man/man1
	install -m 644 man/syncweb.1 $(DESTDIR)/usr/share/man/man1/syncweb.1
	for f in man/syncweb-*.1; do \
		if [ -f "$$f" ]; then \
			install -m 644 "$$f" $(DESTDIR)/usr/share/man/man1/; \
		fi \
	done

release:
	cargo release --execute --no-confirm
