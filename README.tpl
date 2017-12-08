{{readme}}

## Design
Dipstick's design goals are to:
- support as many metrics backends as possible while favoring none
- support all types of applications, from embedded to servers
- promote metrics conventions that facilitate app monitoring and maintenance
- stay out of the way in the code and at runtime (ergonomic, fast, resilient)

## Performance
Predefined timers use a bit more code but are generally faster because their initialization cost is is only paid once.
Ad-hoc timers are redefined "inline" on each use. They are more flexible, but have more overhead because their init cost is paid on each use.
Defining a metric `cache()` reduces that cost for recurring metrics.

Run benchmarks with `cargo +nightly bench --features bench`.

## TODO
Although already usable, Dipstick is still under heavy development and makes no guarantees
of any kind at this point. See the following list for any potential caveats :
- META turn TODOs into GitHub issues
- generic publisher / sources
- feature flags
- time measurement units in metric kind (us, ms, etc.) for naming & scaling
- heartbeat metric on publish
- logger templates
- configurable aggregation (?)
- non-aggregating buffers
- framework glue (rocket, iron, gotham, indicatif, etc.)
- more tests & benchmarks
- complete doc / inline samples
- more example apps
- A cool logo
- method annotation processors `#[timer("name")]`
- fastsinks (M / &M) vs. safesinks (Arc<M>)
- `static_metric!` macro to replace `lazy_static!` blocks and handle generics boilerplate.

License: {{license}}

_this file was generated using [cargo readme](https://github.com/livioribeiro/cargo-readme)_
