# the dipstick handbook
**IN PROGRESS**

## table of contents

## introduction

## static metrics macro

For speed and easier maintenance, metrics are usually defined statically:

```rust,skt-plain
#[macro_use] extern crate dipstick;
#[macro_use] extern crate lazy_static;
use dipstick::*;

metrics!("my_app" => {
    COUNTER_A: Counter = "counter_a";
});

fn main() {
    route_aggregate_metrics(to_stdout());
    COUNTER_A.count(11);
}
```

Metric definition macros are just `lazy_static!` wrappers.
