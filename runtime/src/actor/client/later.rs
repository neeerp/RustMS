use super::ClientActor;
use crate::error::RuntimeError;
use net::packet::op::SendOpcode;
use packet::Packet;
use std::time::Duration;
use tokio::time::Instant;

impl ClientActor {
    /// Send `packet` `after` this long. Packets due at the same time go in
    /// the order they were queued.
    pub(super) fn reply_later(&mut self, after: Duration, packet: Packet) {
        let due = Instant::now() + after;
        let at = self.later.partition_point(|(when, _)| *when <= due);
        self.later.insert(at, (due, packet));
    }

    /// When the next queued packet is due.
    pub(super) fn later_due(&self) -> Option<Instant> {
        self.later.first().map(|(due, _)| *due)
    }

    /// Send every queued packet that is due.
    pub(super) async fn send_later(&mut self) -> Result<(), RuntimeError> {
        let now = Instant::now();
        let due = self.later.partition_point(|(when, _)| *when <= now);
        self.write_later(due).await
    }

    /// Send every queued packet now if `packet` changes the inventory. The
    /// queued ones carry slots' states as they were when they were queued,
    /// so a newer change, such as a pickup into a slot they empty, must not
    /// reach the client before them.
    pub(super) async fn send_later_before(&mut self, packet: &Packet) -> Result<(), RuntimeError> {
        if self.later.is_empty() || !changes_inventory(packet) {
            return Ok(());
        }
        self.write_later(self.later.len()).await
    }

    async fn write_later(&mut self, count: usize) -> Result<(), RuntimeError> {
        for (_, mut packet) in self.later.drain(..count).collect::<Vec<_>>() {
            self.write(&mut packet).await?;
        }
        Ok(())
    }
}

fn changes_inventory(packet: &Packet) -> bool {
    packet.opcode() == SendOpcode::InventoryOperation as i16
}
