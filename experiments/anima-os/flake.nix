{
  description = "Anima OS disposable F1/UEFI-F2 feasibility prototype";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
  outputs = { self, nixpkgs }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
  in {
    packages.${system}.default = import ./image.nix {
      inherit pkgs;
      systemA = self.nixosConfigurations.prototype.config.system.build.toplevel;
      systemB = self.nixosConfigurations.candidate.config.system.build.toplevel;
    };
    nixosConfigurations.candidate = nixpkgs.lib.nixosSystem {
      inherit system;
      modules = [ ./machine.nix { anima.revision = "B"; } ];
    };
    nixosConfigurations.prototype = nixpkgs.lib.nixosSystem {
      inherit system;
      modules = [ ./machine.nix ];
    };
  };
}
