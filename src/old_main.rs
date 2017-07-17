fn main() {
    let metrics = Metrics::sync(&[
        Sample::random(Statsd::new("host:8125", "com.instant.")),
        Aggregate::default(Statsd::new("host:8125", "com.aggregate.")),
        Log::new(INFO)
    ]);

    metrics.write(COUNT, "a_count", 45);

    let counter = metrics.new_counter("b_count");
    counter.value(5);

    let timer = metrics.new_timer("a_time");
    timer.time(|| {
        println!("a");
    });

    metrics.scope("request_scope", |scope| {
        scope.push("username", "jim");
        timer.value(33); //ms
        counter.value(8);
        metrics.write(GAUGE, "ad_hoc", 45)
    });

    let request = metrics.new_scope("request_a");
    request.scope(|scope| {
        scope.push("username", "jane");
        timer.value(33); //ms
        counter.value(8);
        metrics.write(GAUGE, "ad_hoc", 45)
    });


}

