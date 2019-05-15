# Latest changes + history

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
- Stream::to_new_file() 
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
 

