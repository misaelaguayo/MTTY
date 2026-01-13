{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, fenix, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        fenixPkgs = fenix.packages.${system}.complete;
        isLinux = pkgs.stdenv.isLinux;

        commonLibs = with pkgs; [
          libGL
          libxkbcommon
          fontconfig
          freetype
          vulkan-loader
          libglvnd
          mesa
        ];

        waylandLibs = with pkgs; [
          wayland
        ];

        x11Libs = with pkgs; [
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
        ];

        mkShell = { graphicsLibs }: pkgs.mkShell {
          buildInputs = with pkgs; [
            fenixPkgs.cargo
            fenixPkgs.clippy
            fenixPkgs.rustc
            fenixPkgs.rustfmt
            fenixPkgs.rust-analyzer

            nixpkgs-fmt
            llvm

            cargo-flamegraph
            cargo-tarpaulin
            cargo-bundle
          ] ++ pkgs.lib.optionals isLinux (commonLibs ++ graphicsLibs);

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          shellHook = ''
            export RUST_SRC_PATH=${fenixPkgs.rust-src}/lib/rustlib/src/rust/library
          '' + pkgs.lib.optionalString isLinux ''
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath (commonLibs ++ graphicsLibs)}:$LD_LIBRARY_PATH
            export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/gfxstream_vk_icd.x86_64.json:/usr/share/vulkan/icd.d/lvp_icd.x86_64.json
          '';
        };
      in
      {
        devShells = {
          default = mkShell { graphicsLibs = waylandLibs; };
          wayland = mkShell { graphicsLibs = waylandLibs; };
          x11 = mkShell { graphicsLibs = x11Libs; };
        };
      });
}
