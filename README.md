# MCTools
[![Crate](https://img.shields.io/crates/v/mctools.svg)](https://crates.io/crates/mctools)[![API](https://docs.rs/mctools/badge.svg)](https://docs.rs/mctools)<br>
A Rust library that contains some Minecraft tools.

## Features
- [Skin to totem](./#skin-to-totem)

# Usage
## Skin to totem
List of supported png:
- RGB
- RGBA
- Indexed
- Grayscale
- GrayscaleAlpha

Although `rgb`, `indexed` and `grayscale` supported, i recommend not using it.

```rust
fn main() {
    mctools::skin_to_totem::generate("path_to_skin.png", "where_save_totem.png", true).unwrap();
}
```
