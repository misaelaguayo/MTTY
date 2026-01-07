# MTTY

Simple terminal written in rust

![Image 1](https://github.com/user-attachments/assets/dc1edc01-d569-4b1b-b56e-889eea88f54c)
 

![Image 2](https://github.com/user-attachments/assets/f16bba0f-9a37-45f4-be57-8684d4b201fe)
 
# Testing
To run the tests, use the following command:
```bash
cargo test
```

## Coverage
To run the coverage report, use the following command:
```bash
cargo tarpaulin --out Html
```

## Flamegraph
To generate a flamegraph, for release builds, use the following command:
```bash
cargo-flamegraph flamegraph
```

For debug builds, use the following command:
```bash
cargo flamegraph --dev
```

## Terminfo

MTTY uses a custom terminfo file to allow custom features.
The terminfo file is located in the `xterm-mtty.info` file.
To generate the terminfo file, use the following command:

```bash
tic -x xterm-mtty.info
```
