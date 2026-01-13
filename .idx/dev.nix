{ pkgs, ... }: {
  channel = "stable-23.11";
  packages = [
    pkgs.rustup
    pkgs.nodejs_20
    pkgs.gcc
    pkgs.gnumake
    pkgs.pkg-config
    pkgs.gtk3
    pkgs.webkitgtk
    pkgs.librsvg
    pkgs.libsoup
    pkgs.libappindicator-gtk3
  ];
  idx = {
    extensions = [
      "rust-lang.rust-analyzer"
      "tauri-apps.tauri-vscode"
      "bradlc.vscode-tailwindcss"
    ];
    workspace = {
      onCreate = {
        setup = "rustup default stable";
      };
    };
  };
}
