{
  # Brokkr for nix, from the release artifacts rather than from source:
  # the derivation fetches the same attested tarball every other channel
  # installs, so the nix store path is provably the release's bytes.
  #
  # The `sha256` values below are placeholders (sixty-four zeros) until a
  # release renders them — `bash packaging/bump-from-sums.sh` reads the
  # release's attested SHA256SUMS and the release workflow opens the pull
  # request that lands them here. `nix build` against a placeholder fails
  # loudly with nix's own hash mismatch, which is the honest failure; the
  # flake still *evaluates*, which is what `nix flake check` proves.
  description = "Brokkr — a delivery engine that drives agent seats through a reviewable phase machine";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      version = "0.6.0"; # brokkr-version

      # One entry per system nix can install: the release matrix's own
      # artifact names, and the digest that belongs to each.
      artifacts = {
        x86_64-linux = {
          file = "brokkr-linux-x86_64.tar.gz";
          sha256 = "0000000000000000000000000000000000000000000000000000000000000000"; # brokkr-linux-x86_64.tar.gz
        };
        aarch64-linux = {
          file = "brokkr-linux-aarch64.tar.gz";
          sha256 = "0000000000000000000000000000000000000000000000000000000000000000"; # brokkr-linux-aarch64.tar.gz
        };
        aarch64-darwin = {
          file = "brokkr-macos-arm64.tar.gz";
          sha256 = "0000000000000000000000000000000000000000000000000000000000000000"; # brokkr-macos-arm64.tar.gz
        };
        x86_64-darwin = {
          file = "brokkr-macos-x86_64.tar.gz";
          sha256 = "0000000000000000000000000000000000000000000000000000000000000000"; # brokkr-macos-x86_64.tar.gz
        };
      };

      systems = builtins.attrNames artifacts;
      forEachSystem = nixpkgs.lib.genAttrs systems;

      brokkrFor = system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          artifact = artifacts.${system};
        in
        pkgs.stdenvNoCC.mkDerivation {
          pname = "brokkr";
          inherit version;

          src = pkgs.fetchurl {
            url = "https://github.com/feedback-loop-ai/brokkr/releases/download/v${version}/${artifact.file}";
            inherit (artifact) sha256;
          };

          # The tarball is one file at its root, not a directory.
          sourceRoot = ".";

          # A binary built on GitHub's runners looks for the host's loader;
          # autoPatchelf points it at the store's. Darwin needs nothing.
          nativeBuildInputs = nixpkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.autoPatchelfHook ];

          dontConfigure = true;
          dontBuild = true;

          # Decision 0019 ruling 9: one binary, and it is `brokkr`.
          installPhase = ''
            runHook preInstall
            install -Dm755 brokkr "$out/bin/brokkr"
            runHook postInstall
          '';

          meta = with nixpkgs.lib; {
            description = "Delivery engine that drives agent seats through a reviewable phase machine";
            homepage = "https://github.com/feedback-loop-ai/brokkr";
            # Decision 0018: the user picks either licence.
            license = with licenses; [ mit asl20 ];
            platforms = systems;
            mainProgram = "brokkr";
          };
        };
    in
    {
      packages = forEachSystem (system: rec {
        brokkr = brokkrFor system;
        default = brokkr;
      });

      # Not how you install Brokkr — how you work on it. Rust's own
      # toolchain plus the two tools the repository's scripts call.
      devShells = forEachSystem (system:
        let pkgs = nixpkgs.legacyPackages.${system}; in
        {
          default = pkgs.mkShell {
            packages = [ pkgs.cargo pkgs.rustc pkgs.rustfmt pkgs.clippy pkgs.jq pkgs.git ];
          };
        });

      formatter = forEachSystem (system: nixpkgs.legacyPackages.${system}.nixpkgs-fmt);
    };
}
