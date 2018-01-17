Templates
 
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
use dipstick::error::Error;
use std::result::Result;

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