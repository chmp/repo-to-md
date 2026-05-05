{
  description = "repo-to-md development environment";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { flake-utils, nixpkgs, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        playwrightEnv = ''
          export PLAYWRIGHT_BROWSERS_PATH="${pkgs.playwright-driver.browsers}"
          export PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS=true

          for candidate in \
            "$PLAYWRIGHT_BROWSERS_PATH"/chromium-*/chrome-linux/chrome \
            "$PLAYWRIGHT_BROWSERS_PATH"/chromium-*/chrome-linux64/chrome \
            "$PLAYWRIGHT_BROWSERS_PATH"/chromium-*/chrome-headless-shell-linux64/chrome-headless-shell
          do
            if [ -x "$candidate" ]; then
              export PLAYWRIGHT_LAUNCH_OPTIONS_EXECUTABLE_PATH="$candidate"
              break
            fi
          done
        '';

        frontend-test = pkgs.writeShellApplication {
          name = "repo-to-md-frontend-test";
          runtimeInputs = with pkgs; [
            playwright-driver.browsers
            uv
          ];
          text = ''
            ${playwrightEnv}
            exec uv run x.py frontend-test
          '';
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            playwright-driver.browsers
            uv
          ];

          shellHook = playwrightEnv;
        };

        apps.frontend-test = {
          type = "app";
          program = "${frontend-test}/bin/repo-to-md-frontend-test";
        };
      }
    );
}
