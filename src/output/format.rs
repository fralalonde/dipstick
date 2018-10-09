use core::name::Name;
use core::input::Kind;
use core::Value;
use self::TemplateCmd::*;

use std::io;

pub enum TemplateCmd {
    StringLit(String),
    ValueAsText,
}

pub struct Template {
    commands: Vec<TemplateCmd>
}

impl Template {
    pub fn print(&self, output: &mut io::Write, value: Value) -> Result<(), io::Error> {
        for cmd in &self.commands {
            match cmd {
                StringLit(src) => output.write_all(src.as_ref())?,
                ValueAsText => output.write_all(format!("{}", value).as_ref())?,
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
pub struct LineFormat;

impl Format for LineFormat {
    fn template(&self, name: &Name, _kind: Kind) -> Template {
        let mut header = name.join(".");
        header.push(' ');
        Template {
            commands: vec![
                StringLit(header),
                ValueAsText,
                StringLit("\n".to_owned())
            ]
        }
    }

}
