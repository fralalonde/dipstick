use core::name::Name;
use core::input::Kind;
use core::Value;
use self::LineOp::*;

use std::io;
use std::sync::Arc;

/// Print commands are steps in the execution of output templates.
pub enum LineOp {
    /// Print a string.
    Literal(String),
    /// Lookup and print label value for key, if it exists.
    LabelExists(String, Vec<LabelOp>),
    /// Print metric value as text.
    ValueAsText,
    /// Print metric value, divided by the given scale, as text.
    ScaledValueAsText(Value),
    /// Print the newline character.labels.lookup(key)
    NewLine,
}

/// Print commands are steps in the execution of output templates.
pub enum LabelOp {
    /// Print a string.
    Literal(String),
    /// Print the label key.
    LabelKey,
    /// Print the label value.
    LabelValue,
}

/// An sequence of print commands, embodying an output strategy for a single metric.
pub struct LineTemplate {
    ops: Vec<LineOp>
}

impl From<Vec<LineOp>> for LineTemplate {
    fn from(ops: Vec<LineOp>) -> Self {
        LineTemplate { ops }
    }
}

impl LineTemplate {
    /// Template execution applies commands in turn, writing to the output.
    pub fn print<L>(&self, output: &mut io::Write, value: Value, lookup: L) -> Result<(), io::Error>
    where L: Fn(&str) -> Option<Arc<String>>
    {
        for cmd in &self.ops {
            match cmd {
                Literal(src) => output.write_all(src.as_ref())?,
                ValueAsText => output.write_all(format!("{}", value).as_ref())?,
                ScaledValueAsText(scale) => {
                    let scaled = value / scale;
                    output.write_all(format!("{}", scaled).as_ref())?
                },
                NewLine => writeln!(output)?,
                LabelExists(label_key, print_label) => {
                    if let Some(label_value) = lookup(label_key.as_ref()) {
                        for label_cmd in print_label {
                            match label_cmd {
                                LabelOp::LabelValue =>
                                    output.write_all(label_value.as_bytes())?,
                                LabelOp::LabelKey =>
                                    output.write_all(label_key.as_bytes())?,
                                LabelOp::Literal(src) =>
                                    output.write_all(src.as_ref())?,
                            }
                        }
                    }
                },
            };
        }
        Ok(())
    }
}

/// Format output config support.
pub trait Formatting {
    /// Specify formatting of output.
    fn formatting(&self, format: impl LineFormat + 'static) -> Self;
}

/// Forges metric-specific printers
pub trait LineFormat: Send + Sync {

    /// Prepare a template for output of metric values.
    fn template(&self, name: &Name, kind: Kind) -> LineTemplate;
}

/// A simple metric output format of "MetricName {Value}"
#[derive(Default)]
pub struct SimpleFormat {
    // TODO make separator configurable
//    separator: String,
}

impl LineFormat for SimpleFormat {
    fn template(&self, name: &Name, _kind: Kind) -> LineTemplate {
        let mut header = name.join(".");
        header.push(' ');
        LineTemplate {
            ops: vec![
                Literal(header),
                ValueAsText,
                NewLine,
            ]
        }
    }
}

//enum Parsed {
//    Literal(String),
//    Name()
//    Value(Value),
//    StaticLabel(String),
//    DynamicLabel(String),
//}
//
//struct TemplateFormat {
//    tokens: Vec<LineToken>
//}
//
//fn parse(template: &str) -> TemplateFormat {
//
//}


#[cfg(test)]
pub mod test {
    use super::*;
    use ::Labels;

    pub struct TestFormat;

    impl LineFormat for TestFormat {
        fn template(&self, name: &Name, kind: Kind) -> LineTemplate {
            let mut header: String = format!("{:?}", kind);
            header.push('/');
            header.push_str(&name.join("."));
            header.push(' ');
            LineTemplate {
                ops: vec![
                    Literal(header),
                    ValueAsText,
                    Literal(" ".into()),
                    ScaledValueAsText(1000),
                    Literal(" ".into()),
                    LabelExists("test_key".into(), vec![
                        LabelOp::LabelKey,
                        LabelOp::Literal("=".into()),
                        LabelOp::LabelValue]),
                    NewLine,
                ]
            }
        }
    }

    #[test]
    fn print_label_exists() {
        let labels: Labels = labels!("test_key" => "456");
        let format = TestFormat {};
        let mut name = Name::from("abc");
        name = name.prepend("xyz");
        let template = format.template(&name, Kind::Counter);
        let mut out = vec![];
        template.print(&mut out, 123000, |key| labels.lookup(key)).unwrap();
        assert_eq!("Counter/xyz.abc 123000 123 test_key=456\n", String::from_utf8(out).unwrap());
    }

    #[test]
    fn print_label_not_exists() {
        let format = TestFormat {};
        let mut name = Name::from("abc");
        name = name.prepend("xyz");
        let template = format.template(&name, Kind::Counter);
        let mut out = vec![];
        template.print(&mut out, 123000, |_key| None).unwrap();
        assert_eq!("Counter/xyz.abc 123000 123 \n", String::from_utf8(out).unwrap());
    }
}
