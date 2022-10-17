{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            rust-overlay.overlays.default
          ];
        };

        rustVersion = "1.63.0";
      
      in {
        devShell =
          pkgs.mkShell {
            buildInputs = [
              (pkgs.rust-bin.stable.${rustVersion}.default.override {
                extensions = [
                  "cargo"
                  "clippy"
                  "rustc"
                  "rust-src"
                  "rustfmt"
                ];
              })

              pkgs.cmake
              pkgs.pkg-config
              pkgs.fontconfig
              
              pkgs.vulkan-validation-layers
              pkgs.vulkan-tools
              
              pkgs.renderdoc
            ];

            VK_INSTANCE_LAYERS = "VK_LAYER_KHRONOS_validation:VK_EXT_swapchain_colorspace";
            VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
            LD_LIBRARY_PATH = "${pkgs.xorg.libX11}/lib:${pkgs.xorg.libXcursor}/lib:${pkgs.xorg.libXrandr}/lib:${pkgs.xorg.libXi}/lib:${pkgs.vulkan-loader}/lib";
          };
      }
    );
}
