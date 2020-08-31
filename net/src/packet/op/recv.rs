#[derive(FromPrimitive)]
pub enum RecvOpcode {
    // Login Server Opcodes
    LoginCredentials = 0x01,
    GuestLogin = 0x02,
    ServerListReRequest = 0x04,
    CharListRequest = 0x05,
    ServerStatusRequest = 0x06,
    AcceptTOS = 0x07,
    SetGender = 0x08,
    AfterLogin = 0x09,
    RegisterPin = 0x0A,
    ServerListRequest = 0x0B,
    ViewAllChar = 0x0D,
    PickAllChar = 0x0E,
    CharSelect = 0x13,
    CheckCharName = 0x15,
    CreateChar = 0x16,
    DeleteChar = 0x17,
    RegisterPic = 0x1D,
    CharSelectWithPic = 0x1E,
    ViewAllPicRegister = 0x1F,
    ViewAllWithPic = 0x20,
    LoginStarted = 0x23,

    // World Server Opcodes
    PlayerLoggedIn = 0x14,
    PlayerMove = 0x29,
    AllChat = 0x31,

    ChangeKeybinds = 0x87,

    PlayerMapTransfer = 0xCF,
    PartySearch = 0xDF,

    UnusedOpcode = 0xFF,
}
