
first terminal:
```bash
$ jackdmp -d coreaudio

jackdmp -R -d coreaudio -p 64
```

second terminal:
```bash
$ cargo run 1
```

third (if it doesn't auto-connect):
```bash
$ jack_connect system:capture_1 colours:in_1
```

---

running on nix

```bash
$ nix-shell -p jack2 SDL2
```
