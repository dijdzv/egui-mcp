{ pkgs, lib, config, inputs, ... }:

let
  nixgl = inputs.nixgl.packages.${pkgs.system}.nixGLIntel;
in
{
  # https://devenv.sh/basics/
  env.GREET = "egui-mcp dev environment";

  # https://devenv.sh/packages/
  packages = with pkgs; [
    git
    jq
    just

    # Linux GUI dependencies for egui/eframe
    pkg-config
    libxkbcommon
    wayland

    # OpenGL wrapper for WSLg
    nixgl
  ];

  # Environment variables for WSLg
  env = {
    # WSLg Wayland support
    WAYLAND_DISPLAY = "wayland-0";
    XDG_RUNTIME_DIR = "/mnt/wslg/runtime-dir";
    # Suppress MESA/EGL warnings in WSLg
    MESA_DEBUG = "silent";
    LIBGL_DEBUG = "quiet";
    EGL_LOG_LEVEL = "fatal";
    # Library path for dynamic loading of wayland/xkbcommon
    LD_LIBRARY_PATH = "${pkgs.wayland}/lib:${pkgs.libxkbcommon}/lib";
  };

  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    channel = "stable";
    components = [ "rustc" "cargo" "clippy" "rustfmt" "rust-analyzer" ];
  };

  # https://devenv.sh/git-hooks/
  git-hooks.hooks = {
    rustfmt.enable = true;
    clippy.enable = true;
  };

  # https://devenv.sh/scripts/
  scripts.hello.exec = ''
    echo "Welcome to egui-mcp development!"
    echo "Rust version: $(rustc --version)"
    echo "Cargo version: $(cargo --version)"
  '';

  enterShell = ''
    hello
  '';
}
