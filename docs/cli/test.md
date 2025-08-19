### `hk test`

Run per-step tests defined in `hk.pkl`.

Usage:

```bash
hk test [--step STEP]... [--name NAME]... [--list]
```

Flags:

- `--step STEP`: Filter by step name (repeatable)
- `--name NAME`: Filter by test name (repeatable)
- `--list`: List matching tests without running

Notes:

- Tests run in a temporary sandbox. They do not modify your repo.
- Concurrency respects global `-j/--jobs`.

Example step test in `hk.pkl`:

```pkl
steps {
  rustfmt {
    check = "rustfmt --check {{ files }}"
    fix = "rustfmt {{ files }}"
    tests {
      ["formats simple file"] {
        run = "fix"
        write = { "src/example.rs" = """
          fn  main(){println!("hi");}
        """ }
        files = ["src/example.rs"]
        expect {
          code = 0
          files { ["src/example.rs"] = """
            fn main() {
                println!("hi");
            }
          """ }
        }
      }
    }
  }
}
```
