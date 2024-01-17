use util::{BinaryRead, SharedBuffer};
use util::Deserialize;
use crate::bedrock::ConnectedPacket;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FormCancelReason {
    /// The client closed the form.
    Closed,
    /// The client was busy. This for example happens when the client's chat is open and the form cannot be displayed.
    Busy
}

impl TryFrom<u8> for FormCancelReason {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<FormCancelReason> {
        Ok(match value {
            0 => FormCancelReason::Closed,
            1 => FormCancelReason::Busy,
            v => anyhow::bail!("Expected either 0 or 1 for forms cancel reason, got {v}")
        })
    }
}

#[derive(Debug)]
pub struct FormResponseData<'a> {
    pub id: u32,
    pub response_data: Option<&'a str>,
    pub cancel_reason: Option<FormCancelReason>
}

impl<'a> ConnectedPacket for FormResponseData<'a> {
    const ID: u32 = 0x65;
}

impl<'a> Deserialize<'a> for FormResponseData<'a> {
    fn deserialize(mut buffer: SharedBuffer<'a>) -> anyhow::Result<FormResponseData<'a>> {
        let id = buffer.read_var_u32()?;
        let has_data = buffer.read_bool()?;
        let response_data = if has_data {
            Some(buffer.read_str()?)
        } else {
            None
        };

        let has_reason = buffer.read_bool()?;
        let cancel_reason = if has_reason {
            Some(buffer.read_u8()?)
        } else {
            None
        };

        Ok(FormResponseData {
            id,
            response_data,
            cancel_reason: cancel_reason.map(FormCancelReason::try_from).transpose()?
        })
    }
}