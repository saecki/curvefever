.PHONY: build
build:
	cd curvefever_remote && trunk build --release --filehash false
	cargo build --release --bin curvefever

run:
	cargo run --release --bin curvefever
