//! Native NPC and quest scripts. Each is a list of pages with effects on its
//! answers; the runtime applies the effects (and persists them with any
//! quest change) before showing the next page. Text comes from the content
//! pack (`crate::content`), with committed placeholders. Lua is deferred;
//! these keep the effect-returning shape a scripting runtime would have.

use crate::content;
use crate::conti::{self, Route, State};
use crate::dialogue::{DialoguePage, Prompt};
use crate::scripts::Effect;
use game::{guild, Player};
use game_data::server_content::keys;

/// Roger's Apple, the tutorial's lesson on items.
pub const ROGERS_APPLE_QUEST: i16 = 1021;
const ROGERS_APPLE: i32 = 2010007;
const APPLE: i32 = 2010000;
const GREEN_APPLE: i32 = 2010009;
const ROGER: i32 = 2000;
const ROBIN: i32 = 2003;
const MAPLE_ADMINISTRATOR: i32 = 2007;
const SHANKS: i32 = 22000;
const HERACLE: i32 = 2010007;
const LITH_HARBOR: i32 = 104_000_000;
/// Lucas's recommendation letter, which pays for Shanks's ship.
const RECOMMENDATION_LETTER: i32 = 4031801;
const SHIP_FARE: i32 = 150;
const SHIP_LEVEL: u8 = 7;
/// The Ellinia and Orbis stations' crews: ticket sellers, ushers, and the
/// crew who let passengers off before departure.
const JOEL: i32 = 1032007;
const CHERRY: i32 = 1032008;
const PURIN: i32 = 1032009;
const AGATHA: i32 = 2012000;
const RINI: i32 = 2012001;
const ERIN: i32 = 2012002;
/// Orbis Station's guide, and the passage to the platform for Ellinia.
const ISA: i32 = 2012006;
const ORBIS_TUNNEL: i32 = 200000110;

/// The player's answer to the current prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptInput {
    Next,
    Previous,
    Yes,
    No,
    Select(i32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Say(String),
    /// Yes applies `on_yes` and moves on; no ends the conversation, saying
    /// `on_no` first if there is one.
    Ask {
        text: String,
        on_yes: Vec<Effect>,
        on_no: Option<String>,
    },
    /// Each choice, a label and its pages, replaces the menu.
    Menu {
        text: String,
        choices: Vec<(String, Vec<Page>)>,
    },
}

/// What to do after an answer: apply `effects`, then show `prompt` (or end
/// the conversation if there is none).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Step {
    pub prompt: Option<Prompt>,
    pub effects: Vec<Effect>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpcScript {
    npc_id: i32,
    pages: Vec<Page>,
    page: usize,
    /// Applied when the player reads past the last page.
    on_end: Vec<Effect>,
    /// Said instead of moving on when the items an answer gives don't fit.
    inventory_full: Option<String>,
}

impl NpcScript {
    fn new(npc_id: i32, pages: Vec<Page>) -> Self {
        Self {
            npc_id,
            pages,
            page: 0,
            on_end: Vec::new(),
            inventory_full: None,
        }
    }

    /// The script an NPC runs when clicked (`Npc.nx info/script`), as it
    /// plays for `player`.
    pub fn for_npc(npc_id: i32, player: &Player) -> Option<Self> {
        match npc_id {
            ROBIN => Some(robin()),
            SHANKS => Some(shanks(player)),
            HERACLE => Some(heracle(player)),
            JOEL => Some(ticket_seller(JOEL, &conti::TO_ORBIS, player)),
            AGATHA => Some(ticket_seller(AGATHA, &conti::TO_ELLINIA, player)),
            CHERRY => Some(usher(CHERRY, &conti::TO_ORBIS, player, conti::now())),
            RINI => Some(usher(RINI, &conti::TO_ELLINIA, player, conti::now())),
            PURIN => Some(leave_ship(PURIN, &conti::TO_ORBIS)),
            ERIN => Some(leave_ship(ERIN, &conti::TO_ELLINIA)),
            ISA => Some(isa()),
            MAPLE_ADMINISTRATOR => Some(Self::new(
                MAPLE_ADMINISTRATOR,
                vec![Page::Ask {
                    text: text(keys::SKIP_ASK),
                    on_yes: vec![Effect::Warp(LITH_HARBOR)],
                    on_no: Some(text(keys::SKIP_DECLINE)),
                }],
            )),
            _ => None,
        }
    }

