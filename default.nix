{ nixpkgs, fenix_cargo, fenix_rustc }:

let
  llvmPath = nixpkgs.lib.makeBinPath [ nixpkgs.llvm ];
in
nixpkgs.mkShell {
  buildInputs = with nixpkgs; [
    fenix_cargo
    fenix_rustc
    nixpkgs-fmt
    llvm
    cargo-tarpaulin
  ];

  shellHook = ''
    export RUST_LOG='MTTY=INFO';
  '';
}

