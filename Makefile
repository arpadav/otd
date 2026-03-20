.DEFAULT_GOAL := run
.PHONY: css build docker

css:
	tailwindcss -i static/input.css -o static/style.css --minify

build: css
	cargo build --release

run: build
	cargo run --release

docker:
	bash scripts/build.sh
