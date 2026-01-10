{ nixpkgs, fenixPkgs }:

let
  llvmPath = nixpkgs.lib.makeBinPath [ nixpkgs.llvm ];
  libx11Path = nixpkgs.lib.makeLibraryPath [
    nixpkgs.libGL
    nixpkgs.libxkbcommon
    nixpkgs.libx11
    nixpkgs.libxcursor
    nixpkgs.xorg.libXi
    nixpkgs.libxrandr
  ];
  waylandPath = nixpkgs.lib.makeLibraryPath [
    nixpkgs.libGL
    nixpkgs.libxkbcommon
    nixpkgs.wayland
    nixpkgs.fontconfig
    nixpkgs.freetype
  ];
in
nixpkgs.mkShell {
  buildInputs = with nixpkgs; [
    cargo-flamegraph
    fenixPkgs.cargo
    fenixPkgs.clippy
    fenixPkgs.rustc
    fenixPkgs.rustfmt
    fenixPkgs.rust-analyzer
    nixpkgs-fmt
    llvm
    cargo-tarpaulin
    cargo-bundle
    fontconfig
  ];

  nativeBuildInputs = with nixpkgs; [
    pkg-config
  ];

  shellHook = ''
    export LD_LIBRARY_PATH=${if nixpkgs.stdenv.isLinux then waylandPath else ""}:$LD_LIBRARY_PATH
    export RUST_SRC_PATH=${fenixPkgs.rust-src}/lib/rustlib/src/rust/library
    export SHELL=nu
  '';
}

