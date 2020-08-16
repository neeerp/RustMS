#[derive(FromPrimitive)]
pub enum SendOpcode {
    LoginStatus = 0x00,
    GuestIdLogin = 0x01,
    ServerStatus = 0x03,
    CheckPin = 0x06,
    UpdatePin = 0x07,
    ServerList = 0x0A,
    CharList = 0x0B,
    CharNameResponse = 0x0D,
    LastConnectedWorld = 0x1A,
    RecommendedWorlds = 0x1B,
}
