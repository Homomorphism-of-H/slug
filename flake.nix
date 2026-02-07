{
  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs = { self, fenix, flake-utils, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system: 
      let
          toolchain = fenix.packages.${system}.minimal.toolchain;
          pkgs = nixpkgs.legacyPackages.${system};
          cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);

          runtimeDeps = with pkgs; [
            libxkbcommon
            qtcreator
            wayland
          ];
          buildDeps = with pkgs; [
            wayland
            pkg-config
            rustPlatform.bindgenHook
          ];
          devDeps = with pkgs; [  ];

          rustPackage = features:
            (pkgs.makeRustPlatform {
              cargo = toolchain;
              rustc = toolchain;
            }).buildRustPackage {
              pname = cargoToml.package.name;
              version = cargoToml.workspace.package.version;

              src = ./.;
              cargoLock.lockFile = ./Cargo.lock;

              buildFeatures = features;
              buildInputs = runtimeDeps;
              nativeBuildInputs = buildDeps;

            };
            
          mkDevShell =
            pkgs.mkShell {
              shellHook = ''
                export RUST_SRC_PATH=${pkgs.rustPlatform.rustLibSrc}
              '';
              buildInputs = runtimeDeps;
              nativeBuildInputs = buildDeps ++ devDeps ++ [fenix.packages.${system}.default.toolchain];
              LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath runtimeDeps}";
            };

      in {
        packages.default = (rustPackage "");
        devShells.default = (mkDevShell);
      }
  );
}
