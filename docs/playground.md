# Vinglish Playground

The Vinglish website includes an interactive browser preview of the Skyline
Runner example. The preview mirrors the game rules so visitors can try the
controls immediately; the program itself is written in Vinglish and targets the
native UI runtime rather than JavaScript.

## Native example

[`examples/skyline_runner.ving`](../examples/skyline_runner.ving) demonstrates
the `std.ui` drawing API, a fixed-step game loop, collision-friendly state, and
the `is_space_pressed` input helper.

```bash
vng build examples/skyline_runner.ving --output skyline-runner
./skyline-runner
```

Press `Space` to jump. The native UI bridge is backed by Rust's `minifb`
library, exported through `#[vinglish_export]` and surfaced as ordinary
Vinglish foreign functions.

## Browser preview

The embedded version is intentionally a JavaScript visual preview, not a
Vinglish interpreter. Vinglish currently compiles to native C, so the browser
cannot execute `.ving` source directly. Keeping the preview alongside the full
source lets visitors play the design while inspecting the program that maps to
the native build.
