let
  nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-24.05";
  pkgs = import nixpkgs { config = {}; overlays = []; };
  libraries = with pkgs; [ SDL2 SDL2_ttf ];
in

pkgs.mkShellNoCC {
  shellHook = ''
    export LIBRARY_PATH="${pkgs.lib.makeLibraryPath libraries}"
  '';
}
