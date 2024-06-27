# Simple persistent key/value database

This project is part of my effort to learn Rust. The goal was not to build an
efficient database but instead to understand the Rust language with an emphasis
on:

- Traits
- Command line argument parsing
- Error handling
- File IO
- Data durability with CRC checking
- String vs &str

After building the project, you can run the executable to see the help:

```text
$ cargo run --quiet
Usage: kvstore.exe <DATABASE> <COMMAND>

Commands:
  delete  Delete a value from the database
  get     Get a value from the database
  insert  Insert a value into the database
  update  Update a value in the database
  help    Print this message or the help of the given subcommand(s)

Arguments:
  <DATABASE>  Database file name

Options:
  -h, --help     Print help
  -V, --version  Print version
```

Adding a value to the database:

```text
cargo run --quiet -- kvstore.db insert hello world
```

Retrieving a value from the database:

```text
cargo run --quiet -- kvstore.db get hello
```

Updating a value from the database:

```text
cargo run --quiet -- kvstore.db update hello "new world"
```

Deleting a value from the database:

```text
cargo run --quiet -- kvstore.db delete hello
```
