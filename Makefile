.PHONY: build test clean fmt lint install all

BINARY_NAME=syncweb
BUILD_TAGS=noassets

all: fmt lint test build

build:
	go build -tags "$(BUILD_TAGS)" -o $(BINARY_NAME) ./cmd/syncweb

dev:
	(sleep 2 && xdg-open http://localhost:8889) &
	air -d

fmt:
	gofmt -s -w -e .
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

clean:
	rm -f $(BINARY_NAME)

install:
	go install ./cmd/syncweb