    /// A quest's start or end script (`Check.img startscript`/`endscript`),
    /// as it plays for `player`.
    pub fn for_quest(script: &str, player: &Player) -> Option<Self> {
        match script {
            "q1021s" => Some(roger_start(player)),
            "q1021e" => Some(roger_end(player)),
            _ => None,
        }
    }

    pub fn npc_id(&self) -> i32 {
        self.npc_id
    }

    /// The first prompt.
    pub fn begin(&self) -> Step {
        self.step(Vec::new())
    }

    /// Advance with the player's answer. `None` means the answer did not fit
    /// the prompt (the runtime ends the conversation).
    pub fn respond(&mut self, input: ScriptInput) -> Option<Step> {
        match (&self.pages[self.page], input) {
            (Page::Say(_), ScriptInput::Next) => Some(self.advance(Vec::new())),
            (Page::Say(_), ScriptInput::Previous) if self.has_previous() => {
                self.page -= 1;
                Some(self.begin())
            }
            (Page::Ask { on_yes, .. }, ScriptInput::Yes) => {
                let effects = on_yes.clone();
                Some(self.advance(effects))
            }
            (Page::Ask { on_no, .. }, ScriptInput::No) => Some(match on_no.clone() {
                Some(text) => self.replace(text),
                None => Step::default(),
            }),
            (Page::Menu { choices, .. }, ScriptInput::Select(choice)) => {
                let (_, pages) = choices.get(usize::try_from(choice).ok()?)?;
                self.pages = pages.clone();
                self.page = 0;
                Some(self.begin())
            }
            _ => None,
        }
    }

    /// The items the last step gave don't fit: say so instead, if the
    /// script has words for it.
    pub fn inventory_full(&mut self) -> Option<Step> {
        let text = self.inventory_full.take()?;
        Some(self.replace(text))
    }

    /// End with a single page and no further effects.
    fn replace(&mut self, text: String) -> Step {
        self.pages = vec![Page::Say(text)];
        self.page = 0;
        self.on_end.clear();
        self.begin()
    }

    fn advance(&mut self, mut effects: Vec<Effect>) -> Step {
        self.page += 1;
        if self.page < self.pages.len() {
            return self.step(effects);
        }
        effects.append(&mut self.on_end);
        Step {
            prompt: None,
            effects,
        }
    }

    fn has_previous(&self) -> bool {
        self.page > 0 && matches!(self.pages[self.page - 1], Page::Say(_))
    }

    fn step(&self, effects: Vec<Effect>) -> Step {
        let npc_id = self.npc_id;
        let prompt = match &self.pages[self.page] {
            Page::Say(text) => Prompt::Text(DialoguePage {
                npc_id,
                text: text.clone(),
                has_previous: self.has_previous(),
                has_next: self.page + 1 < self.pages.len() || !self.on_end.is_empty(),
            }),
            Page::Ask { text, .. } => Prompt::YesNo {
                npc_id,
                text: text.clone(),
            },
            Page::Menu { text, choices } => {
                let mut menu = text.clone();
                for (choice, (label, _)) in choices.iter().enumerate() {
                    menu.push_str(&format!("\n#L{choice}##b{label}#k#l"));
                }
                Prompt::Selection { npc_id, text: menu }
            }
        };
        Step {
            prompt: Some(prompt),
            effects,
        }
    }
}

/// `q1021s`: Roger hurts the player and hands over an apple.
fn roger_start(player: &Player) -> NpcScript {
    let title = text(if player.stats.gender == 0 {
        keys::ROGER_TITLE_MALE
    } else {
        keys::ROGER_TITLE_FEMALE
    });
    let mut on_yes = Vec::new();
    if player.stats.hp >= 50 {
        on_yes.push(Effect::SetHp(25));
    }
    if player.inventory.count(ROGERS_APPLE) == 0 {
        on_yes.push(Effect::GiveItem {
            item_id: ROGERS_APPLE,
            quantity: 1,
        });
    }
    on_yes.push(Effect::StartQuest(ROGERS_APPLE_QUEST));
    let [greeting, explain, ask, after, last] = keys::ROGER_START;
    let mut script = NpcScript::new(
        ROGER,
        vec![
            Page::Say(content::text().get(greeting, &[("title", &title)])),
            Page::Say(text(explain)),
            Page::Ask {
                text: text(ask),
                on_yes,
                on_no: None,
            },
            Page::Say(text(after)),
            Page::Say(text(last)),
        ],
    );
    script.on_end = vec![Effect::AvatarEffect("UI/tutorial.img/28".into())];
    script.inventory_full = Some(text(keys::ROGER_INVENTORY_FULL));
    script
}

