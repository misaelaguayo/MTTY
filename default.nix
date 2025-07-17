{ nixpkgs }:

nixpkgs.mkShell {
  buildInputs = with nixpkgs; [
    cargo
    rustc
    rustup
    nixpkgs-fmt
    glib
    gtk3
    libglvnd
    libxkbcommon
    xorg.libX11
  ];

  nativeBuildInputs = with nixpkgs; [
    pkg-config
  ];

  shellHook = ''
    export RUSTFLAGS="-Cinstrument-coverage";
    export RUSTFLAGS="-Ccodegen-units=1 -Copt-level=0 -Coverflow-checks=off -Cpanic=abort";
    export RUSTDOCFLAGS="-Cpanic=abort";
    export LLVM_PROFILE_FILE="coverage.profraw";
    export CARGO_INCREMENTAL=0;
    export ICED_BACKEND="tiny-skia";
  '';

  LD_LIBRARY_PATH = "${nixpkgs.libxkbcommon}/lib";
}

