flakeInputs:
flakeInputs.nixpkgs-lib.lib.attrsets.mapAttrs (
  system: pkgs:
  (flakeInputs.treefmt-nix.lib.evalModule pkgs (
    { ... }:
    {
      config = {
        enableDefaultExcludes = true;
        projectRootFile = "flake.nix";
        programs = {
          nixfmt.enable = true;
          rustfmt.enable = true;
        };
        settings.global.excludes = [
          "*.gitignore"
          ".git-blame-ignore-revs"
        ];
      };
    }
  ))
) flakeInputs.self.legacyPackages
