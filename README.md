# dotenv Parser

This is a library for parsing dotenv files. It was built for educational purposes and is not intended to replace any other libraries or be used in production.

Please feel free, though, to use it and report any bugs or issues.

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

### **Note that this parser does not support quoted or multi-line keys or values (yet).**