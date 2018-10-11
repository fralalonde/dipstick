use core::name::Name;
use core::input::Kind;
use core::Value;
use self::Print::*;
use ::LabelValue;

use std::io;

/// Print commands are steps in the execution of output templates.
pub enum Print {
    /// Print a string.
    Literal(String),
    /// Lookup and print label value for key, if it exists.
    Label(PrintLabel),
    /// Print metric value as text.
    ValueAsText,
    /// Print metric value, divided by the given scale, as text.
    ScaledValueAsText(Value),
    /// Print the newline character.
    NewLine,
}

/// Print commands are steps in the execution of output templates.
pub enum PrintLabel {
    /// Lookup and print label value for key, if it exists.
    Value(String),
}

/// An sequence of print commands, embodying an output strategy for a single metric.
pub struct Template {
    commands: Vec<Print>
}

impl Template {
    /// Template execution applies commands in turn, writing to the output.
    pub fn print<L: Fn(&str) -> Option<LabelValue>>(&self, output: &mut io::Write, value: Value, lookup: L) -> Result<(), io::Error> {
        for cmd in &self.commands {
            match cmd {
                Literal(src) => output.write_all(src.as_ref())?,
                ValueAsText => output.write_all(format!("{}", value).as_ref())?,
                ScaledValueAsText(scale) => {
                    let scaled = value / scale;
                    output.write_all(format!("{}", scaled).as_ref())?
                },
                NewLine => writeln!(output)?,
                Label(PrintLabel::Value(label_key)) => {
                    if let Some(label_value) = lookup(label_key.as_ref()) {
                        output.write_all(label_value.as_bytes())?
                    }
                }
            };
        }
        Ok(())
    }
}


/// Forges metric-specific printers
pub trait Format: Send + Sync {

    /// Prepare a template for output of metric values.
    fn template(&self, name: &Name, kind: Kind) -> Template;
}

/// A simple metric output format of "MetricName {Value}"
#[derive(Default)]
pub struct LineFormat {
//    separator: String,
}

impl Format for LineFormat {
    fn template(&self, name: &Name, _kind: Kind) -> Template {
        let mut header = name.join(".");
        header.push(' ');
        Template {
            commands: vec![
                Literal(header),
                ValueAsText,
                NewLine,
            ]
        }
    }

}


