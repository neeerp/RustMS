#[derive(FromPrimitive)]
pub enum SendOpcode {
    LoginStatus = 0x00,
    GuestIdLogin = 0x01,
    ServerStatus = 0x03,
    CheckPin = 0x06,
    UpdatePin = 0x07,
    ServerList = 0x0A,
    NewCharacter = 0x0E,
    DeleteCharacter = 0x0F,
    CharList = 0x0B,
    ServerIp = 0x0C,
    CharNameResponse = 0x0D,
    LastConnectedWorld = 0x1A,
    RecommendedWorlds = 0x1B,

    BuddyList = 0x3F,
    FamilyInfo = 0x5F,
    FamilyList = 0x64,
    SetField = 0x7D,

    KeyMap = 0x14F,
}
