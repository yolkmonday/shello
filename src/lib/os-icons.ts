/**
 * Maps detected OS distro identifiers to Iconify icon names and brand colors.
 * Uses `mdi` icon set for all OS icons.
 */

export interface OsInfo {
  os_type: string;
  distro: string;
}

interface OsEntry {
  icon: string;
  color: string;
}

const distroMap: Record<string, OsEntry> = {
  // Linux distros
  ubuntu:    { icon: "mdi:ubuntu",           color: "#E95420" },
  debian:    { icon: "mdi:debian",           color: "#A81D33" },
  centos:    { icon: "mdi:centos",           color: "#262577" },
  redhat:    { icon: "mdi:redhat",           color: "#EE0000" },
  fedora:    { icon: "mdi:fedora",           color: "#51A2DA" },
  archlinux: { icon: "mdi:arch",             color: "#1793D1" },
  alpine:    { icon: "mdi:alpine",           color: "#0D597F" },
  opensuse:  { icon: "mdi:linux",            color: "#73BA25" },
  suse:      { icon: "mdi:linux",            color: "#73BA25" },
  rocky:     { icon: "mdi:linux",            color: "#10B981" },
  almalinux: { icon: "mdi:linux",            color: "#0F4266" },
  gentoo:    { icon: "mdi:gentoo",           color: "#54487A" },
  manjaro:   { icon: "mdi:linux",            color: "#35BF5C" },
  kali:      { icon: "mdi:linux",            color: "#557C94" },
  pop_os:    { icon: "mdi:linux",            color: "#48B9C7" },
  linuxmint: { icon: "mdi:linux-mint",       color: "#87CF3E" },
  nixos:     { icon: "mdi:nix",              color: "#5277C3" },
  void:      { icon: "mdi:linux",            color: "#478061" },
  amazon:    { icon: "mdi:aws",              color: "#FF9900" },
  oracle:    { icon: "mdi:linux",            color: "#F80000" },
  linux:     { icon: "mdi:linux",            color: "#FCC624" },

  // Non-Linux
  macos:     { icon: "mdi:apple",            color: "#A2AAAD" },
  windows:   { icon: "mdi:microsoft-windows", color: "#00BCF2" },
  freebsd:   { icon: "mdi:freebsd",          color: "#AB2B28" },
  openbsd:   { icon: "mdi:linux",            color: "#F2CA30" },
  netbsd:    { icon: "mdi:linux",            color: "#FF6600" },
};

const osTypeMap: Record<string, OsEntry> = {
  linux:   { icon: "mdi:linux",             color: "#FCC624" },
  macos:   { icon: "mdi:apple",             color: "#A2AAAD" },
  windows: { icon: "mdi:microsoft-windows", color: "#00BCF2" },
  freebsd: { icon: "mdi:freebsd",           color: "#AB2B28" },
};

const defaultEntry: OsEntry = { icon: "mdi:server", color: "#9898A3" };

function getEntry(info: OsInfo | null | undefined): OsEntry {
  if (!info) return defaultEntry;
  return distroMap[info.distro] ?? osTypeMap[info.os_type] ?? defaultEntry;
}

export function getOsIcon(info: OsInfo | null | undefined): string {
  return getEntry(info).icon;
}

export function getOsColor(info: OsInfo | null | undefined): string {
  return getEntry(info).color;
}

export function getOsLabel(info: OsInfo | null | undefined): string {
  if (!info || info.distro === "unknown") return "Unknown OS";
  const name = info.distro.charAt(0).toUpperCase() + info.distro.slice(1);
  return name;
}