/// `q1021e`: Roger checks the HP and pays the reward.
fn roger_end(player: &Player) -> NpcScript {
    if player.stats.hp < player.stats.max_hp {
        // Not recovered: say so and stop without completing.
        return NpcScript::new(ROGER, vec![Page::Say(text(keys::ROGER_NOT_RECOVERED))]);
    }
    let mut script = NpcScript::new(
        ROGER,
        keys::ROGER_END.map(|key| Page::Say(text(key))).to_vec(),
    );
    script.on_end = vec![
        Effect::GiveItem {
            item_id: APPLE,
            quantity: 3,
        },
        Effect::GiveItem {
            item_id: GREEN_APPLE,
            quantity: 3,
        },
        Effect::GainExp(10),
        Effect::CompleteQuest(ROGERS_APPLE_QUEST),
    ];
    script.inventory_full = Some(text(keys::ROGER_END_INVENTORY_FULL));
    script
}

/// `begin7`: Shanks sails to Victoria Island for Lucas's letter or the fare.
fn shanks(player: &Player) -> NpcScript {
    let mut pages = vec![Page::Ask {
        text: text(keys::SHANKS_ASK),
        on_yes: Vec::new(),
        on_no: Some(text(keys::SHANKS_DECLINE)),
    }];
    let mut on_end = Vec::new();
    if player.inventory.count(RECOMMENDATION_LETTER) > 0 {
        pages.extend(keys::SHANKS_LETTER.map(|key| Page::Say(text(key))));
        on_end.push(Effect::TakeItem {
            item_id: RECOMMENDATION_LETTER,
            quantity: 1,
        });
    } else {
        pages.push(Page::Say(text(keys::SHANKS_FARE)));
        pages.push(Page::Say(text(if player.stats.level < SHIP_LEVEL {
            keys::SHANKS_TOO_WEAK
        } else if player.meso < SHIP_FARE {
            keys::SHANKS_NO_MESOS
        } else {
            on_end.push(Effect::SpendMeso(SHIP_FARE));
            keys::SHANKS_PAID
        })));
    }
    if !on_end.is_empty() {
        on_end.push(Effect::Warp(LITH_HARBOR));
    }
    let mut script = NpcScript::new(SHANKS, pages);
    script.on_end = on_end;
    script
}

/// Heracle founds guilds; the client then asks for the name. Founding is
/// all he offers so far.
fn heracle(player: &Player) -> NpcScript {
    let page = if player.guild.is_some() {
        Page::Say(text(keys::HERACLE_IN_GUILD))
    } else if player.stats.level < guild::MIN_LEVEL {
        Page::Say(text(keys::HERACLE_TOO_WEAK))
    } else if player.meso < guild::CREATE_COST {
        Page::Say(text(keys::HERACLE_NO_MESOS))
    } else {
        Page::Ask {
            text: text(keys::HERACLE_ASK),
            on_yes: vec![Effect::OpenGuildNamePrompt],
            on_no: Some(text(keys::HERACLE_DECLINE)),
        }
    };
    NpcScript::new(HERACLE, vec![page])
}

/// A station's ticket booth: one ticket for the route's price. Like the
/// original, it asks first and only then says the player can't pay.
fn ticket_seller(npc_id: i32, route: &Route, player: &Player) -> NpcScript {
    let short = text(keys::SHIPS_SELL_SHORT);
    let can_pay = player.meso >= route.price;
    let on_yes = if can_pay {
        vec![
            Effect::SpendMeso(route.price),
            Effect::GiveItem {
                item_id: route.ticket,
                quantity: 1,
            },
        ]
    } else {
        Vec::new()
    };
    let mut pages = vec![Page::Ask {
        text: text(keys::SHIPS_SELL_ASK),
        on_yes,
        on_no: Some(text(keys::SHIPS_SELL_DECLINE)),
    }];
    if !can_pay {
        pages.push(Page::Say(short.clone()));
    }
    let mut script = NpcScript::new(npc_id, pages);
    script.inventory_full = Some(short);
    script
}

