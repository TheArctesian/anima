{
  description = "Anima OS recovery prototype and experimental UEFI installer";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
  outputs = { self, nixpkgs }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
    daemonPackage = import ./daemon-package.nix { inherit pkgs; };
  in {
    packages.${system} = {
      default = import ./image.nix {
        inherit pkgs;
        systemA = self.nixosConfigurations.prototype.config.system.build.toplevel;
        systemB = self.nixosConfigurations.candidate.config.system.build.toplevel;
        paramsA = self.nixosConfigurations.prototype.config.boot.kernelParams;
        paramsB = self.nixosConfigurations.candidate.config.boot.kernelParams;
      };
      iso = self.nixosConfigurations.installer.config.system.build.isoImage;
    };
    nixosConfigurations.appliance = nixpkgs.lib.nixosSystem {
      inherit system;
      specialArgs = { inherit daemonPackage; };
      modules = [ ./appliance.nix ];
    };
    nixosConfigurations.installer = nixpkgs.lib.nixosSystem {
      inherit system;
      specialArgs.installedSystem = self.nixosConfigurations.appliance.config.system.build.toplevel;
      modules = [ ./iso.nix ];
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
