{
  description = "throwparty/tardy";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/release-25.11";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    throwparty = {
      url = "git+ssh://git@github.com/throwparty/nix";
      inputs.flake-utils.follows = "flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      flake-utils,
      nixpkgs,
      rust-overlay,
      throwparty,
      self,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            (import rust-overlay)
          ];
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default;
        inherit (pkgs.lib) getExe getExe';
      in
      {
        devShells.default =
          let
            inherit (throwparty.devShells.${system}) commonTools githubActions nodejs_24;
            inherit (throwparty.lib) mergeShells mkToolVersions;
            inherit (pkgs)
              cargo-auditable
              cargo-binstall
              cargo-deny
              libxkbcommon
              mdformat
              nixfmt
              pkg-config
              wayland
              ;
            toolVersions = mkToolVersions {
              inherit pkgs;
              name = "default";
              commands = ''
                ${getExe mdformat} --version
                ${getExe nixfmt} --version
                ${getExe pkg-config} --version
                ${getExe' rustToolchain "cargo"} --version
                ${getExe' rustToolchain "rustc"} --version
                printf "cargo-binstall %s\n" "$(${getExe cargo-binstall} --version 2>/dev/null || true)"
              '';
            };
            tools = pkgs.mkShell {
              buildInputs = [
                wayland
                libxkbcommon
              ];
              nativeBuildInputs = [
                cargo-auditable
                cargo-binstall
                cargo-deny
                mdformat
                nixfmt
                pkg-config
                rustToolchain
              ];
              shellHook = ''
                cat ${toolVersions}
                export LD_LIBRARY_PATH="${wayland}/lib:${libxkbcommon}/lib:$LD_LIBRARY_PATH"
              '';
            };
          in
          mergeShells [
            commonTools
            githubActions
            nodejs_24
            tools
          ];

        packages.default =
          (pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          }).buildRustPackage
            rec {
              pname = "tardy";
              version = "0.1.0";

              src = ../.;

              nativeBuildInputs = [
                pkgs.makeWrapper
                pkgs.pkg-config
                pkgs.wayland
                pkgs.libxkbcommon
              ];
              buildInputs = [
                pkgs.wayland
                pkgs.libxkbcommon
              ];
              cargoHash = "sha256-wZYF1YNKSBIUQR+hZOvT6a6mbJk0fdqpxM/CyrxL10I=";

              postInstall = ''
                wrapProgram $out/bin/tardy \
                  --prefix LD_LIBRARY_PATH : ${pkgs.wayland}/lib:${pkgs.libxkbcommon}/lib
              '';

              meta = {
                description = "Never be late to meetings again";
                license = pkgs.lib.licenses.mit;
                mainProgram = "tardy";
              };
            };

        checks.default = self.packages.${system}.default;
      }
    )
    // {
      overlays.default = final: prev: {
        tardy = self.packages.${final.system}.default;
      };
    };
}
