flakeInputs:
flakeInputs.nixpkgs-lib.lib.genAttrs [
  "x86_64-linux"
  "aarch64-linux"
] (system: flakeInputs.self.formatterModule.${system}.config.build.wrapper)
