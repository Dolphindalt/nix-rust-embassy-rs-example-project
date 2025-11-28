flakeInputs:
let
  inherit (flakeInputs.nixpkgs-lib)
    lib
    ;

  inherit (lib)
    composeManyExtensions
    ;

  inherit (flakeInputs)
    rust-overlay
    ;

  topLevelOverlay = topFinal: topPrev: {
    christmas = topPrev.callPackage ./packages/christmas.nix { };
  };

  rustOverlay = rust-overlay.overlays.default;
in
{
  default = composeManyExtensions ([
    topLevelOverlay
    rustOverlay
  ]);
}
