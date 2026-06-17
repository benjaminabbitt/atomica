# CHROME AND CODE — dev tasks.
.PHONY: test check run web serve clean

WASM := target/wasm32-unknown-unknown/release/atomica.wasm

# Pure-logic tests (no system libs / display needed).
test:
	cargo test -p atomica-sim -p atomica-run

# Type-check the whole workspace for the web target.
check:
	cargo check -p atomica-sim -p atomica-run
	cargo check -p atomica-game --target wasm32-unknown-unknown

# Native run (needs a display + X11/ALSA dev libs).
run:
	cargo run -p atomica-game

# Build the wasm artifact and stage it next to web/index.html.
web:
	cargo build -p atomica-game --target wasm32-unknown-unknown --release
	cp $(WASM) web/atomica.wasm

# Build, then serve the page locally.
serve: web
	@echo "Open http://localhost:8000"
	python3 -m http.server 8000 --directory web

clean:
	cargo clean
	rm -f web/atomica.wasm
