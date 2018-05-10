Templates

Use `cargo test --features="skeptic"` to run the examples in the README using the `skeptic` crate. 
 
```rust,skt-run
extern crate dipstick;
use dipstick::*;

fn main() {{
    {}
}}
```

```rust,skt-fail
extern crate dipstick;
use dipstick::*;
use std::result::Result;

fn main() {{
    run().ok();
}}

fn run() -> Result<(), dipstick::error::Error> {{
    {}
    Ok(())
}}
```


```rust,skt-plain
{}
```