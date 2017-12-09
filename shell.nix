with import <nixpkgs> {};

stdenv.mkDerivation {
    name = "rust";
    buildInputs = [
        cargo
        jack2Full
        libjack2
        SDL2
    ];
    shellHook = ''
    '';
}
