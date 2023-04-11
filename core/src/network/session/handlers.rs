use util::{
    bail, Deserialize, Result,
};
use util::bytes::MutableBuffer;

use crate::network::{Attribute, CommandOutput, CommandOutputMessage, CommandOutputType, CommandRequest, SettingsCommand, TextData, UpdateAttributes};
use crate::network::{
    {
        Animate, RequestAbility,
        TextMessage,
        UpdateSkin,
    },
    Session,
};
use crate::command::ParsedCommand;

use super::SessionLike;

impl Session {
    pub fn process_settings_command(&self, packet: MutableBuffer) -> anyhow::Result<()> {
        let request = SettingsCommand::deserialize(packet.as_ref())?;
        tracing::info!("{request:?}");

        Ok(())
    }

    pub fn process_text_message(&self, packet: MutableBuffer) -> anyhow::Result<()> {
        let request = TextMessage::deserialize(packet.as_ref())?;
        if !matches!(request.data, TextData::Chat { .. }) {
            anyhow::bail!("Client is only allowed to send chat messages");
        }

        // We must also return the packet to the client that sent it.
        // Otherwise their message won't be displayed in their own chat.
        self.broadcast(request)
    }

    pub fn process_skin_update(&self, packet: MutableBuffer) -> anyhow::Result<()> {
        let request = UpdateSkin::deserialize(packet.as_ref())?;
        self.broadcast(request)
    }

    pub fn process_ability_request(&self, packet: MutableBuffer) -> anyhow::Result<()> {
        let request = RequestAbility::deserialize(packet.as_ref())?;
        tracing::info!("{request:?}");

        Ok(())
    }

    pub fn process_animation(&self, packet: MutableBuffer) -> anyhow::Result<()> {
        let _request = Animate::deserialize(packet.as_ref())?;

        Ok(())
    }

    pub fn process_command_request(&self, packet: MutableBuffer) -> anyhow::Result<()> {
        let request = CommandRequest::deserialize(packet.as_ref())?;

        todo!();
    }
}
