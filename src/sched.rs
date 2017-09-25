use std::time::Duration;
use std::thread;

pub fn schedule(every: Duration, operation: F) -> JoinHandle<()>
    where F: Fn() -> ()
{
    thread::spawn(|| {
        thread::sleep(every);
        operation
    })
}