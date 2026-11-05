# Tainah Neural Resampler

Optional UTAU command-line resampler that runs the Tainah WORLD engine followed by the NSF vocoder. Kamafeu Studio's built-in Tainah WORLD renderer remains part of the base app; this helper is installed separately when the neural mode is selected.

The Tainah native library and ONNX model are provided beside the executable or installed through Kamafeu's engine setup. They are not embedded in the Studio package.

## Build

```sh
cargo build --release --locked
```

## License

MIT. Kamafeu core is used as a Git dependency; retain the upstream notices.
