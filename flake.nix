{
  description = "repo-to-md development environment";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
  };

  outputs =
    { flake-utils, nixpkgs, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        frontend-test = pkgs.writeShellApplication {
          name = "repo-to-md-frontend-test";
          runtimeInputs = with pkgs; [
            playwright-driver.browsers
            uv
          ];
          text = ''
            export PLAYWRIGHT_BROWSERS_PATH=${pkgs.playwright-driver.browsers}
            export PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS=true
            exec uv run x.py frontend-test
          '';
        };
      in
      {
        apps.frontend-test = {
          type = "app";
          program = "${frontend-test}/bin/repo-to-md-frontend-test";
        };
      }
    );
}
