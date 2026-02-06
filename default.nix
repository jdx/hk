{ pkgs, lib, stdenv, fetchFromGitHub, rustPlatform, coreutils, bash, direnv, openssl, git }:
let
  cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
in rustPlatform.buildRustPackage {
  pname = "hk";
  inherit (cargoToml.package) version;

  src = lib.cleanSource ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = with pkgs; [ pkg-config ];
  buildInputs = with pkgs; [
    libgit2
    openssl
  ];

  checkPhase = ''
    RUST_BACKTRACE=full cargo test --all-features -- \
      --skip cli::init::detector::tests::test_detect_builtins_with_cargo_toml \
      --skip cli::init::detector::tests::test_detect_builtins_with_package_json \
      --skip cli::init::detector::tests::test_detect_eslint_with_contains \
      --skip cli::init::detector::tests::test_detect_shell_scripts \
      --skip cli::util::python_check_ast::tests::test_invalid_python \
      --skip settings::tests::test_settings_builder_fluent_api \
      --skip settings::tests::test_settings_from_config \
      --skip settings::tests::test_settings_snapshot_caching
  '';

  meta = with lib; {
    description = "git hooks and project lints";
    homepage = "https://github.com/jdx/hk";
    license = licenses.mit;
    mainProgram = "hk";
  };
}
