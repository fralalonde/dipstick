dipstick
--------
Configurable metrics toolkit for Rust applications

```rust
    let channel_a = ProxyChannel::new( LogChannel::new() );
    let channel_b = ProxyChannel::new( StatsdChannel::new("localhost:8125", "hello.").unwrap() );
    let channel_x = DualChannel::new( channel_a, channel_b );
    let sugar_x = SugarChannel::new(channel_x);
    let counter = sugar_x.new_count("sugar_count_a");
    counter.value(1);
```

##TODO
- scopes
- sampling
- tags
- tests
- bench
- doc
- samples