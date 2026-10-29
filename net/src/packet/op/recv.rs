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
    /// Name Change bought in the Cash Shop: the character id and birthday
    /// code (0x4733ca; report c060). Its name check is `CheckCharName`.
    NameChangeRules = 0x10,
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
    /// A Remote Gachapon Ticket's window picked a town (0xa14074).
    RemoteGachapon = 0x74,
    /// A Water of Life was double-clicked (FUN_00a1dce6).
    WaterOfLife = 0x75,
    /// A GM's `/notice*`, `/alert*` or `/slide*` (report b175).
    AdminBroadcast = 0x76,
    MultiChat = 0x77,
    PartyOperation = 0x7C,
    AdminCommand = 0x80,
    /// The line of each GM slash command, after its packet (0x52e28f).
    AdminLog = 0x81,
    BuddyListModify = 0x82,
    NoteAction = 0x83,
    UseDoor = 0x85,
    /// A GM's `/slideax` (report b175).
    GmSlide = 0x86,
    /// The Rock, Paper, Scissors window's buttons (report b038).
    RpsAction = 0x88,
    /// Proposals with an engagement ring box, by mode (report b052).
    RingAction = 0x89,
    /// The Wedding Gift window's give, take and close (report b085).
    WeddingAction = 0x8A,
    /// One of the couple closed the priest's line (report b075).
    WeddingTalk = 0x8B,
    /// A guest at the Cathedral's altar said Yes to blessing the couple
    /// (report b075).
    WeddingBless = 0x8C,
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
    /// A dead player's Wheel of Destiny, with where they lie (FUN_0095af8e).
    UseDeathItem = 0x35,
    Whisper = 0x78,
    /// A line to the player's spouse (FUN_008defff).
    SpouseChat = 0x79,
    Messenger = 0x7A,
    PlayerInteraction = 0x7B,
    MonsterBookCover = 0x39,
    TalkToNpc = 0x3A,
    /// A Store Remote Controller was used; the client waits for
    /// ENTRUSTED_SHOP_CHECK_RESULT.
    RemoteShopRequest = 0x3B,
    NpcTalkMore = 0x3C,
    NpcShopAction = 0x3D,
    TrunkRequest = 0x3E,
    /// A hired merchant permit was used; the client waits for
    /// ENTRUSTED_SHOP_CHECK_RESULT.
    EntrustedShopRequest = 0x3F,
    /// The Store Bank window's Get All, its confirm, or its closing.
    StoreBankRequest = 0x40,
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
    /// A scripted item (243xxxx) used from the Use tab (FUN_00a09b26): the
    /// tick, its slot and the item.
    UseScriptedItem = 0x4E,
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
    /// A report of a player (CWvsContext::SendClaim, FUN_00a2719c).
    Claim = 0x6A,
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
    /// A trap hurt a mob, as the client saw it (0x667d46).
    FieldDamageMob = 0xBF,
    MobDamageMobFriendly = 0xC0,
    MobBomb = 0xC1,
    MobDamageMob = 0xC2,
    NpcAction = 0xC5,
    ItemPickup = 0xCA,
    DamageReactor = 0xCD,
    /// The player stepped onto or off a reactor set off by touch (185 §6).
    TouchReactor = 0xCE,

    PlayerMapTransfer = 0xCF,
    /// An attack hit a Snowball field's ball or snowman (b168).
    Snowball = 0xD3,
    /// The player walked into a ball (b168).
    LeftKnockback = 0xD4,
    /// An attack hit a Coconut Season nut (b167).
    Coconut = 0xD5,
    /// The Monster Carnival window spends CP (v83ref b163).
    MonsterCarnival = 0xDA,
    ContiStateRequest = 0xDC,
    PartySearchStart = 0xDE,
    PartySearchStop = 0xDF,
    CheckCash = 0xE4,
    CashShopOperation = 0xE5,
    CouponCode = 0xE6,
    /// The raise window asks for its record's value: the record (b233).
    RaiseRecord = 0xEC,
    /// An item dragged onto the raise window: its bag, its slot, and the
    /// raise item (b233).
    RaiseFeed = 0xEE,
    UseHammer = 0x104,

    UnusedOpcode = 0xFF,
}
