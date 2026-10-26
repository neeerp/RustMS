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
    ChangeChannel = 0x10,
    CharList = 0x0B,
    ServerIp = 0x0C,
    CharNameResponse = 0x0D,
    LastConnectedWorld = 0x1A,
    RecommendedWorlds = 0x1B,

    InventoryOperation = 0x1D,
    InventoryGrow = 0x1E,
    GiveBuff = 0x20,
    CancelBuff = 0x21,
    StatChange = 0x1F,
    UpdateSkills = 0x24,
    FameResponse = 0x26,
    ShowStatusInfo = 0x27,
    MemoResult = 0x29,
    MapTransferResult = 0x2A,
    /// What became of a report of a player (FUN_00a27891).
    ClaimResult = 0x2D,
    /// The hours reports are taken, start and end (FUN_00a27b38).
    ClaimAvailableTime = 0x2E,
    /// Whether reports are taken at all (FUN_00a27b61).
    ClaimStatusChanged = 0x2F,
    SetTamingMobInfo = 0x30,
    /// A quest's title is earned (`u16` quest id) (0xa26d44).
    QuestClear = 0x31,
    SkillBookResult = 0x33,
    GatherItemResult = 0x34,
    SortItemResult = 0x35,
    /// Whether a hired merchant permit may be used, by code (50,
    /// FUN_00a27d75; report 165 §5).
    EntrustedShopCheckResult = 0x32,
    /// What a Pigmy Egg held, or 0 for a full inventory (FUN_00a28298).
    IncubatorResult = 0x45,
    /// The Owl of Minerva's answers by mode: 6 a search's results, 7 the
    /// most searched items (FUN_00a28c29).
    OwlResult = 0x46,
    /// Whether GO on a row of the Owl's results may go (FUN_00a29013).
    OwlGoResult = 0x47,
    /// A proposal to the partner, by mode (report b052).
    MarriageRequest = 0x48,
    /// The answer to a proposal, by mode; closes the proposer's waiting box.
    MarriageResult = 0x49,
    /// The wedding registry's windows and answers, by mode (report b084).
    WeddingGiftResult = 0x4A,
    /// The partner's map, or 0 (report b057).
    NotifyMarriedPartnerMapTransfer = 0x4B,
    CashPetFoodResult = 0x4C,
    BridleMobCatchFail = 0x4F,

    PartyOperation = 0x3E,
    SpawnPortal = 0x43,
    BroadcastMsg = 0x44,
    CharInfo = 0x3D,
    BuddyList = 0x3F,
    GuildOperation = 0x41,
    /// Guild alliances ("Guild Union"), by mode (0xa39f4e).
    AllianceOperation = 0x42,
    /// The guild bulletin board's list and threads (0x816c32).
    GuildBbs = 0x3B,
    NotifyLevelUp = 0x69,
    NotifyMarriage = 0x6A,
    NotifyJobChange = 0x6B,
    /// An avatar megaphone was refused (`u8` reason) (0xa2a3bc).
    AvatarMegaphoneResult = 0x6E,
    /// An avatar megaphone's banner (0xa2a486).
    AvatarMegaphone = 0x6F,
    /// Hide the avatar megaphone's banner (0xa2a65b).
    ClearAvatarMegaphone = 0x70,
    /// The answer to cancelling a Name Change request from the bag
    /// (0xa2a677; report c060).
    CancelNameChangeResult = 0x71,
    /// The character has no Name Change request to cancel (0xa2a7e6).
    NoNameChange = 0x78,
    /// Whether the Equip window shows the second pendant cell (report b144).
    ExtraPendantSlot = 0x79,
    MonsterBookSetCard = 0x53,
    MonsterBookSetCover = 0x54,
    /// A named value the client keeps for the session, as the Dojo's
    /// `energy` (FUN_00a12bd6).
    SessionValue = 0x5A,
    /// The Family Tree (report b039).
    FamilyChart = 0x5E,
    FamilyInfo = 0x5F,
    /// A refusal or an end, by code.
    FamilyResult = 0x60,
    /// Someone asks the player to be their junior.
    FamilyJoinRequest = 0x61,
    /// The junior asked answered.
    FamilyJoinRequestResult = 0x62,
    /// The player has a new senior.
    FamilyJoinAccepted = 0x63,
    /// The entitlements and their prices.
    FamilyPrivilegeList = 0x64,
    FamilyRepGain = 0x65,
    /// A senior or junior came online or left.
    FamilyNotifyLoginOrLogout = 0x66,
    /// The icon of the rate entitlement working for the player.
    FamilySetPrivilege = 0x67,
    /// A family member asks to summon the player.
    FamilySummonRequest = 0x68,
    /// The answer to a Party Potion used on someone (0xA0): unlocks, or a
    /// notice (report c041).
    RandomMorphRes = 0x77,
    /// A line among the floating script messages (0xa13f20).
    ScriptProgressMessage = 0x7A,
    MacroSysDataInit = 0x7C,
    SetField = 0x7D,
    SetCashShop = 0x7F,
    BlockedMap = 0x83,
    BlockedServer = 0x84,
    /// An event field's word to the player: their Coconut team (b167).
    FieldSpecificData = 0x85,
    FieldEffect = 0x8A,
    /// A named map object, such as a trap, shown in another state (139).
    FieldObstacleOnOff = 0x8B,
    /// The states of a field's named map objects, for a player entering (140).
    FieldObstacleOnOffList = 0x8C,
    BlowWeather = 0x8E,
    PlayJukebox = 0x8F,
    AdminResult = 0x90,
    /// The OX Quiz's question or answer (b171).
    OxQuiz = 0x91,
    GmEventInstructions = 0x92,
    Clock = 0x93,
    ContiMove = 0x94,
    ContiState = 0x95,
    StopClock = 0x9A,
    PyramidGauge = 0x9D,
    PyramidScore = 0x9E,
    QuickSlotInit = 0x9F,
    MultiChat = 0x86,
    Whisper = 0x87,
    /// Spouse chat: the sender's echo or the partner's line (FUN_00532087).
    SpouseChat = 0x88,
    /// A summoning sack refused on this map (FUN_00532fcf).
    SummonItemUnavailable = 0x89,
    SpawnPlayer = 0xA0,
    RemovePlayerFromMap = 0xA1,
    ChatText = 0xA2,
    Chalkboard = 0xA4,
    UpdateCharBox = 0xA5,
    /// An item buff's aura over a player, the owner included (FUN_0093abc0).
    ShowConsumeEffect = 0xA6,
    ShowScrollEffect = 0xA7,
    SpawnPet = 0xA8,
    MovePet = 0xAA,
    PetChat = 0xAB,
    PetNameChange = 0xAC,
    PetExceptionList = 0xAD,
    PetCommand = 0xAE,
    SpawnSummon = 0xAF,
    RemoveSummon = 0xB0,
    MoveSummon = 0xB1,
    SummonAttack = 0xB2,
    SummonSkill = 0xB3,
    DamageSummon = 0xB4,
    ShowItemGainInChat = 0xCE,
    /// Up a Mu Lung Dojo floor to its portal by index (r25 0x959727: a byte
    /// that, if set, lets the client ask again; then the portal's).
    DojoWarpUp = 0xCF,
    /// "You have used the meso bag to receive %d mesos." (FUN_009599bb;
    /// LUCKSACK_FAIL in other servers, though it is the good news).
    MesoBagUsed = 0xD1,
    /// "You have failed to use the meso bag." (FUN_00959a47).
    MesoBagFailed = 0xD2,
    UpdateQuestInfo = 0xD3,
    HpTaken = 0xD4,
    /// What the Item Maker made (FUN_0095dad3).
    MakerResult = 0xD9,
    /// A cash mist's face for its user (FUN_00938b3e).
    ItemEmotion = 0xE2,
    Cooldown = 0xEA,
    SpawnMonster = 0xEC,
    KillMonster = 0xED,
    SpawnMonsterControl = 0xEE,
    MoveMonster = 0xEF,
    MoveMonsterResponse = 0xF0,
    MobStatSet = 0xF2,
    MobStatReset = 0xF3,
    DamageMonster = 0xF6,
    ShowMonsterHp = 0xFA,
    ShowMagnet = 0xFB,
    CatchMonsterWithItem = 0xFC,
    PlayerHint = 0xD6,
    MovePlayer = 0xB9,
    CloseRangeAttack = 0xBA,
    RangedAttack = 0xBB,
    MagicAttack = 0xBC,
    EnergyAttack = 0xBD,
    SkillEffect = 0xBE,
    CancelSkillEffect = 0xBF,
    DamagePlayer = 0xC0,
    FacialExpression = 0xC1,
    ShowItemEffect = 0xC2,
    ShowForeignEffect = 0xC6,
    GiveForeignBuff = 0xC7,
    CancelForeignBuff = 0xC8,
    UpdateCharLook = 0xC5,
    UpdatePartyMemberHp = 0xC9,
    ShowChair = 0xC4,
    ThrowGrenade = 0xCC,
    CancelChair = 0xCD,
    GuildNameChanged = 0xCA,
    GuildMarkChanged = 0xCB,
    SpawnNpc = 0x101,
    RemoveNpc = 0x102,
    SpawnNpcRequestController = 0x103,
    NpcAction = 0x104,
    /// Show an NPC, or hide it, with an effect (261; report b095 §2).
    NpcEnabled = 0x105,
    /// Ask the controller to request an NPC's action by name (262).
    NpcSpecialAction = 0x106,
    /// Give NPCs scripts for some dates, for the session (263).
    SetNpcScriptable = 0x107,
    /// A hired merchant stands on the map (265; report 165 §1).
    SpawnHiredMerchant = 0x109,
    /// A hired merchant left the map (266).
    DestroyHiredMerchant = 0x10A,
    /// A hired merchant's balloon changed (267).
    UpdateHiredMerchant = 0x10B,
    DropItemFromMapObject = 0x10C,
    RemoveItemFromMap = 0x10D,
    CannotSpawnKite = 0x10E,
    SpawnKite = 0x10F,
    RemoveKite = 0x110,
    SpawnMist = 0x111,
    RemoveMist = 0x112,
    SpawnDoor = 0x113,
    RemoveDoor = 0x114,
    HitReactor = 0x115,
    SpawnReactor = 0x117,
    RemoveReactor = 0x118,
    /// A Snowball field's balls and snowmen (b168).
    SnowballState = 0x119,
    HitSnowball = 0x11A,
    SnowballMessage = 0x11B,
    /// A ball pushes the player back (b168).
    LeftKnockBack = 0x11C,
    /// A Coconut Season nut shows, shakes, bursts or falls (b167).
    CoconutHit = 0x11D,
    CoconutScore = 0x11E,
    /// The Monster Carnival's sides and CP on arrival (v83ref b163).
    CarnivalStart = 0x121,
    CarnivalObtainedCp = 0x122,
    CarnivalPartyCp = 0x123,
    /// Who summoned or cast what with their CP, and why a spend failed.
    CarnivalSummon = 0x124,
    CarnivalMessage = 0x125,
    CarnivalDied = 0x126,
    /// How a battle ended, for the chat log (296; Cosmic never sends it).
    CarnivalResult = 0x128,
    /// How long Horntail's Cave stays open (302; v83 FUN_005335a3).
    HorntailCave = 0x12E,
    /// How long the Zakum Shrine stays open (303; v83 FUN_0053347c).
    ZakumShrine = 0x12F,
    NpcDialogue = 0x130,
    /// The priest's line at the altar (320; report b075).
    WeddingProgress = 0x140,
    /// The hearts between the married couple (321; report b075).
    WeddingCeremonyEnd = 0x141,
    /// The Duey window and its parcels (322).
    Parcel = 0x142,
    QueryCashResult = 0x144,
    CashShopOperation = 0x145,
    /// Whether the name typed for a Name Change is free (0x47baea).
    CheckNewNameResult = 0x148,
    /// Whether the character may ask for a new name (0x47bbb6).
    NameChangeRulesResult = 0x149,
    /// The Rock, Paper, Scissors window (312; report b038).
    RpsGame = 0x138,
    Messenger = 0x139,
    PlayerInteraction = 0x13A,
    OpenNpcShop = 0x131,
    ConfirmShopTransaction = 0x132,
    TrunkResult = 0x135,
    /// What came of taking everything from Fredrick's Store Bank (310).
    StoreBankMessage = 0x136,
    /// Fredrick's Store Bank: the window, the fee, or where the merchant
    /// is (311).
    StoreBank = 0x137,

    KeyMap = 0x14F,
    AutoHpPot = 0x150,
    AutoMpPot = 0x151,
    /// A Maple TV message (0x6371c1).
    SendTv = 0x155,
    /// Clear the Maple TV (0x6371ad).
    RemoveTv = 0x156,
    /// How sending a Maple TV message went (0x6373a0).
    TvResult = 0x157,
    ViciousHammer = 0x162,
    VegaScroll = 0x166,
}
