use uuid::Uuid;

use util::{Result, Serialize};
use util::bytes::{BinaryWrite, MutableBuffer};

use crate::network::CommandOriginType;
use crate::network::ConnectedPacket;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CommandOutputType {
    None,
    LastOutput,
    Silent,
    AllOutput,
    DataSet,
}

#[derive(Debug, Clone)]
pub struct CommandOutputMessage<'a> {
    pub is_success: bool,
    pub message: &'a str,
    pub parameters: &'a [String],
}

#[derive(Debug, Clone)]
pub struct CommandOutput<'a> {
    pub origin: CommandOriginType,
    pub request_id: &'a str,
    pub output_type: CommandOutputType,
    pub success_count: u32,
    pub output: &'a [CommandOutputMessage<'a>],
}

impl<'a> ConnectedPacket for CommandOutput<'a> {
    const ID: u32 = 0x4f;
}

impl<'a> Serialize for CommandOutput<'a> {
    fn serialize<W>(&self, writer: W) -> anyhow::Result<()>
    where
        W: BinaryWrite
    {
        writer.write_var_u32(self.origin as u32)?;
        writer.write_uuid_le(&Uuid::nil())?;
        writer.write_str(self.request_id)?;

        match self.origin {
            CommandOriginType::Test | CommandOriginType::DevConsole => {
                writer.write_var_i64(0)?;
            }
            _ => ()
        }

        writer.write_u8(self.output_type as u8)?;
        writer.write_var_u32(self.success_count)?;

        writer.write_var_u32(self.output.len() as u32)?;
        for output in self.output {
            writer.write_bool(output.is_success)?;
            writer.write_str(output.message)?;

            writer.write_var_u32(output.parameters.len() as u32)?;
            for param in output.parameters {
                writer.write_str(param)?;
            }
        }

        if self.output_type == CommandOutputType::DataSet {
            unimplemented!();
        }

        Ok(())
    }
}