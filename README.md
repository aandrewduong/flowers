<h1 align="center">flowers</h1>

<p align="center"><em>A garden that grows in your terminal.</em></p>

<p align="center">
  <img src="assets/demo.gif" alt="flowers in interactive mode: a garden sprouts and blooms, a new one grows, species are switched off until only tulips and lavender remain, more flowers are added, and every species comes back for one last garden">
</p>

`flowers` is a small Rust command-line toy that grows gardens of colorful flowers right in
your terminal. Daisies, sunflowers, roses, dahlias, cosmos, poppies, tulips and lavender are
each drawn from scratch every time, in their own palettes and with petal-by-petal variation,
so no two gardens come out the same. Found one you love? Every garden has a seed, and
handing that seed back regrows it exactly.

Run it for a splash of color between tasks, watch a garden sprout and bloom, or open the
interactive mode and browse garden after garden from the keyboard.

## Highlights

- **Eight species**, each with several color palettes and natural variation in every petal.
- **Smooth, colorful art**: anti-aliased shapes in 24-bit color, two square pixels per character cell.
- **Watch it grow**: stems rise, leaves unfurl and flowers open, one after another.
- **Interactive mode**: grow new gardens, step back through ones you've seen, and switch species, flower count and grass on the fly.
- **Reproducible**: every run prints its seed; the same seed, size and options always grow the same garden.
- **Pipe-friendly**: the art goes to stdout and the seed to stderr, so redirecting output keeps the picture clean.

## Install

You need a Rust toolchain, version 1.85 or newer ([rustup.rs](https://rustup.rs)). From a
checkout of this repository:

```sh
cargo install --path .
```

That puts `flowers` on your `PATH`. To try it without installing, use `cargo run --release --`
in place of `flowers` in the examples below.

## Quick start

```sh
flowers                  # a garden that fits your terminal
flowers -n 1             # one big flower
flowers --bloom          # watch the garden sprout and bloom
flowers -k rose,tulip    # only roses and tulips
flowers -s 7             # regrow a garden from its seed
flowers -i               # browse gardens interactively
```

For a fresh flower in every new terminal, add `flowers -n 1 -H 12` to your shell's startup file.

## Interactive mode

`flowers -i` opens a full-screen garden that fills the terminal and regrows to fit when the
window is resized. The bar along the bottom shows which species are growing, the seed, and the
keys. When you quit, it prints the seed of the garden that was on screen.

| Key | Action |
| --- | --- |
| `space` / `enter` / `n` | Grow a new garden |
| `←` / `→` | Step back and forward through gardens you've seen |
| `b` | Replay the bloom |
| `1`–`8` | Toggle a species on or off (at least one always stays on) |
| `0` | Grow every species again |
| `+` / `-` (or `↑` / `↓`) | More or fewer flowers |
| `g` | Grass on or off |
| `q` / `esc` / `ctrl-c` | Quit |

`--kind`, `--count`, `--seed`, `--no-grass` and `--colors` set where the session starts.

## The species

<p align="center">
  <img src="assets/species.png" alt="One of each species: a lilac daisy, a golden sunflower, a cream rose, a peach dahlia, a magenta cosmos, a scarlet poppy, a pink tulip and a spike of lavender" width="760">
</p>

| Species | Look |
| --- | --- |
| `daisy` | White, blush or lilac rays around a golden eye |
| `sunflower` | Big golden heads with spiralled seed centres |
| `rose` | Layered petals furled around a swirling heart |
| `dahlia` | Ring upon ring of pointed petals |
| `cosmos` | Eight notched petals on a slender stem |
| `poppy` | Silky cups with a dark blotched eye |
| `tulip` | Side-on cups, sometimes flamed at the rim |
| `lavender` | Purple spikes of tiny florets |

## Options

| Flag | Meaning |
| --- | --- |
| `-n, --count <N>` | Number of flowers (default: as many as fit the width) |
| `-k, --kind <KINDS>` | Only grow these species, comma-separated or repeated |
| `-s, --seed <SEED>` | Seed for a reproducible garden |
| `-W, --width <COLS>` | Width in columns (default: terminal width) |
| `-H, --height <ROWS>` | Height in rows (default: 24, or less to fit) |
| `-b, --bloom` | Animate the garden growing |
| `-i, --interactive` | Browse gardens full-screen with the keyboard |
| `--no-grass` | Leave out the grass |
| `--colors ansi256` | For terminals without 24-bit color |

## Terminal support

`flowers` draws in 24-bit color, which most modern terminals support. If the colors come out
wrong or banded, as in older versions of macOS Terminal, pass `--colors ansi256` to use the
standard 256-color palette instead.

## How it works

Nothing is stored as pixel art. Each flower is built from shapes described by math: petals
are outlines repeated around a center, with each ring of petals casting a soft shadow on the
ring beneath it. Sunflower and daisy centers pack their seeds in the golden-angle spiral that
real sunflowers use, and stems are gently curved Bézier paths.

The garden is painted onto a canvas sampled nine times per pixel, which smooths the edges,
and then printed with the half-block character `▀`. Its foreground color is the upper pixel
and its background the lower, so every character cell shows two square pixels. A small
deterministic random number generator, fed by the seed, makes every choice along the way.

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```
