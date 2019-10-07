# buffo

interview coding project - serializing an array of strings

## Usage

```rust
// Construct Buffo from iterator of &str
let buffo = Buffo::str_array(vec!["Marge", "Homer", "Bart", "Lisa", "Maggie"]);
// NB: bytes aren't actually validated right now
let buffo = Buffo::from_bytes(...).expect("corrupt Buffo");

// access nth string in buffo
let oldest: &str = buffo.nth_str(0).unwrap();
let youngest: &str = buffo.nth_str(buffo.count() - 1).unwrap();

// iterate strings in buffo
let simpsons: Vec<&str> = buffo.iter_strs().collect();
```

## Docs

```
cargo doc
```

## Tests

```
cargo test
```

The property test can take some time; to run only unit tests:

```
cargo test --lib
```
