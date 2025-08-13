{ nixpkgs }:

nixpkgs.mkShell {
  buildInputs = with nixpkgs; [
    cargo
    rustc
    nixpkgs-fmt
  ];

  shellHook = ''
    export RUST_LOG='MTTY=INFO';
  '';
}

