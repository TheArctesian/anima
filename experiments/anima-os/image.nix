{ pkgs, systemA, systemB }:
let
  closure = pkgs.closureInfo { rootPaths = [ systemA systemB ]; };
  vm = pkgs.vmTools.override { rootModules = [ "virtio_pci" "virtio_blk" "virtiofs" "btrfs" "vfat" "nls_cp437" "nls_iso8859_1" ]; };
in vm.runInLinuxVM (pkgs.runCommand "anima-prototype-disk" {
  nativeBuildInputs = with pkgs; [ btrfs-progs dosfstools util-linux parted nix ];
  memSize = 4096;
  preVM = ''
    diskImage=$PWD/anima.raw
    truncate -s 24G "$diskImage"
  '';
  postVM = ''
    mkdir -p $out
    ${pkgs.qemu-utils}/bin/qemu-img convert -f raw -O qcow2 "$diskImage" $out/anima.qcow2
    cp ${closure}/store-paths $out/closure-paths
    echo ${systemA} > $out/system-a
    echo ${systemB} > $out/system-b
    sha256sum $out/anima.qcow2 > $out/SHA256SUMS
  '';
} ''
  parted -s /dev/vda mklabel gpt mkpart ESP fat32 1MiB 1025MiB set 1 esp on mkpart ANIMA btrfs 1025MiB 100%
  mkfs.vfat -n ESP /dev/vda1
  mkfs.btrfs -f -L ANIMA /dev/vda2
  mkdir -p /mnt
  mount /dev/vda2 /mnt
  for name in root nix var persist; do btrfs subvolume create /mnt/@$name; done
  umount /mnt
  mount -o subvol=@root /dev/vda2 /mnt
  mkdir -p /mnt/{nix,var,persist,boot,etc,root}
  for name in nix var persist; do mount -o subvol=@$name /dev/vda2 /mnt/$name; done
  mount /dev/vda1 /mnt/boot
  mkdir -p /mnt/nix/store /mnt/nix/var/nix/profiles
  while read -r source; do cp -a "$source" /mnt/nix/store/; done < ${closure}/store-paths
  nix-store --store /mnt --load-db < ${closure}/registration
  ln -s ${systemA} /mnt/nix/var/nix/profiles/system-1-link
  ln -s system-1-link /mnt/nix/var/nix/profiles/system
  ln -s ${systemA}/init /mnt/init
  mkdir -p /mnt/boot/EFI/BOOT /mnt/boot/loader/entries /mnt/boot/EFI/anima
  cp ${pkgs.systemd}/lib/systemd/boot/efi/systemd-bootx64.efi /mnt/boot/EFI/BOOT/BOOTX64.EFI
  for revision in a b; do
    if test "$revision" = a; then system=${systemA}; else system=${systemB}; fi
    cp "$system/kernel" /mnt/boot/EFI/anima/$revision-kernel
    cp "$system/initrd" /mnt/boot/EFI/anima/$revision-initrd
    cat > /mnt/boot/loader/entries/anima-$revision.conf <<ENTRY
  title Anima prototype $revision
  linux /EFI/anima/$revision-kernel
  initrd /EFI/anima/$revision-initrd
  options init=$system/init console=ttyS0,115200 panic=10
  ENTRY
  done
  printf 'default anima-a.conf\ntimeout 1\n' > /mnt/boot/loader/loader.conf
  sync
  umount -R /mnt
'')
