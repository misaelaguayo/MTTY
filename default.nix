{ nixpkgs ? import <nixpkgs> {} }:

nixpkgs.rustPlatform.buildRustPackage {
  pname = "MTTY";
  version = "v0.1";
  src = ./.;
  cargoHash = "sha256-VX/7Gn3dO5OIj1Ktm78dWOKwKW0+MS/aFJMYMFDYj+I=";
  buildInputs = 
    [ nixpkgs.SDL2 nixpkgs.SDL2_ttf nixpkgs.openssl nixpkgs.pkg-config nixpkgs.libiconv nixpkgs.darwin.apple_sdk.frameworks.AppKit ];
}
