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
    EnterCashShop = 0x28,

    PlayerMove = 0x29,
    CancelChair = 0x2A,
    UseChair = 0x2B,
    CloseRangeAttack = 0x2C,
    RangedAttack = 0x2D,
    MagicAttack = 0x2E,
    TouchMonsterAttack = 0x2F,
    TakeDamage = 0x30,
    AllChat = 0x31,
    CloseChalkboard = 0x32,
    /// A Water of Life was double-clicked (FUN_00a1dce6).
    WaterOfLife = 0x75,
    MultiChat = 0x77,
    PartyOperation = 0x7C,
    AdminCommand = 0x80,
    BuddyListModify = 0x82,
    NoteAction = 0x83,
    UseDoor = 0x85,
    /// The Rock, Paper, Scissors window's buttons (report b038).
    RpsAction = 0x88,
    /// Proposals with an engagement ring box, by mode (report b052).
    RingAction = 0x89,
    DenyPartyRequest = 0x7D,
    GuildOperation = 0x7E,
    DenyGuildRequest = 0x7F,
    /// Alliance requests, by op (0x6ec9ce).
    AllianceOperation = 0x8F,
    /// An alliance invitation declined, by the card or the client itself.
    DenyAllianceRequest = 0x90,
    /// A character's Family Tree (report b039).
    OpenFamilyPedigree = 0x91,
    /// The Family window opened.
    OpenFamily = 0x92,
    /// Add Junior, by name.
    AddFamily = 0x93,
    /// A senior parting from a junior, by id (report b048).
    SeparateFamilyBySenior = 0x94,
    /// A junior parting from their senior.
    SeparateFamilyByJunior = 0x95,
    /// An answer to a junior request (the card, or the client itself).
    AcceptFamily = 0x96,
    /// An entitlement bought from the Family window, with a family
    /// member's name for those that need one.
    UseFamily = 0x97,
    /// The leader's new precepts, from the Family window.
    ChangeFamilyMessage = 0x98,
    /// The answer to a family member's summons.
    FamilySummonResponse = 0x99,
    /// The guild bulletin board (FUN_008166f6 and its siblings).
    GuildBbs = 0x9B,
    FaceExpression = 0x33,
    UseItemEffect = 0x34,
    Whisper = 0x78,
    Messenger = 0x7A,
    PlayerInteraction = 0x7B,
    MonsterBookCover = 0x39,
    TalkToNpc = 0x3A,
    NpcTalkMore = 0x3C,
    NpcShopAction = 0x3D,
    TrunkRequest = 0x3E,
    DueyAction = 0x41,
    /// The Owl of Minerva's search window opened (`u8 5`): it wants the
    /// most searched items.
    OwlOpen = 0x42,
    /// GO on a row of the Owl's results: the shop and its map.
    OwlGo = 0x43,
    GatherItems = 0x45,
    SortItems = 0x46,
    ItemMove = 0x47,
    UseItem = 0x48,
    UseSummonBag = 0x4B,
    PetFood = 0x4C,
    UseMountFood = 0x4D,
    UseCashItem = 0x4F,
    /// A doll that can't be revived (FUN_00a240a2), before the client
    /// offers the Cash Shop.
    DeadPet = 0x50,
    UseCatchItem = 0x51,
    CancelItemEffect = 0x49,
    UseSkillBook = 0x52,
    /// The Owl of Minerva's search (FUN_008a2407).
    UseOwl = 0x53,
    UseTeleportRock = 0x54,
    UseReturnScroll = 0x55,
    UseUpgradeScroll = 0x56,
    SpendAp = 0x57,
    HealOverTime = 0x59,
    SpendSp = 0x5A,
    UseSkill = 0x5B,
    CancelBuff = 0x5C,
    SkillEffect = 0x5D,
    MesoDrop = 0x5E,
    GiveFame = 0x5F,
    CharInfoRequest = 0x61,
    SpawnPet = 0x62,
    ChangeMapSpecial = 0x64,
    TeleportRockList = 0x66,
    QuestAction = 0x6B,
    CalcDamageStat = 0x6C,
    ThrowGrenade = 0x6D,
    SkillMacro = 0x6E,
    /// A box with a `reward` node (FUN_00a1249f): the USE slot and the item.
    UseItemReward = 0x70,
    /// The Item Maker's Start, once its gauge is full (FUN_00827096).
    ItemMake = 0x71,

    ChangeKeybinds = 0x87,
    UseSolomonItem = 0x9D,
    UseGachaExp = 0x9E,
    /// An item used on the character named in v83's Transform dialog: the
    /// Maplemas Party Potion (report c041).
    UseTargetItem = 0xA0,
    MovePet = 0xA7,
    PetChat = 0xA8,
    PetCommand = 0xA9,
    PetLoot = 0xAA,
    PetAutoPot = 0xAB,
    PetExcludeItems = 0xAC,
    MoveSummon = 0xAF,
    SummonAttack = 0xB0,
    DamageSummon = 0xB1,
    Beholder = 0xB2,
    QuickSlot = 0xB7,
    MoveLife = 0xBC,
    MobDistance = 0xBD,
    MobDamageMobFriendly = 0xC0,
    MobBomb = 0xC1,
    MobDamageMob = 0xC2,
    NpcAction = 0xC5,
    ItemPickup = 0xCA,
    DamageReactor = 0xCD,

    PlayerMapTransfer = 0xCF,
    ContiStateRequest = 0xDC,
    PartySearchStart = 0xDE,
    PartySearchStop = 0xDF,
    CheckCash = 0xE4,
    CashShopOperation = 0xE5,
    CouponCode = 0xE6,
    UseHammer = 0x104,

    UnusedOpcode = 0xFF,
}
