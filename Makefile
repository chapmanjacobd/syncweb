.PHONY: build test clean fmt lint install all

BINARY_NAME=syncweb
BUILD_TAGS=noassets

all: fmt lint test build

build:
	go build -tags "$(BUILD_TAGS)" -o $(BINARY_NAME) ./cmd/syncweb

test:
	go test -tags "$(BUILD_TAGS)" ./...

fmt:
	gofmt -s -w -e .
	go fix -tags "$(BUILD_TAGS)" ./...

lint:
	-staticcheck -tags "$(BUILD_TAGS)" ./...
	go vet -tags "$(BUILD_TAGS)" ./...

clean:
	rm -f $(BINARY_NAME)

install:
	go install ./cmd/syncweb