/// `get_ticket`: the platform's usher takes a ticket and shows the player
/// to the waiting room while the ship boards, as of route time `t`.
fn usher(npc_id: i32, route: &Route, player: &Player, t: i64) -> NpcScript {
    let page = match route.timetable.state(t) {
        State::Wait if player.inventory.count(route.ticket) == 0 => {
            Page::Say(text(keys::SHIPS_BOARD_NO_TICKET))
        }
        State::Wait => Page::Ask {
            text: text(keys::SHIPS_BOARD_ASK),
            on_yes: vec![
                Effect::RequireBoarding(route.platform),
                Effect::TakeItem {
                    item_id: route.ticket,
                    quantity: 1,
                },
                Effect::Warp(route.waiting_room),
            ],
            on_no: Some(text(keys::SHIPS_BOARD_DECLINE)),
        },
        State::Start => Page::Say(text(keys::SHIPS_BOARD_CLOSING)),
        _ => Page::Say(text(keys::SHIPS_BOARD_FULL)),
    };
    NpcScript::new(npc_id, vec![page])
}

/// `goOutWaitingRoom`: back to the platform, without the ticket.
fn leave_ship(npc_id: i32, route: &Route) -> NpcScript {
    NpcScript::new(
        npc_id,
        vec![Page::Ask {
            text: text(keys::SHIPS_LEAVE_ASK),
            on_yes: vec![Effect::Warp(route.platform)],
            on_no: Some(text(keys::SHIPS_LEAVE_DECLINE)),
        }],
    )
}

/// `station_in`: Isa shows the way to Orbis Station's platforms; only the
/// one for Ellinia's ship so far.
fn isa() -> NpcScript {
    NpcScript::new(
        ISA,
        vec![Page::Menu {
            text: text(keys::ISA_MENU),
            choices: vec![(
                text(keys::ISA_PLATFORM),
                vec![Page::Ask {
                    text: text(keys::ISA_ASK),
                    on_yes: vec![Effect::Warp(ORBIS_TUNNEL)],
                    on_no: Some(text(keys::ISA_DECLINE)),
                }],
            )],
        }],
    )
}

/// `begin5`: Robin's questions and answers, as many topics and pages as the
/// text has.
fn robin() -> NpcScript {
    let content = content::text();
    let choices = (0..)
        .map_while(|topic| {
            let label = keys::robin_choice(topic);
            content.contains(&label).then(|| {
                let pages = (0..)
                    .map(|page| keys::robin_answer(topic, page))
                    .take_while(|key| content.contains(key))
                    .map(|key| Page::Say(text(&key)))
                    .collect();
                (text(&label), pages)
            })
        })
        .collect();
    NpcScript::new(
        ROBIN,
        vec![Page::Menu {
            text: text(keys::ROBIN_MENU),
            choices,
        }],
    )
}

fn text(key: &str) -> String {
    content::text().get(key, &[])
}

