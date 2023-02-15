# Latest changes + history

## version 0.9.1
- Fix sleep in `basic` example (@RafalGoslawski)
- Expose attributes::MetricId+Attributes to make extending new outputs possible (@RafalGoslawski #86)
- Various clippy fixes
- Dropped boilerplate-y CoC and contribution documents. Just be nice, mkay?
- Dropped .travis.yml. Trav's dead, baby.
- Revalidated full build after overdue `cargo update`

## version 0.9.1
- Fix bugs

## version 0.9.0
- Abandon custom Result type and error module in favor 
  of io::Result usage across all API. (Based on @rtyler's comment in #80)
- Update all dependencies to latest versions
- Move Void module to output (internal change)
- Examples no longer declare `extern crate dipstick;`  

## version 0.8.0 - ("SUCH REJOICING")
- THIS VERSION HAS BEEN YANKED - API broke (again) for 0.9.0 and 0.8.0 hadn't been out long enough.
- Abandon non-threadsafe "Output"s in exchange for a simpler, more consistent API.   
  Everything is now threadsafe and thus all "Output" have been promoted to Inputs.
  No significant performance loss was observed (using parking_lot locks). 
  Some client code (custom output classes, etc.) rework might be necessary.
- Flattened internal project structure down to only two modules, including root.

## version 0.7.13
- Fixed statsd & graphite panic when running on async threadpool. 

## version 0.7.11
- Make OnFlushCancel Send + Sync (@vorner)

## version 0.7.10
- Make OnFlushCancel public
- Add dyn keyword to dyn traits

## version 0.7.9
- Prometheus uses HTTP POST, not GET
- Add proxy_multi_output example

## version 0.7.8
- Fix Prometheus output https://github.com/fralalonde/dipstick/issues/70 

## version 0.7.6
- Move to Rust 2018 using cargo fix --edition and some manual help
- Fix nightly's 'acceptable regression' https://github.com/rust-lang/rust/pull/59825
- Give each flush listener a unique id

## version 0.7.5
- Fix leak on observers when registering same metric twice. 
- Add `metric_id()` on `InputMetric`

## version 0.7.4
- Reexport `ObserveWhen` to make it public 

## version 0.7.3
- Fixed / shushed a bunch of `clippy` warnings 
- Made `clippy` part of `make` checks

## version 0.7.2

### features
- Observe gauge On Flush
- Observe gauge Periodically
- Stream::write_to_new_file() 
- Level

### Enhancement
- Use crossbeam channels & parking_lot locks by default
- Single thread scheduler

## version 0.7.1
- API changes, some methods renamed / deprecated for clarity
- Logging output now have selectable level

## version 0.7.0 

- Add `Delegate` mechanism to allow runtime (re)configuration of metrics 
- Enhance macros to allow metrics of different types within a single block
- Additional pre-typed 'delegate' and 'aggregate' macros
 

