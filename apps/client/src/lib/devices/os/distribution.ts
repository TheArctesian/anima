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
  almalinux: { icon: 'simple-icons:almalinux', label: 'AlmaLinux' },
  alpine: { icon: 'simple-icons:alpinelinux', label: 'Alpine Linux' },
  aosc: { icon: 'simple-icons:linux', label: 'AOSC OS' },
  arch: { icon: 'simple-icons:archlinux', label: 'Arch Linux' },
  archcraft: { icon: 'simple-icons:linux', label: 'Archcraft' },
  archlabs: { icon: 'simple-icons:linux', label: 'ArchLabs' },
  arcolinux: { icon: 'simple-icons:linux', label: 'ArcoLinux' },
  artix: { icon: 'simple-icons:artixlinux', label: 'Artix Linux' },
  biglinux: { icon: 'simple-icons:linux', label: 'BigLinux' },
  cachyos: { icon: 'simple-icons:cachyos', label: 'CachyOS' },
  centos: { icon: 'simple-icons:centos', label: 'CentOS' },
  'fedora-coreos': { icon: 'simple-icons:linux', label: 'Fedora CoreOS' },
  crystal: { icon: 'simple-icons:crystal', label: 'Crystal Linux' },
  debian: { icon: 'simple-icons:debian', label: 'Debian' },
  deepin: { icon: 'simple-icons:deepin', label: 'deepin' },
  devuan: { icon: 'simple-icons:devuan', label: 'Devuan' },
  elementary: { icon: 'simple-icons:elementary', label: 'elementary OS' },
  endeavouros: { icon: 'simple-icons:endeavouros', label: 'EndeavourOS' },
  fedora: { icon: 'simple-icons:fedora', label: 'Fedora' },
  freebsd: { icon: 'simple-icons:freebsd', label: 'FreeBSD' },
  garuda: { icon: 'simple-icons:garudalinux', label: 'Garuda Linux' },
  gentoo: { icon: 'simple-icons:gentoo', label: 'Gentoo' },
  guix: { icon: 'simple-icons:linux', label: 'GNU Guix' },
  hyperbola: { icon: 'simple-icons:linux', label: 'Hyperbola' },
  illumos: { icon: 'simple-icons:linux', label: 'illumos' },
  kali: { icon: 'simple-icons:kalilinux', label: 'Kali Linux' },
  neon: { icon: 'simple-icons:kdeneon', label: 'KDE neon' },
  'opensuse-leap': { icon: 'simple-icons:opensuse', label: 'openSUSE Leap' },
  linuxmint: { icon: 'simple-icons:linuxmint', label: 'Linux Mint' },
  locos: { icon: 'simple-icons:linux', label: 'LocOS' },
  lxle: { icon: 'simple-icons:linux', label: 'LXLE' },
  mageia: { icon: 'simple-icons:linux', label: 'Mageia' },
  mandriva: { icon: 'simple-icons:linux', label: 'Mandriva' },
  manjaro: { icon: 'simple-icons:manjaro', label: 'Manjaro' },
  mx: { icon: 'simple-icons:mxlinux', label: 'MX Linux' },
  nixos: { icon: 'simple-icons:nixos', label: 'NixOS' },
  nobara: { icon: 'simple-icons:linux', label: 'Nobara' },
  openbsd: { icon: 'simple-icons:openbsd', label: 'OpenBSD' },
  opensuse: { icon: 'simple-icons:opensuse', label: 'openSUSE' },
  openwrt: { icon: 'simple-icons:openwrt', label: 'OpenWrt' },
  parabola: { icon: 'simple-icons:linux', label: 'Parabola' },
  parrot: { icon: 'simple-icons:parrotsecurity', label: 'Parrot OS' },
  pop: { icon: 'simple-icons:popos', label: 'Pop!_OS' },
  postmarketos: { icon: 'simple-icons:linux', label: 'postmarketOS' },
  puppy: { icon: 'simple-icons:linux', label: 'Puppy Linux' },
  qubes: { icon: 'simple-icons:qubesos', label: 'Qubes OS' },
  raspbian: { icon: 'simple-icons:raspberrypi', label: 'Raspberry Pi OS' },
  rhel: { icon: 'simple-icons:redhat', label: 'Red Hat Enterprise Linux' },
  rocky: { icon: 'simple-icons:rockylinux', label: 'Rocky Linux' },
  sabayon: { icon: 'simple-icons:linux', label: 'Sabayon' },
  slackware: { icon: 'simple-icons:slackware', label: 'Slackware' },
  solus: { icon: 'simple-icons:solus', label: 'Solus' },
  tails: { icon: 'simple-icons:tails', label: 'Tails' },
  trisquel: { icon: 'simple-icons:linux', label: 'Trisquel' },
  'opensuse-tumbleweed': { icon: 'simple-icons:opensuse', label: 'openSUSE Tumbleweed' },
  ubuntu: { icon: 'simple-icons:ubuntu', label: 'Ubuntu' },
  vanilla: { icon: 'simple-icons:linux', label: 'Vanilla OS' },
  void: { icon: 'simple-icons:voidlinux', label: 'Void Linux' },
  xerolinux: { icon: 'simple-icons:linux', label: 'XeroLinux' },
  zorin: { icon: 'simple-icons:zorin', label: 'Zorin OS' },
  kubuntu: { icon: 'simple-icons:kubuntu', label: 'Kubuntu' },
};

/** The distribution an os-release `ID` names, or null when it has no logo of its own. */
export function distribution(id: string | null | undefined): Distribution | null {
  if (id === null || id === undefined) return null;
  return Object.hasOwn(DISTRIBUTIONS, id) ? (DISTRIBUTIONS[id] ?? null) : null;
}
