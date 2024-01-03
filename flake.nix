{
  inputs = {
      nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable-small";
      fenix = {
        url = "github:nix-community/fenix";
        inputs.nixpkgs.follows = "nixpkgs";
      };
  };

  outputs = { self, fenix, nixpkgs }:
  let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      config.allowUnfree = true;
    };
  in
  {
    devShell.${system} = pkgs.mkShell {
      buildInputs = with pkgs; [
        pre-commit

        pkg-config
        libxkbcommon
        udev
        libseat
        mesa
        libinput
        pixman

        (fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-nEfxHpwByxtZZV9CrlC8xaaYXIEIxHRKLR1QGY3Nx7I=";
        })
      ];

      shellHook = ''
           export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath [
             # For aider
             pkgs.stdenv.cc.cc

             pkgs.wayland
             pkgs.libglvnd
           ]}
      '';
    };
  };
}
