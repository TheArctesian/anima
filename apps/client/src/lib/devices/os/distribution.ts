import type { IconName } from '$lib/icons/icons';

/** A Linux distribution's logo and its name as the operator reads it. */
export interface Distribution {
  readonly icon: IconName;
  readonly label: string;
}

/**
 * The distributions with a logo in the bundled Nerd Font (font-logos), by the `ID` their `/etc/os-release` gives. An ID
 * not listed here keeps the plain Linux logo.
 */
const DISTRIBUTIONS: Readonly<Record<string, Distribution>> = {
  almalinux: { icon: 'nf-linux-almalinux', label: 'AlmaLinux' },
  alpine: { icon: 'nf-linux-alpine', label: 'Alpine Linux' },
  aosc: { icon: 'nf-linux-aosc', label: 'AOSC OS' },
  arch: { icon: 'nf-linux-archlinux', label: 'Arch Linux' },
  archcraft: { icon: 'nf-linux-archcraft', label: 'Archcraft' },
  archlabs: { icon: 'nf-linux-archlabs', label: 'ArchLabs' },
  arcolinux: { icon: 'nf-linux-arcolinux', label: 'ArcoLinux' },
  artix: { icon: 'nf-linux-artix', label: 'Artix Linux' },
  biglinux: { icon: 'nf-linux-biglinux', label: 'BigLinux' },
  cachyos: { icon: 'nf-linux-cachyos', label: 'CachyOS' },
  centos: { icon: 'nf-linux-centos', label: 'CentOS' },
  'fedora-coreos': { icon: 'nf-linux-coreos', label: 'Fedora CoreOS' },
  crystal: { icon: 'nf-linux-crystal', label: 'Crystal Linux' },
  debian: { icon: 'nf-linux-debian', label: 'Debian' },
  deepin: { icon: 'nf-linux-deepin', label: 'deepin' },
  devuan: { icon: 'nf-linux-devuan', label: 'Devuan' },
  elementary: { icon: 'nf-linux-elementary', label: 'elementary OS' },
  endeavouros: { icon: 'nf-linux-endeavour', label: 'EndeavourOS' },
  fedora: { icon: 'nf-linux-fedora', label: 'Fedora' },
  freebsd: { icon: 'nf-linux-freebsd', label: 'FreeBSD' },
  garuda: { icon: 'nf-linux-garuda', label: 'Garuda Linux' },
  gentoo: { icon: 'nf-linux-gentoo', label: 'Gentoo' },
  guix: { icon: 'nf-linux-gnu_guix', label: 'GNU Guix' },
  hyperbola: { icon: 'nf-linux-hyperbola', label: 'Hyperbola' },
  illumos: { icon: 'nf-linux-illumos', label: 'illumos' },
  kali: { icon: 'nf-linux-kali_linux', label: 'Kali Linux' },
  neon: { icon: 'nf-linux-kde_neon', label: 'KDE neon' },
  'opensuse-leap': { icon: 'nf-linux-leap', label: 'openSUSE Leap' },
  linuxmint: { icon: 'nf-linux-linuxmint', label: 'Linux Mint' },
  locos: { icon: 'nf-linux-locos', label: 'LocOS' },
  lxle: { icon: 'nf-linux-lxle', label: 'LXLE' },
  mageia: { icon: 'nf-linux-mageia', label: 'Mageia' },
  mandriva: { icon: 'nf-linux-mandriva', label: 'Mandriva' },
  manjaro: { icon: 'nf-linux-manjaro', label: 'Manjaro' },
  mx: { icon: 'nf-linux-mxlinux', label: 'MX Linux' },
  nixos: { icon: 'nf-linux-nixos', label: 'NixOS' },
  nobara: { icon: 'nf-linux-nobara', label: 'Nobara' },
  openbsd: { icon: 'nf-linux-openbsd', label: 'OpenBSD' },
  opensuse: { icon: 'nf-linux-opensuse', label: 'openSUSE' },
  openwrt: { icon: 'nf-linux-openwrt', label: 'OpenWrt' },
  parabola: { icon: 'nf-linux-parabola', label: 'Parabola' },
  parrot: { icon: 'nf-linux-parrot', label: 'Parrot OS' },
  pop: { icon: 'nf-linux-pop_os', label: 'Pop!_OS' },
  postmarketos: { icon: 'nf-linux-postmarketos', label: 'postmarketOS' },
  puppy: { icon: 'nf-linux-puppy', label: 'Puppy Linux' },
  qubes: { icon: 'nf-linux-qubesos', label: 'Qubes OS' },
  raspbian: { icon: 'nf-linux-raspberry_pi', label: 'Raspberry Pi OS' },
  rhel: { icon: 'nf-linux-redhat', label: 'Red Hat Enterprise Linux' },
  rocky: { icon: 'nf-linux-rocky_linux', label: 'Rocky Linux' },
  sabayon: { icon: 'nf-linux-sabayon', label: 'Sabayon' },
  slackware: { icon: 'nf-linux-slackware', label: 'Slackware' },
  solus: { icon: 'nf-linux-solus', label: 'Solus' },
  tails: { icon: 'nf-linux-tails', label: 'Tails' },
  trisquel: { icon: 'nf-linux-trisquel', label: 'Trisquel' },
  'opensuse-tumbleweed': { icon: 'nf-linux-tumbleweed', label: 'openSUSE Tumbleweed' },
  ubuntu: { icon: 'nf-linux-ubuntu', label: 'Ubuntu' },
  vanilla: { icon: 'nf-linux-vanilla', label: 'Vanilla OS' },
  void: { icon: 'nf-linux-void', label: 'Void Linux' },
  xerolinux: { icon: 'nf-linux-xerolinux', label: 'XeroLinux' },
  zorin: { icon: 'nf-linux-zorin', label: 'Zorin OS' },
  kubuntu: { icon: 'nf-linux-kubuntu', label: 'Kubuntu' },
};

/** The distribution an os-release `ID` names, or null when it has no logo of its own. */
export function distribution(id: string | null | undefined): Distribution | null {
  if (id === null || id === undefined) return null;
  return Object.hasOwn(DISTRIBUTIONS, id) ? (DISTRIBUTIONS[id] ?? null) : null;
}
