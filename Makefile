.PHONY: build test clean fmt lint install all version e2e e2e-install e2e-init e2e-web e2e-cli webbuild webtest web-install

BINARY_NAME=syncweb
BUILD_TAGS=noassets

# Version info from git
VERSION := $(shell git describe --tags --always 2>/dev/null || echo "dev")
GIT_HASH := $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")
GIT_DIRTY := $(shell test -z "$(shell git status --porcelain 2>/dev/null)" || echo "-dirty")
BUILD_TIME := $(shell date -u '+%Y-%m-%d %H:%M:%S UTC')

LDFLAGS := -X 'github.com/chapmanjacobd/syncweb/internal/version.Version=$(VERSION)' \
	-X 'github.com/chapmanjacobd/syncweb/internal/version.GitHash=$(GIT_HASH)' \
	-X 'github.com/chapmanjacobd/syncweb/internal/version.GitDirty=$(GIT_DIRTY)' \
	-X 'github.com/chapmanjacobd/syncweb/internal/version.BuildTime=$(BUILD_TIME)'

all: fmt lint test build

web-install:
	cd web && npm install

webbuild:
	cd web && npm run build

build: webbuild
	go build -tags "$(BUILD_TAGS)" -ldflags "$(LDFLAGS)" -o $(BINARY_NAME) ./cmd/syncweb

version:
	@echo "Version: $(VERSION)"
	@echo "Git Hash: $(GIT_HASH)"
	@echo "Dirty: $(GIT_DIRTY)"

run:
	go run -tags "$(BUILD_TAGS)" -ldflags "$(LDFLAGS)" ./cmd/syncweb $(ARGS)

dev:
	(sleep 2 && xdg-open http://localhost:8889) &
	air -d

fmt:
	gofmt -s -w -e .
	-goimports -w -e .
	-gofumpt -w .
	-gci write .
	go fix -tags "$(BUILD_TAGS)" ./...

lint:
	-staticcheck -tags "$(BUILD_TAGS)" ./...
	go vet -tags "$(BUILD_TAGS)" ./...

test:
	go test -tags "$(BUILD_TAGS)" ./...

cover:
	go test -tags "$(BUILD_TAGS)" -coverprofile=coverage.out ./...
	go tool cover -func=coverage.out | awk '{n=split($$NF,a,"%%"); if (a[1] < 85) print $$0}' | sort -k3 -n

webtest:
	npm test --prefix web

webcover:
	npm run cover --prefix web

ifeq ($(OS),Windows_NT)
	EXE=.exe
else
	EXE=
endif

# Cross-platform build target
release-build: webbuild
	@mkdir -p dist
	GOOS=$(GOOS) GOARCH=$(GOARCH) go build -tags "$(BUILD_TAGS)" -ldflags "$(LDFLAGS)" -o dist/$(BINARY_NAME)-$(GOOS)-$(GOARCH)$(EXE) ./cmd/syncweb

clean:
	rm -f $(BINARY_NAME)
	rm -rf web/dist/*
	rm -rf dist/*

install:
	go install -tags "$(BUILD_TAGS)" ./cmd/syncweb

# E2E Tests
e2e-install:
	npm install --prefix e2e
	npm run install --prefix e2e

e2e: build
	npm run test --prefix e2e

e2e-cli: build
	npm run test --prefix e2e -- --grep 'cli-'

e2e-web: build
	npm run test --prefix e2e -- --grep-invert 'cli-'
