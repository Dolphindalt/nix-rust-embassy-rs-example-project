flakeInputs:
flakeInputs.nixpkgs-lib.lib.genAttrs
  [
    "x86_64-linux"
    "aarch64-linux"
  ]
  (
    system:
    let
      inherit (flakeInputs.self.legacyPackages.${system})
        christmas
        ;
    in
    {
      inherit christmas;
      default = christmas;
    }
  )
