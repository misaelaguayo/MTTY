# MTTY

Simple terminal written in rust

<img width="641" alt="Screenshot 2025-04-20 at 6 44 03 PM" src="https://github.com/user-attachments/assets/dc1edc01-d569-4b1b-b56e-889eea88f54c" />

<img width="641" alt="Screenshot 2025-04-23 at 2 29 51 PM" src="https://github.com/user-attachments/assets/f16bba0f-9a37-45f4-be57-8684d4b201fe" />

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

## Terminfo

MTTY uses a custom terminfo file to allow custom features.
The terminfo file is located in the `xterm-mtty.info` file.
To generate the terminfo file, use the following command:

```bash
tic -x xterm-mtty.info
```

Missing Features
- [ ] Allow terminal resizing
- [ ] OS support besides mac
