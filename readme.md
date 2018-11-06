
first terminal:
```bash
$ jackdmp -d coreaudio
```

second terminal:
```bash
$ cargo run 1
```

third:
```bash
$ jack_connect system:capture_1 colours:in_1
```
