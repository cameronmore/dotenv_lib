# dotenv Parser

This is a library for parsing dotenv files. It was built for educational purposes and although tested, has not been tested against a baseline `.env` set of tests.

## Quickstart

Example of usage:
```Rust
use std::fs;
use std::collections::HashMap;

use dotenv_lib as dot;

fn main() {
    let contents = fs::read_to_string(".env").expect("unable to read file");
    let new_env_map: HashMap<String, String> =
        dot::process_dot_env(contents).expect("unable to parse env file");
    for (k, v) in new_env_map.iter() {
        println!("{} : {}", k, v);
    }
}
```

## Docs

It is designed to follow the syntax outlined [here](https://hexdocs.pm/dotenvy/dotenv-file-format.html#variable-names), but does not perform any interpolation for single-quoted values and does not support triple-quoted values (aka, `KEY="""VALUE"""`).

In brief:
- keys must start with a letter and contain only letters, underscores, and numbers.
- values terminate at a comment sign (`#`), newline (`\n`), and end-of-file.
- values may be single or double quoted.
- for values to have special characters (`#`, `=`, `\n`, `'`, and `"`), they must be single or double quoted (single to hold double quotes, double to hold single quotes).

Please feel free, though, to use it and report any bugs or issues.

### **CAUTION. This parser supports single quoted (') ad double quoted (") values. It does not support multi-line triple quoting (`key="""example \n value"""`). It also does not handle any interpolation**