with import <nixpkgs> {};

stdenv.mkDerivation {
    name = "rust";
    buildInputs = [
        cargo
        jack2Full
        libjack2
        nodejs
        SDL2
    ];
    shellHook = ''
    '';
}
