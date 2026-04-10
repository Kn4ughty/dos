{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  buildInputs =  with pkgs; [
    pkgsCross.i686-embedded.buildPackages.gcc
    pkgsCross.i686-embedded.buildPackages.binutils
    nasm
    qemu
    grub2
    xorriso
    mtools
    libisoburn
  ];
}
