.PHONY: all fmt lint test build clean check clippy install install-completions install-manpage manpage completions

all: fmt lint test build manpage completions

fmt:
	cargo fmt --all

lint:
	cargo fix --broken-code --allow-dirty
	cargo clippy --fix --allow-dirty
	cargo clippy --all-targets --all-features -- -D warnings

clippy:
	cargo clippy --all-targets --all-features

test:
	cargo test --all-targets --all-features --quiet
	cargo test --doc

build:
	cargo build --all-targets --all-features

check:
	cargo check --all-targets --all-features

clean:
	cargo clean

install:
	cargo install --path .

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
