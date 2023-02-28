use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use ibc::core::ics04_channel::msgs::chan_open_ack::MsgChannelOpenAck;
use ibc::core::ics24_host::identifier::PortId;
use penumbra_storage::{StateRead, StateWrite};
use penumbra_transaction::Transaction;

use crate::action_handler::ActionHandler;
use crate::ibc::component::channel::execution::channel_open_ack::ChannelOpenAckExecute;
use crate::ibc::component::channel::stateful::channel_open_ack::ChannelOpenAckCheck;
use crate::ibc::ibc_handler::{AppHandlerCheck, AppHandlerExecute};
use crate::ibc::transfer::Ics20Transfer;

#[async_trait]
impl ActionHandler for MsgChannelOpenAck {
    async fn check_stateless(&self, _context: Arc<Transaction>) -> Result<()> {
        // NOTE: no additional stateless validation is possible

        Ok(())
    }

    async fn check_stateful<S: StateRead + 'static>(&self, state: Arc<S>) -> Result<()> {
        state.validate(self).await?;
        let transfer = PortId::transfer();
        if self.port_id_on_a == transfer {
            Ics20Transfer::chan_open_ack_check(state, self).await?;
        } else {
            return Err(anyhow::anyhow!("invalid port id"));
        }

        Ok(())
    }

    async fn execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        state.execute(self).await;
        let transfer = PortId::transfer();
        if self.port_id_on_a == transfer {
            Ics20Transfer::chan_open_ack_execute(state, self).await;
        } else {
            return Err(anyhow::anyhow!("invalid port id"));
        }

        Ok(())
    }
}
