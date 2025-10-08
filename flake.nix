{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    {
      overlay = final: prev: {
        hk = prev.callPackage ./default.nix { };
      };
    } // flake-utils.lib.eachDefaultSystem(system:
      let
        pkgs = import nixpkgs { inherit system; };
        hk = pkgs.callPackage ./default.nix { };
      in
        {
          packages = {
            inherit hk;
            default = hk;
          };

          devShells.default = pkgs.mkShell {
            name = "hk-develop";

            inputsFrom = [ hk ];

            nativeBuildInputs = with pkgs; [
              just
              clippy
              rustfmt
              shellcheck
              shfmt
              nodejs
              cargo-release
              cargo-insta
            ];
          };
        }
    );
}
