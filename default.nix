{ nixpkgs }:

nixpkgs.mkShell {
  buildInputs = with nixpkgs; [
    cargo
    rustc
    nixpkgs-fmt
  ];
}

