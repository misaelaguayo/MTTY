let
  nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-24.11";
  pkgs = import nixpkgs { config = {}; overlays = []; };
  bin = with pkgs; [ libllvm ];
in

pkgs.mkShellNoCC {
  nativeBuildInputs = with pkgs.buildPackages; [ clang ];
  packages = with pkgs; [
    grcov
    lcov
    cargo
  ];
  shellHook = ''
    export CARGO_INCREMENTAL=0
    export RUSTFLAGS="-Cinstrument-coverage"
    export LLVM_PROFILE_FILE="cargo-test-%p-%m.profraw"
    export BIN_PATH="${pkgs.lib.makeBinPath bin}"
  '';

  # generating coverage reports
  # cargo test
  # grcov . --binary-path ./target/debug/deps/ -s . -t html --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage/html --llvm-path=$BIN_PATH
}
