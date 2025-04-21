# 2025 Spring ICG Homework 1

A [GitHub Pages](https://edwar4rd.github.io/2025S_ICG_HW1/) of this repo is available online.

## Usage

- WASD for moving around, E for up, Q for down.

## Building

[Install rust](https://www.rust-lang.org/en-US/learn/get-started) and cargo before building.

Building for web:
```bash
cargo install trunk --locked
trunk build --release
```


Building for native:
```bash
cargo run --release
```

## FAQ

- The page can be stuck loading sometimes due to caching errors.\
  Try force refreshing the page with `Shift` + `F5` or `Ctrl` + `Shift` + `R`.
