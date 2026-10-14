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
    ChangeChannel = 0x27,

    ChangeMap = 0x26,

    PlayerMove = 0x29,
    CloseRangeAttack = 0x2C,
    TakeDamage = 0x30,
    AllChat = 0x31,
    Whisper = 0x78,
    TalkToNpc = 0x3A,
    NpcTalkMore = 0x3C,
    GatherItems = 0x45,
    SortItems = 0x46,
    ItemMove = 0x47,
    UseItem = 0x48,
    SpendAp = 0x57,
    HealOverTime = 0x59,
    SpendSp = 0x5A,
    UseSkill = 0x5B,
    MesoDrop = 0x5E,
    ChangeMapSpecial = 0x64,
    QuestAction = 0x6B,

    ChangeKeybinds = 0x87,
    MoveLife = 0xBC,
    ItemPickup = 0xCA,

    PlayerMapTransfer = 0xCF,
    PartySearchUpdate = 0xDF,

    UnusedOpcode = 0xFF,
}
