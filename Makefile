.PHONY: css build

css:
	tailwindcss -i input.css -o static/style.css --minify

build: css
	cargo build --release
