mod handshake;
pub mod login;
pub mod world;

pub use self::handshake::build_handshake_packet;

#[cfg(test)]
mod tests {
    use crate::packet::op::SendOpcode;

    /// Opcode values from HeavenMS's `SendOpcode`, for every opcode a builder
    /// uses. Builder tests compare against the enum, so this pins the enum.
    #[test]
    fn send_opcodes_match_v83_reference() {
        let table: [(SendOpcode, i16); 31] = [
            (SendOpcode::LoginStatus, 0x00),
            (SendOpcode::GuestIdLogin, 0x01),
            (SendOpcode::ServerStatus, 0x03),
            (SendOpcode::ServerList, 0x0A),
            (SendOpcode::CharList, 0x0B),
            (SendOpcode::ServerIp, 0x0C),
            (SendOpcode::CharNameResponse, 0x0D),
            (SendOpcode::NewCharacter, 0x0E),
            (SendOpcode::DeleteCharacter, 0x0F),
            (SendOpcode::ChangeChannel, 0x10),
            (SendOpcode::LastConnectedWorld, 0x1A),
            (SendOpcode::RecommendedWorlds, 0x1B),
            (SendOpcode::StatChange, 0x1F),
            (SendOpcode::ShowStatusInfo, 0x27),
            (SendOpcode::SetField, 0x7D),
            (SendOpcode::SetCashShop, 0x7F),
            (SendOpcode::Whisper, 0x87),
            (SendOpcode::SpawnPlayer, 0xA0),
            (SendOpcode::RemovePlayerFromMap, 0xA1),
            (SendOpcode::ChatText, 0xA2),
            (SendOpcode::MovePlayer, 0xB9),
            (SendOpcode::SpawnNpc, 0x101),
            (SendOpcode::SpawnNpcRequestController, 0x103),
            (SendOpcode::NpcAction, 0x104),
            (SendOpcode::NpcEnabled, 0x105),
            (SendOpcode::NpcSpecialAction, 0x106),
            (SendOpcode::SetNpcScriptable, 0x107),
            (SendOpcode::NpcDialogue, 0x130),
            (SendOpcode::QueryCashResult, 0x144),
            (SendOpcode::CashShopOperation, 0x145),
            (SendOpcode::KeyMap, 0x14F),
        ];
        for (opcode, value) in table {
            assert_eq!(opcode as i16, value);
        }
    }
}