/// Map an NPC_TALK_MORE answer to a script input, given the prompt it
/// answers. `None` closes the conversation.
pub fn input(
    prompt: &Prompt,
    message_type: u8,
    response: i8,
    selection: Option<i32>,
) -> Option<ScriptInput> {
    match (prompt, message_type, response) {
        (Prompt::Text(_), 0, 1) => Some(ScriptInput::Next),
        (Prompt::Text(page), 0, 0) if page.has_previous => Some(ScriptInput::Previous),
        (Prompt::YesNo { .. }, 1, 1) => Some(ScriptInput::Yes),
        (Prompt::YesNo { .. }, 1, 0) => Some(ScriptInput::No),
        (Prompt::Selection { .. }, 4, 1) => selection.map(ScriptInput::Select),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(hp: i16) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.max_hp = 50;
        player.stats.hp = hp;
        player
    }

    fn kind(step: &Step) -> Option<u8> {
        step.prompt.as_ref().map(|p| match p {
            Prompt::Text(_) => 0,
            Prompt::YesNo { .. } => 1,
            Prompt::Selection { .. } => 4,
        })
    }

    fn buttons(step: &Step) -> (bool, bool) {
        match &step.prompt {
            Some(Prompt::Text(page)) => (page.has_previous, page.has_next),
            other => panic!("text page expected, got {other:?}"),
        }
    }

    #[test]
    fn roger_starts_the_quest_with_an_apple_and_a_scratch() {
        let mut script = NpcScript::for_quest("q1021s", &player(50)).unwrap();
        assert_eq!(buttons(&script.begin()), (false, true));
        assert_eq!(
            buttons(&script.respond(ScriptInput::Next).unwrap()),
            (true, true)
        );
        let ask = script.respond(ScriptInput::Next).unwrap();
        assert_eq!(kind(&ask), Some(1));
        let accepted = script.respond(ScriptInput::Yes).unwrap();
        assert_eq!(
            accepted.effects,
            [
                Effect::SetHp(25),
                Effect::GiveItem {
                    item_id: ROGERS_APPLE,
                    quantity: 1
                },
                Effect::StartQuest(ROGERS_APPLE_QUEST),
            ]
        );
        // No going back to the question.
        assert_eq!(buttons(&accepted), (false, true));
        assert_eq!(
            buttons(&script.respond(ScriptInput::Next).unwrap()),
            (true, true)
        );
        let end = script.respond(ScriptInput::Next).unwrap();
        assert_eq!(end.prompt, None);
        assert_eq!(
            end.effects,
            [Effect::AvatarEffect("UI/tutorial.img/28".into())]
        );
    }

    #[test]
    fn declining_roger_changes_nothing() {
        let mut script = NpcScript::for_quest("q1021s", &player(50)).unwrap();
        script.respond(ScriptInput::Next);
        script.respond(ScriptInput::Next);
        assert_eq!(script.respond(ScriptInput::No), Some(Step::default()));
        // Answers that don't fit the prompt end the conversation.
        let mut script = NpcScript::for_quest("q1021s", &player(50)).unwrap();
        assert_eq!(script.respond(ScriptInput::Yes), None);
    }

    #[test]
    fn roger_says_so_when_the_apple_does_not_fit() {
        let mut script = NpcScript::for_quest("q1021s", &player(50)).unwrap();
        script.respond(ScriptInput::Next);
        script.respond(ScriptInput::Next);
        script.respond(ScriptInput::Yes);
        let full = script.inventory_full().unwrap();
        assert!(full.effects.is_empty());
        assert_eq!(buttons(&full), (false, false));
        assert_eq!(script.respond(ScriptInput::Next), Some(Step::default()));
    }

    #[test]
    fn roger_pays_only_after_full_recovery() {
        let mut script = NpcScript::for_quest("q1021e", &player(25)).unwrap();
        let not_yet = script.begin();
        assert!(not_yet.effects.is_empty());
        assert_eq!(buttons(&not_yet), (false, false));
        assert_eq!(script.respond(ScriptInput::Next), Some(Step::default()));

        let mut script = NpcScript::for_quest("q1021e", &player(50)).unwrap();
        script.respond(ScriptInput::Next).unwrap();
        let reward = script.respond(ScriptInput::Next).unwrap();
        assert_eq!(buttons(&reward), (true, true));
        let paid = script.respond(ScriptInput::Next).unwrap();
        assert_eq!(paid.prompt, None);
        assert!(paid.effects.contains(&Effect::GainExp(10)));
        assert!(paid
            .effects
            .contains(&Effect::CompleteQuest(ROGERS_APPLE_QUEST)));
    }

    #[test]
    fn robin_answers_any_topic_page_by_page() {
        let mut robin = NpcScript::for_npc(ROBIN, &player(50)).unwrap();
        let Some(Prompt::Selection { text, .. }) = &robin.begin().prompt else {
            panic!("menu expected");
        };
        assert_eq!(text.matches("#L").count(), 18);
        // Maple Island has four pages.
        let first = robin.respond(ScriptInput::Select(5)).unwrap();
        assert_eq!(buttons(&first), (false, true));
        for _ in 0..2 {
            robin.respond(ScriptInput::Next).unwrap();
        }
        assert_eq!(
            buttons(&robin.respond(ScriptInput::Next).unwrap()),
            (true, false)
        );
        assert_eq!(robin.respond(ScriptInput::Next), Some(Step::default()));
        let mut robin = NpcScript::for_npc(ROBIN, &player(50)).unwrap();
        assert_eq!(robin.respond(ScriptInput::Select(18)), None);
    }

    #[test]
    fn the_administrator_warps_to_lith_harbor_on_yes() {
        let mut admin = NpcScript::for_npc(MAPLE_ADMINISTRATOR, &player(50)).unwrap();
        assert_eq!(kind(&admin.begin()), Some(1));
        assert_eq!(
            admin.respond(ScriptInput::Yes).unwrap(),
            Step {
                prompt: None,
                effects: vec![Effect::Warp(LITH_HARBOR)]
            }
        );
        let mut admin = NpcScript::for_npc(MAPLE_ADMINISTRATOR, &player(50)).unwrap();
        assert_eq!(
            buttons(&admin.respond(ScriptInput::No).unwrap()),
            (false, false)
        );
        assert_eq!(admin.respond(ScriptInput::Next), Some(Step::default()));
    }

    /// Shanks's pages after yes, and what reading past the last one does.
    fn sail(player: &Player) -> (Vec<(bool, bool)>, Vec<Effect>) {
        let mut shanks = NpcScript::for_npc(SHANKS, player).unwrap();
        assert_eq!(kind(&shanks.begin()), Some(1));
        let mut step = shanks.respond(ScriptInput::Yes).unwrap();
        assert!(step.effects.is_empty());
        let mut pages = Vec::new();
        while step.prompt.is_some() {
            pages.push(buttons(&step));
            step = shanks.respond(ScriptInput::Next).unwrap();
        }
        (pages, step.effects)
    }

    fn traveller(level: u8, meso: i32, letter: bool) -> Player {
        let mut player = player(50);
        player.stats.level = level;
        player.meso = meso;
        if letter {
            player.inventory.insert(
                game::InventoryType::Etc,
                1,
                game::Item::stack(RECOMMENDATION_LETTER, 1),
            );
        }
        player
    }

    #[test]
    fn shanks_sails_for_the_fare_from_level_7() {
        let warp = Effect::Warp(LITH_HARBOR);
        assert_eq!(
            sail(&traveller(7, 150, false)),
            (
                vec![(false, true), (true, true)],
                vec![Effect::SpendMeso(SHIP_FARE), warp.clone()]
            )
        );
        // Too weak or too poor: he says so and stays.
        assert_eq!(
            sail(&traveller(6, 1000, false)),
            (vec![(false, true), (true, false)], vec![])
        );
        assert_eq!(
            sail(&traveller(7, 149, false)),
            (vec![(false, true), (true, false)], vec![])
        );
        // Lucas's letter pays, at any level.
        assert_eq!(
            sail(&traveller(1, 0, true)),
            (
                vec![(false, true), (true, true)],
                vec![
                    Effect::TakeItem {
                        item_id: RECOMMENDATION_LETTER,
                        quantity: 1
                    },
                    warp
                ]
            )
        );
        let mut shanks = NpcScript::for_npc(SHANKS, &traveller(7, 150, false)).unwrap();
        assert_eq!(
            buttons(&shanks.respond(ScriptInput::No).unwrap()),
            (false, false)
        );
        assert_eq!(shanks.respond(ScriptInput::Next), Some(Step::default()));
    }

    #[test]
    fn heracle_asks_for_a_guild_name_from_level_10_with_the_fee() {
        let founder = |level, meso, guilded| {
            let mut player = crate::codec::stats::sample_player();
            player.stats.level = level;
            player.meso = meso;
            if guilded {
                player.guild = Some(guild::GuildTag {
                    id: 1,
                    name: "Testers".into(),
                    emblem: guild::Emblem::default(),
                });
            }
            NpcScript::for_npc(HERACLE, &player).unwrap()
        };
        let mut heracle = founder(10, guild::CREATE_COST, false);
        assert_eq!(kind(&heracle.begin()), Some(1));
        let yes = heracle.respond(ScriptInput::Yes).unwrap();
        assert_eq!(yes.prompt, None);
        assert_eq!(yes.effects, [Effect::OpenGuildNamePrompt]);

        // Too weak, too poor, or already in a guild: he says so.
        for mut heracle in [
            founder(9, guild::CREATE_COST, false),
            founder(10, guild::CREATE_COST - 1, false),
            founder(10, guild::CREATE_COST, true),
        ] {
            assert_eq!(buttons(&heracle.begin()), (false, false));
            assert_eq!(heracle.respond(ScriptInput::Next), Some(Step::default()));
        }
    }

    #[test]
    fn the_booths_sell_a_ticket_for_5000_mesos() {
        let buyer = traveller(10, 5000, false);
        let mut joel = NpcScript::for_npc(JOEL, &buyer).unwrap();
        assert_eq!(kind(&joel.begin()), Some(1));
        assert_eq!(
            joel.respond(ScriptInput::Yes).unwrap(),
            Step {
                prompt: None,
                effects: vec![
                    Effect::SpendMeso(5000),
                    Effect::GiveItem {
                        item_id: conti::TO_ORBIS.ticket,
                        quantity: 1
                    }
                ]
            }
        );
        let mut agatha = NpcScript::for_npc(AGATHA, &buyer).unwrap();
        agatha.respond(ScriptInput::Yes).unwrap();
        assert!(agatha.inventory_full().is_some(), "a full Etc tab is told");

        // Short of mesos: yes is answered with why not.
        let mut joel = NpcScript::for_npc(JOEL, &traveller(10, 4999, false)).unwrap();
        let short = joel.respond(ScriptInput::Yes).unwrap();
        assert!(short.effects.is_empty());
        assert_eq!(buttons(&short), (false, false));
    }

    #[test]
    fn the_usher_boards_ticket_holders_only_while_the_ship_boards() {
        const HOUR: i64 = 3_600_000;
        let minute = |m: i64| 7 * HOUR + m * 60_000;
        let mut holder = traveller(10, 0, false);
        holder.inventory.insert(
            game::InventoryType::Etc,
            1,
            game::Item::stack(conti::TO_ORBIS.ticket, 1),
        );
        let mut cherry = usher(CHERRY, &conti::TO_ORBIS, &holder, minute(12));
        assert_eq!(kind(&cherry.begin()), Some(1));
        assert_eq!(
            cherry.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::RequireBoarding(101000300),
                Effect::TakeItem {
                    item_id: conti::TO_ORBIS.ticket,
                    quantity: 1
                },
                Effect::Warp(101000301),
            ]
        );
        // No ticket, boarding closed, or the ship gone: she says so.
        for cherry in [
            usher(
                CHERRY,
                &conti::TO_ORBIS,
                &traveller(10, 0, false),
                minute(12),
            ),
            usher(CHERRY, &conti::TO_ORBIS, &holder, minute(14)),
            usher(CHERRY, &conti::TO_ORBIS, &holder, minute(5)),
        ] {
            assert_eq!(buttons(&cherry.begin()), (false, false));
        }
    }

    #[test]
    fn the_crew_let_passengers_off_and_isa_shows_the_way() {
        let mut purin = NpcScript::for_npc(PURIN, &player(50)).unwrap();
        assert_eq!(
            purin.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::Warp(101000300)]
        );
        let mut erin = NpcScript::for_npc(ERIN, &player(50)).unwrap();
        assert_eq!(
            erin.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::Warp(200000111)]
        );
        let mut isa = NpcScript::for_npc(ISA, &player(50)).unwrap();
        assert_eq!(kind(&isa.begin()), Some(4));
        assert_eq!(kind(&isa.respond(ScriptInput::Select(0)).unwrap()), Some(1));
        assert_eq!(
            isa.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::Warp(ORBIS_TUNNEL)]
        );
    }

    #[test]
    fn answers_must_fit_the_prompt() {
        let page = Prompt::Text(DialoguePage {
            npc_id: 1,
            text: String::new(),
            has_previous: false,
            has_next: true,
        });
        assert_eq!(input(&page, 0, 1, None), Some(ScriptInput::Next));
        assert_eq!(input(&page, 0, 0, None), None, "no previous page");
        assert_eq!(input(&page, 1, 1, None), None);
        let menu = Prompt::Selection {
            npc_id: 1,
            text: String::new(),
        };
        assert_eq!(input(&menu, 4, 1, Some(3)), Some(ScriptInput::Select(3)));
        assert_eq!(input(&menu, 4, -1, None), None);
    }
}
