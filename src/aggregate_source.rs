//// Aggregate
#[derive(Debug)]
pub enum StatsType {
    HitCount,
    Sum,
    Max,
    Min,
    MeanRate,
    MeanValue,
}

//type StatWrite = FnOnce(ScoreTypes, C) -> ();
//
//const EVENT_STATS: &'static [StatsType] = &[StatsType::HitCount];
//const GAUGE_STATS: &'static [StatsType] = &[StatsType::MeanValue, StatsType::Max, StatsType::Min];
//const COUNT_STATS: &'static [StatsType] = &[StatsType::MeanValue, StatsType::Max, StatsType::Min, StatsType::Sum];
//const TIMER_STATS: &'static [StatsType] = &[StatsType::MeanValue, StatsType::Max, StatsType::Min, StatsType::Sum];
//
//fn metric_stats(metric_type: MetricType) -> &'static [StatsType] {
//    match metric_type {
//        MetricType::Gauge => GAUGE_STATS,
//        MetricType::Count => COUNT_STATS,
//        MetricType::Time => TIMER_STATS,
//        MetricType::Event => EVENT_STATS
//    }
//}
//
//fn stat_value(stat_type: StatsType, scores: &ScoreTypes, interval_ms: u64) -> u64 {
//    match stat_type {
//        StatsType::MeanValue => scores.value_sum / scores.hit_count,
//        StatsType::Max => scores.max,
//        StatsType::Min => scores.min,
//        StatsType::Sum => scores.value_sum,
//        StatsType::HitCount => scores.hit_count,
//        StatsType::MeanRate => scores.hit_count / interval_ms,
//    }
//}



//                vec!(
//                    self.write.target.define(m_type, format!("{}.count", name), sample),
//                    self.write.target.define(m_type, format!("{}.rate", name), sample)
//                    )
//            }
//            MetricType::Gauge => {
//                vec!(
//                    self.write.target.define(m_type, format!("{}{}.avg", group.metric_format, name), sample),
//                    self.write.target.define(m_type, format!("{}.max", name), sample)
//                    )
//            }
//            MetricType::Count => {
//                vec!(
//                    self.write.target.define(m_type, format!("{}.avg", name), sample),
//                    self.write.target.define(m_type, format!("{}.sum", name), sample),
//                    self.write.target.define(m_type, format!("{}.max", name), sample),
//                    self.write.target.define(m_type, format!("{}.rate", name), sample)
//                    )
//            }
//            MetricType::Time => {
//                vec!(
//                    self.write.target.define(m_type, format!("{}.avg", name), sample),
//                    self.write.target.define(m_type, format!("{}.sum", name), sample),
//                    self.write.target.define(m_type, format!("{}.max", name), sample),
//                    self.write.target.define(m_type, format!("{}.rate", name), sample)
//                    )
//            }
