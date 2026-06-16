# atomica — CHROME AND CODE

Async auto-battler / roguelike-deckbuilder. Cyberpunk, neo-Japan cyber-samurai.
Design lives in [`CHROME-AND-CODE.md`](CHROME-AND-CODE.md).

## Stack

Rust → WebAssembly, via **[macroquad](https://github.com/not-fl3/macroquad)** (2D
rendering, best-in-class wasm story) + **[egui](https://github.com/emilk/egui)**
(data-heavy UI) through `egui-macroquad`. No `wasm-bindgen` required.

## Architecture

The battle resolution is decoupled from the engine so it can run headless and
reproduce identically (needed for async / replayable auto-resolution):

```
crates/
  sim/    atomica-sim   pure Rust, no engine deps, deterministic (seeded RNG)
  game/   atomica-game  thin macroquad + egui front-end that renders sim state
web/      index.html    macroquad wasm loader
```

The front-end never owns rules — it reads `Battle` state and draws it. Swap the
renderer, or run the sim on a server, without touching game logic.

> Status: scaffold. `sim` models the *locked shapes* (stat line, layered defense,
> penetration tiers, armor-matrix axis, initiative-ordered ticks) with placeholder
> values and a minimal "attack nearest / step toward" resolution loop. The status
> pool, both contagions, netrunning, IFF/spoof, and Heat are marked extension
> points, not guessed numbers.

## Develop

```sh
make test     # run the deterministic sim tests (no display needed)
make check    # type-check the workspace for the wasm target
make web      # build target/.../atomica.wasm and stage it into web/
make serve    # build + serve at http://localhost:8000
make run      # native window (needs a display + X11/ALSA dev libs)
```

First build downloads crates from crates.io.

### Notes
- The web build (`make web` / `make check`) is the canonical target and needs no
  system libraries. Native `make run` needs desktop dev libs (X11, ALSA).
- egui clipboard/IME in the browser may need extra JS glue; the base loader covers
  rendering and pointer/keyboard input.
