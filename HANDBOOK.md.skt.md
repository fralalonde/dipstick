Templates

Use `cargo test --features="skeptic"` to run the examples in the README using the `skeptic` crate. 
 
```rust,skt-run
#[macro_use] 
extern crate dipstick;
use dipstick::*;
use std::time::Duration;

fn main() {{
    {}
}}
```

```rust,skt-fail
extern crate dipstick;
use dipstick::*;
use std::result::Result;
use std::time::Duration;

fn main() {{
    run().ok();
}}

fn run() -> Result<(), Error> {{
    {}
    Ok(())
}}
```


```rust,skt-plain
{}
```