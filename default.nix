{ nixpkgs, fenix_cargo, fenix_rustc }:

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
in
nixpkgs.mkShell {
  buildInputs = with nixpkgs; [
    fenix_cargo
    fenix_rustc
    nixpkgs-fmt
    llvm
    cargo-tarpaulin
    cargo-bundle
  ];

  shellHook = ''
    export LD_LIBRARY_PATH=${if nixpkgs.stdenv.isLinux then libx11Path else ""}:$LD_LIBRARY_PATH
  '';
}

