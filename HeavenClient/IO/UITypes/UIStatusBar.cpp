//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#include "UIStatusBar.h"

#include "UIChannel.h"
#include "UIChatBar.h"
#include "UIEquipInventory.h"
#include "UIGameOption.h"
#include "UIItemInventory.h"
#include "UIKeyConfig.h"
#include "UIMemo.h"
#include "UIMessenger.h"
#include "UINotice.h"
#include "UIQuestLog.h"
#include "UIQuiz.h"
#include "UIQuit.h"
#include "UIMonsterBook.h"
#include "UISkillBook.h"
#include "UIStatsInfo.h"
#include "UISystemOption.h"
#include "UIUserList.h"

#include "../KeyConfig.h"
#include "../Messages.h"
#include "../UI.h"

#include "../Components/MapleButton.h"
#include "../Components/MenuCover.h"

#include "../../Configuration.h"
#include "../../Character/ExpTable.h"
#include "../../Character/GachaExp.h"
#include "../../Data/ItemData.h"
#include "../../Data/SkillData.h"
#include "../../Gameplay/Stage.h"
#include "../../Graphics/GraphicsGL.h"
#include "../../Net/Packets/GameplayPackets.h"
#include "../../Net/Packets/PlayerPackets.h"
#include "../../Util/NxFiles.h"

namespace ms
{
	namespace
	{
		// The icon in UIWindow.img/KeyConfig/icon for each action the client
		// knows. v83 numbers its functions differently from the client.
		const std::map<int32_t, int32_t> ACTION_ICONS = {
			{ KeyAction::Id::EQUIPMENT, 0 },
			{ KeyAction::Id::ITEMS, 1 },
			{ KeyAction::Id::STATS, 2 },
			{ KeyAction::Id::SKILLS, 3 },
			{ KeyAction::Id::FRIENDS, 4 },
			{ KeyAction::Id::WORLDMAP, 5 },
			{ KeyAction::Id::MAPLECHAT, 6 },
			{ KeyAction::Id::MINIMAP, 7 },
			{ KeyAction::Id::QUESTLOG, 8 },
			{ KeyAction::Id::KEYBINDINGS, 9 },
			{ KeyAction::Id::SAY, 10 },
			{ KeyAction::Id::WHISPER, 11 },
			{ KeyAction::Id::PARTYCHAT, 12 },
			{ KeyAction::Id::FRIENDSCHAT, 13 },
			{ KeyAction::Id::MENU, 14 },
			{ KeyAction::Id::QUICKSLOTS, 15 },
			{ KeyAction::Id::TOGGLECHAT, 16 },
			{ KeyAction::Id::GUILD, 17 },
			{ KeyAction::Id::GUILDCHAT, 18 },
			{ KeyAction::Id::PARTY, 19 },
			{ KeyAction::Id::NOTIFIER, 20 },
			{ KeyAction::Id::TOSPOUSE, 21 },
			{ KeyAction::Id::MONSTERBOOK, 22 },
			{ KeyAction::Id::CASHSHOP, 23 },
			{ KeyAction::Id::ALLIANCECHAT, 24 },
			{ KeyAction::Id::MEDALS, 27 },
			{ KeyAction::Id::PICKUP, 50 },
			{ KeyAction::Id::SIT, 51 },
			{ KeyAction::Id::ATTACK, 52 },
			{ KeyAction::Id::JUMP, 53 },
			{ KeyAction::Id::INTERACT_HARVEST, 54 },
			{ KeyAction::Id::FACE1, 100 },
			{ KeyAction::Id::FACE2, 101 },
			{ KeyAction::Id::FACE3, 102 },
			{ KeyAction::Id::FACE4, 103 },
			{ KeyAction::Id::FACE5, 104 },
			{ KeyAction::Id::FACE6, 105 },
			{ KeyAction::Id::FACE7, 106 }
		};

		// How long a key glows unless it is clicked (FUN_008de964 checks for 90000 ms).
		constexpr uint16_t KEY_HINT_TIME = 90000 / Constants::TIMESTEP;

		// Inner left edge and width of the HP, MP and EXP fills in gauge/bar.
		constexpr int16_t GAUGE_TOP = 15;
		constexpr int16_t GAUGE_HEIGHT = 14;
		constexpr int16_t HP_LEFT = 2;
		constexpr int16_t MP_LEFT = 110;
		constexpr int16_t EXP_LEFT = 223;
		constexpr int16_t HPMP_WIDTH = 105;
		constexpr int16_t EXP_WIDTH = 115;

		const Point<int16_t> GAUGE_POS = Point<int16_t>(218, 38);

		// The EXP numbers over the gauge, (442, 560) to (555, 573) on v83's
		// 800x600 screen: hovering shows the EXP and any gacha EXP, and
		// double-clicking asks to take the gacha EXP in (FUN_008d76d7,
		// FUN_008d7611).
		constexpr Rectangle<int16_t> EXP_NUMBERS = Rectangle<int16_t>(442, 555, 31, 44);
		// The job and name, (88, 559) to (170, 573): hovering shows the
		// login ID, channel and name (FUN_008d76d7, StringPool 0x7d3).
		constexpr Rectangle<int16_t> NAME_AREA = Rectangle<int16_t>(88, 170, 30, 44);

		constexpr const char *TAKE_GACHA_EXP = "Do you wish to charge the EXP \\nearned from The Writs of Solomon?";

		// The warning flashes sit over the HP and MP fills, at (218, 580) and
		// (326, 580) on v83's 800x600 screen.
		const Point<int16_t> HPFLASH_POS = Point<int16_t>(218, 51);
		const Point<int16_t> MPFLASH_POS = Point<int16_t>(326, 51);

		// The soHPFlash and soMPFlash options, in 5% steps. v83 reads them
		// from the registry, 10 (50%) unless set in the options.
		constexpr int32_t FLASH_LEVEL = 10;
		// Quick slot cells are 32 x 32 at (7, 8) + (35, 33) per column and row (0xbe2db0).
		const Point<int16_t> CELL_PITCH = Point<int16_t>(35, 33);
		constexpr int16_t CELL_SIZE = 32;

		// An open Key Config's edits are on the key map v83 draws the panel
		// from, so the panel shows them.
		UIKeyConfig *open_keyconfig()
		{
			auto keyconfig = UI::get().get_element<UIKeyConfig>();

			return keyconfig && keyconfig->is_active() ? keyconfig.get() : nullptr;
		}

		Keyboard::Mapping quickslot_mapping(int32_t key)
		{
			if (UIKeyConfig *keyconfig = open_keyconfig())
				return keyconfig->get_staged_mapping(key);

			return UI::get().get_keyboard().get_maple_mapping(key);
		}

		// Bind a key to a mapping and tell the server. As in Key Config, the
		// mapping leaves any other key it was on. With Key Config open it is
		// one of the window's edits, sent on its OK or taken back on Cancel
		// (0x87 goes only when Key Config is shut, DAT_00bed650).
		void bind_key(int32_t key, Keyboard::Mapping mapping)
		{
			if (UIKeyConfig *keyconfig = open_keyconfig())
			{
				keyconfig->stage_key(key, mapping);
				return;
			}

			Keyboard &keyboard = UI::get().get_keyboard();
			std::vector<std::tuple<KeyConfig::Key, KeyType::Id, int32_t>> changes;

			for (auto const &[other, bound] : keyboard.get_maplekeys())
				if (bound == mapping && other != key)
					changes.emplace_back(KeyConfig::actionbyid(other), KeyType::Id::NONE, KeyAction::Id::LENGTH);

			if (keyboard.get_maple_mapping(key) != mapping)
				changes.emplace_back(KeyConfig::actionbyid(key), mapping.type, mapping.action);

			if (changes.empty())
				return;

			ChangeKeyMapPacket(changes).dispatch();

			for (auto const &[changed, type, action] : changes)
			{
				if (type == KeyType::Id::NONE)
					keyboard.remove(changed);
				else
					keyboard.assign(changed, type, action);
			}
		}
	}

	UIStatusBar::UIStatusBar(const CharStats &st) : stats(st)
	{
		VWIDTH = Constants::Constants::get().get_viewwidth();
		VHEIGHT = Constants::Constants::get().get_viewheight();

		right = VWIDTH - 800;
		quickslot_active = true;
		menu = Menu::NONE;

		nl::node src = NxFiles::ui_classic["StatusBar.img"];
		nl::node base = src["base"];
		nl::node gauge = src["gauge"];
		nl::node window = NxFiles::ui_classic["UIWindow.img"];
		nl::node basic = NxFiles::ui_classic["Basic.img"];

		backgrnd = base["backgrnd"];
		backgrnd2 = base["backgrnd2"];
		gaugebar = gauge["bar"];
		gaugegray = gauge["gray"];
		graduation = gauge["graduation"];
		hpflash.animation = gauge["hpFlash"];
		hpflash.pos = HPFLASH_POS;
		mpflash.animation = gauge["mpFlash"];
		mpflash.pos = MPFLASH_POS;
		quickslot = base["quickSlot"];

		// A cell's label follows its key (FUN_008ddb30).
		for (nl::node label : window["KeyConfig"]["quickslotConfig"]["key"])
			keylabels[std::stoi(label.name())] = label;

		for (auto &iter : ACTION_ICONS)
			action_icons[iter.first] = window["KeyConfig"]["icon"][std::to_string(iter.second)];

		for (size_t i = 0; i < SkillMacros::MAX_MACROS; i++)
			macro_icons[i] = window["SkillMacro"]["Macroicon"][std::to_string(i)]["icon"];

		const char *hint_keys[Key::NUM_KEYS] = { "EquipKey", "InvenKey", "StatKey", "SkillKey" };

		for (size_t i = 0; i < Key::NUM_KEYS; i++)
			key_hints[i] = src[hint_keys[i]]["ani"];

		sprites.emplace_back(base["box"], Point<int16_t>(573 + right, 8));
		sprites.emplace_back(base["iconMemo"], Point<int16_t>(599 + right, 12));

		number = Charset(src["number"], Charset::Alignment::LEFT);
		levelnumber = Charset(basic["LevelNo"], Charset::Alignment::LEFT);
		itemnumber = Charset(basic["ItemNo"], Charset::Alignment::LEFT);

		joblabel = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE);
		namelabel = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE);

		buttons[Buttons::BT_CLAIM] = std::make_unique<MapleButton>(src["BtClaim"], Point<int16_t>(573 + right, 8));
		buttons[Buttons::BT_EQUIP] = std::make_unique<MapleButton>(src["EquipKey"], Point<int16_t>(618 + right, 8));
		buttons[Buttons::BT_INVEN] = std::make_unique<MapleButton>(src["InvenKey"], Point<int16_t>(648 + right, 8));
		buttons[Buttons::BT_STAT] = std::make_unique<MapleButton>(src["StatKey"], Point<int16_t>(678 + right, 8));
		buttons[Buttons::BT_SKILL] = std::make_unique<MapleButton>(src["SkillKey"], Point<int16_t>(708 + right, 8));
		buttons[Buttons::BT_KEYSET] = std::make_unique<MapleButton>(src["KeySet"], Point<int16_t>(738 + right, 8));
		buttons[Buttons::BT_QUICKSLOT_SHOW] = std::make_unique<MapleButton>(src["QuickSlot"],
																			Point<int16_t>(768 + right, 8));
		buttons[Buttons::BT_QUICKSLOT_HIDE] = std::make_unique<MapleButton>(src["QuickSlotD"],
																			Point<int16_t>(768 + right, 8));
		buttons[Buttons::BT_CASHSHOP] = std::make_unique<MapleButton>(src["BtShop"], Point<int16_t>(573 + right, 36));
		buttons[Buttons::BT_TRADE] = std::make_unique<MapleButton>(src["BtNPT"], Point<int16_t>(629 + right, 36));
		buttons[Buttons::BT_MENU] = std::make_unique<MapleButton>(src["BtMenu"], Point<int16_t>(685 + right, 36));
		buttons[Buttons::BT_SHORTCUT] = std::make_unique<MapleButton>(src["BtShort"], Point<int16_t>(741 + right, 36));

		buttons[Buttons::BT_QUICKSLOT_SHOW]->set_active(!quickslot_active);
		buttons[Buttons::BT_QUICKSLOT_HIDE]->set_active(quickslot_active);

		// v83 turns Menu's help off and attaches the trade button's to the
		// Cash Shop's by mistake, so only these have any (FUN_008d01b2).
		set_button_help(Buttons::BT_CASHSHOP, "Game/Button/Shop");
		set_button_help(Buttons::BT_KEYSET, "Game/Button/KeySet");
		set_button_help(Buttons::BT_QUICKSLOT_SHOW, "Game/Button/QuickSlot");
		set_button_help(Buttons::BT_QUICKSLOT_HIDE, "Game/Button/QuickSlotD");
		set_button_help(Buttons::BT_CLAIM, "Game/Button/Claim");

		quickslot_pos = Point<int16_t>(647 + right, QUICKSLOT_TOP);

		// Each menu's pointer sits over the button that opens it, as far as
		// the screen allows.
		nl::node game = window["GameMenu"];
		nl::node shortcut = window["ShortCut"];

		gamemenu = game["backgrnd"];
		shortcutmenu = shortcut["backgrnd"];

		gamemenu_pos = Point<int16_t>(666 + right, 36 - gamemenu.height());
		shortcutmenu_pos = Point<int16_t>(
				std::min<int16_t>(723, 800 - shortcutmenu.width()) + right,
				36 - shortcutmenu.height()
		);

		const char *game_buttons[] = { "BtChannel", "BtGameOpt", "BtSysOpt", "BtQuit" };

		for (uint16_t i = 0; i < 4; i++)
			buttons[Buttons::BT_GAME_CHANNEL + i] = std::make_unique<MapleButton>(
					game[game_buttons[i]],
					gamemenu_pos + Point<int16_t>(6, 25 + 25 * i)
			);

		const char *shortcut_buttons[] = {
			"BtEquip", "BtItem", "BtStat", "BtSkill", "BtQuest", "BtComm", "BtMessenger", "BtMobbook"
		};

		for (uint16_t i = 0; i < 8; i++)
			buttons[Buttons::BT_SHORTCUT_EQUIP + i] = std::make_unique<MapleButton>(
					shortcut[shortcut_buttons[i]],
					shortcutmenu_pos + Point<int16_t>(5, 23 + 26 * i)
			);

		set_menu(Menu::NONE);

		position = Point<int16_t>(0, VHEIGHT - HEIGHT);
		dimension = Point<int16_t>(VWIDTH, HEIGHT);
	}

	void UIStatusBar::draw(float alpha) const
	{
		// The chat strip is cut out of both backgrounds so that UIChatBar can
		// draw it whichever of the two is in front.
		int16_t strip_right = CHAT_STRIP.right();
		int16_t strip_top = CHAT_STRIP.top();
		int16_t strip_bottom = CHAT_STRIP.bottom();

		backgrnd.draw(
				DrawArgument(position, Point<int16_t>(VWIDTH, strip_top)),
				Rectangle<int16_t>(0, 800, 0, strip_top)
		);
		backgrnd.draw(
				DrawArgument(position, Point<int16_t>(VWIDTH - strip_right, strip_bottom - strip_top)),
				Rectangle<int16_t>(strip_right, 800, strip_top, strip_bottom)
		);
		backgrnd.draw(
				DrawArgument(position, Point<int16_t>(VWIDTH, HEIGHT - strip_bottom)),
				Rectangle<int16_t>(0, 800, strip_bottom, HEIGHT)
		);
		// v83 copies backgrnd2 two pixels in from the left (0x8d3ff2).
		Point<int16_t> backgrnd2_at = position + Point<int16_t>(2, 0);
		backgrnd2.draw(backgrnd2_at, Rectangle<int16_t>(0, backgrnd2.width(), 0, strip_top));
		backgrnd2.draw(backgrnd2_at, Rectangle<int16_t>(0, backgrnd2.width(), strip_bottom, HEIGHT));

		UIElement::draw_sprites(alpha);

		gaugebar.draw(position + GAUGE_POS);
		draw_gauge(HP_LEFT, HPMP_WIDTH, gethppercent());
		draw_gauge(MP_LEFT, HPMP_WIDTH, getmppercent());
		draw_gauge(EXP_LEFT, EXP_WIDTH, getexppercent());
		graduation.draw(position + GAUGE_POS);
		hpflash.draw(position);
		mpflash.draw(position);

		int16_t level = stats.get_stat(MapleStat::Id::LEVEL);
		int16_t hp = stats.get_stat(MapleStat::Id::HP);
		int16_t mp = stats.get_stat(MapleStat::Id::MP);
		int32_t maxhp = stats.get_total(EquipStat::Id::HP);
		int32_t maxmp = stats.get_total(EquipStat::Id::MP);
		int64_t exp = stats.get_exp();

		std::string expstring = std::to_string(100 * getexppercent());

		draw_number("[" + std::to_string(hp) + "/" + std::to_string(maxhp) + "]", Point<int16_t>(237, 41));
		draw_number("[" + std::to_string(mp) + "/" + std::to_string(maxmp) + "]", Point<int16_t>(349, 41));
		draw_number(
				std::to_string(exp) + "[" + expstring.substr(0, expstring.find('.') + 3) + "%]",
				Point<int16_t>(466, 41)
		);

		std::string levelstring = std::to_string(level);
		int16_t level_x = 56 - 6 * static_cast<int16_t>(levelstring.size());

		levelnumber.draw(levelstring, 12, position + Point<int16_t>(level_x, 47));

		joblabel.draw(position + Point<int16_t>(87, 32));
		namelabel.draw(position + Point<int16_t>(87, 46));

		if (quickslot_active)
			draw_quickslots();

		// An open menu covers the icon row and quick slots, so its frame goes
		// between the status bar's buttons and its own.
		for (auto &iter : buttons)
			if (iter.first < Buttons::BT_GAME_CHANNEL && iter.second)
				iter.second->draw(position);

		for (size_t i = 0; i < Key::NUM_KEYS; i++)
			if (key_hint_left[i] > 0)
				key_hints[i].draw(position + Point<int16_t>(618 + right + 30 * static_cast<int16_t>(i), 8), alpha);

		if (menu == Menu::GAME)
			gamemenu.draw(position + gamemenu_pos);
		else if (menu == Menu::SHORTCUT)
			shortcutmenu.draw(position + shortcutmenu_pos);

		for (auto &iter : buttons)
			if (iter.first >= Buttons::BT_GAME_CHANNEL && iter.second)
				iter.second->draw(position);
	}

	void UIStatusBar::draw_gauge(int16_t left, int16_t width, float percent) const
	{
		percent = std::max(0.0f, std::min(1.0f, percent));

		auto filled = static_cast<int16_t>(width * percent);

		if (filled >= width)
			return;

		gaugegray.draw(
				DrawArgument(
						position + GAUGE_POS + Point<int16_t>(left + filled, GAUGE_TOP),
						Point<int16_t>(width - filled, GAUGE_HEIGHT)
				)
		);
	}

	void UIStatusBar::draw_number(const std::string &text, Point<int16_t> pos) const
	{
		// StatusBar.img/number names its symbols Lbracket, Rbracket, slash and
		// percent; the brackets are two pixels taller than the digits. It has
		// no decimal point, which footage shows as one pixel on the baseline.
		int16_t shift = 0;

		for (char c : text)
		{
			if (c == '.')
			{
				Point<int16_t> dot = position + pos + Point<int16_t>(shift + 1, 7);

				GraphicsGL::get().drawrectangle(dot.x(), dot.y(), 1, 1, 1.0f, 1.0f, 1.0f, 1.0f);
				shift += 3;

				continue;
			}

			int8_t glyph = c;
			int16_t top = 1;

			switch (c)
			{
				case '[':
					glyph = 'L';
					top = 0;
					break;
				case ']':
					glyph = 'R';
					top = 0;
					break;
				case '/':
					glyph = 's';
					break;
				case '%':
					glyph = 'p';
					break;
				default:
					break;
			}

			number.draw(glyph, position + pos + Point<int16_t>(shift, top));
			shift += number.getw(glyph) + 1;
		}
	}

	void UIStatusBar::draw_quickslots() const
	{
		Point<int16_t> panel = position + quickslot_pos;

		quickslot.draw(panel);

		const QuickSlots &quick_slots = UI::get().get_quick_slots();
		const Inventory &inventory = Stage::get().get_player().get_inventory();

		for (int16_t i = 0; i < 8; i++)
		{
			int16_t col = i % 4;
			int16_t row = i / 4;
			Point<int16_t> cellpos = panel + Point<int16_t>(7 + col * CELL_PITCH.x(), 8 + row * CELL_PITCH.y());

			Keyboard::Mapping mapping = quickslot_mapping(quick_slots.key(i));

			// Icons hang from their bottom-left corner.
			Point<int16_t> iconpos = cellpos + Point<int16_t>(0, 32);

			switch (mapping.type)
			{
				case KeyType::Id::SKILL:
					SkillData::get(mapping.action).get_icon(SkillData::Icon::NORMAL).draw(iconpos);
					break;
				case KeyType::Id::ITEM:
				case KeyType::Id::CASH:
				{
					ItemData::get(mapping.action).get_icon(false).draw(iconpos);

					int16_t count = inventory.get_total_item_count(mapping.action);

					itemnumber.draw(std::to_string(count), cellpos + Point<int16_t>(0, 20));
					break;
				}
				case KeyType::Id::MACRO:
					// v83 FUN_008d66be.
					if (mapping.action >= 0 && mapping.action < static_cast<int32_t>(SkillMacros::MAX_MACROS))
						macro_icons[mapping.action].draw(iconpos);

					break;
				case KeyType::Id::MENU:
				case KeyType::Id::ACTION:
				case KeyType::Id::FACE:
				{
					auto icon = action_icons.find(mapping.action);

					if (icon != action_icons.end())
						icon->second.draw(iconpos);

					break;
				}
				default:
					break;
			}

			// Right Shift keeps its own label.
			auto label = keylabels.find(quick_slots.keys()[i]);

			if (label != keylabels.end())
				label->second.draw(panel + Point<int16_t>(9 + col * CELL_PITCH.x(), 10 + row * CELL_PITCH.y()));
		}
	}

	void UIStatusBar::update()
	{
		UIElement::update();

		for (size_t i = 0; i < Key::NUM_KEYS; i++)
		{
			if (key_hint_left[i] > 0)
			{
				key_hints[i].update();
				key_hint_left[i]--;
			}
		}
		hpflash.check(stats.get_stat(MapleStat::Id::HP), stats.get_total(EquipStat::Id::HP));
		mpflash.check(stats.get_stat(MapleStat::Id::MP), stats.get_total(EquipStat::Id::MP));
		hpflash.update();
		mpflash.update();

		joblabel.change_text(stats.get_jobname());
		namelabel.change_text(stats.get_name());
	}

	void UIStatusBar::show_key_hint(Key key)
	{
		if (key_hint_left[key] > 0)
			return;

		key_hints[key].reset();
		key_hint_left[key] = KEY_HINT_TIME;
	}

	Button::State UIStatusBar::button_pressed(uint16_t id)
	{
		Player &player = Stage::get().get_player();

		// Clicking a glowing key puts its glow out.
		if (id >= Buttons::BT_EQUIP && id <= Buttons::BT_SKILL)
			key_hint_left[id - Buttons::BT_EQUIP] = 0;

		switch (id)
		{
			case Buttons::BT_CLAIM:
			case Buttons::BT_TRADE:
				break;
			case Buttons::BT_EQUIP:
			case Buttons::BT_SHORTCUT_EQUIP:
				UI::get().emplace<UIEquipInventory>(player.get_inventory());
				break;
			case Buttons::BT_INVEN:
			case Buttons::BT_SHORTCUT_ITEM:
				UI::get().emplace<UIItemInventory>(player.get_inventory());
				break;
			case Buttons::BT_STAT:
			case Buttons::BT_SHORTCUT_STAT:
				UI::get().emplace<UIStatsInfo>(player.get_stats());
				break;
			case Buttons::BT_SKILL:
			case Buttons::BT_SHORTCUT_SKILL:
				UI::get().emplace<UISkillBook>(player.get_stats(), player.get_skills());
				break;
			case Buttons::BT_KEYSET:
				UI::get().emplace<UIKeyConfig>(player.get_inventory(), player.get_skills());
				break;
			case Buttons::BT_QUICKSLOT_SHOW:
			case Buttons::BT_QUICKSLOT_HIDE:
				toggle_qs();
				return Button::State::NORMAL;
			case Buttons::BT_CASHSHOP:
				Stage::get().enter_cash_shop();
				break;
			case Buttons::BT_MENU:
				toggle_menu(Menu::GAME);
				return Button::State::NORMAL;
			case Buttons::BT_SHORTCUT:
				toggle_menu(Menu::SHORTCUT);
				return Button::State::NORMAL;
			case Buttons::BT_GAME_CHANNEL:
				// v83 FUN_00a06735: not during a quiz.
				if (taking_quiz())
				{
					if (auto chatbar = UI::get().get_element<UIChatBar>())
						chatbar->send_chatline("You can't do this while taking the quiz.", UIChatBar::LineType::SKILL);

					break;
				}

				UI::get().emplace<UIChannel>();
				break;
			case Buttons::BT_GAME_OPTION:
				UI::get().emplace<UIGameOption>();
				break;
			case Buttons::BT_GAME_SYSTEM:
				UI::get().emplace<UISystemOption>();
				break;
			case Buttons::BT_GAME_QUIT:
				ask_to_quit();
				break;
			case Buttons::BT_SHORTCUT_QUEST:
				UI::get().emplace<UIQuestLog>(player.get_quests());
				break;
			case Buttons::BT_SHORTCUT_COMMUNITY:
			{
				auto userlist = UI::get().get_element<UIUserList>();
				auto tab = UIUserList::Tab::FRIEND;

				if (!userlist)
				{
					UI::get().emplace<UIUserList>(tab);
				} else
				{
					if (userlist->is_active() && userlist->get_tab() == tab)
						userlist->deactivate();
					else
						userlist->makeactive();

					userlist->change_tab(tab);
				}

				break;
			}
			case Buttons::BT_SHORTCUT_MESSENGER:
				UIMessenger::toggle();
				break;
			case Buttons::BT_SHORTCUT_MONSTERBOOK:
				UI::get().emplace<UIMonsterBook>(player.get_monsterbook());
				break;
		}

		remove_menus();

		return Button::State::NORMAL;
	}

	void UIStatusBar::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
		{
			if (menu == Menu::NONE)
				toggle_menu(Menu::GAME);
			else
				remove_menus();
		} else if (keycode == KeyAction::Id::RETURN)
		{
			for (auto &iter : buttons)
				if (iter.first >= Buttons::BT_GAME_CHANNEL && iter.second->is_active()
					&& iter.second->get_state() == Button::State::MOUSEOVER)
				{
					button_pressed(iter.first);
					break;
				}
		} else if (menu != Menu::NONE && (keycode == KeyAction::Id::UP || keycode == KeyAction::Id::DOWN))
		{
			uint16_t min_id = menu == Menu::GAME ? Buttons::BT_GAME_CHANNEL : Buttons::BT_SHORTCUT_EQUIP;
			uint16_t max_id = menu == Menu::GAME ? Buttons::BT_GAME_QUIT : Buttons::BT_SHORTCUT_MONSTERBOOK;
			uint16_t id = min_id;
			bool found = false;

			for (uint16_t i = min_id; i <= max_id; i++)
			{
				if (buttons[i]->get_state() != Button::State::NORMAL)
				{
					id = i;
					found = true;

					buttons[i]->set_state(Button::State::NORMAL);
					break;
				}
			}

			if (found && keycode == KeyAction::Id::DOWN)
				id = id < max_id ? id + 1 : min_id;
			else if (found && keycode == KeyAction::Id::UP)
				id = id > min_id ? id - 1 : max_id;
			else if (!found && keycode == KeyAction::Id::UP)
				id = max_id;

			buttons[id]->set_state(Button::State::MOUSEOVER);
		}
	}

	// Dropping a skill, item or macro on a quick slot binds the slot's key to
	// it.
	bool UIStatusBar::send_icon(const Icon &icon, Point<int16_t> cursorpos)
	{
		int16_t slot = quickslot_at(cursorpos);
		Keyboard::Mapping mapping = icon.get_mapping();

		if (slot >= 0 && mapping.type != KeyType::Id::NONE)
		{
			bind_key(UI::get().get_quick_slots().key(slot), mapping);
			Sound(Sound::Name::DRAGEND).play();
		}

		return true;
	}

	int16_t UIStatusBar::quickslot_at(Point<int16_t> cursorpos) const
	{
		if (!quickslot_active)
			return -1;

		Point<int16_t> panel = position + quickslot_pos;

		for (int16_t i = 0; i < 8; i++)
		{
			int16_t col = i % 4;
			int16_t row = i / 4;
			Point<int16_t> cellpos = panel + Point<int16_t>(7 + col * CELL_PITCH.x(), 8 + row * CELL_PITCH.y());

			if (Rectangle<int16_t>(cellpos, cellpos + Point<int16_t>(CELL_SIZE, CELL_SIZE)).contains(cursorpos))
				return i;
		}

		return -1;
	}

	bool UIStatusBar::is_in_range(Point<int16_t> cursorpos) const
	{
		Point<int16_t> pos = cursorpos - position;

		if (menu != Menu::NONE && get_menu_bounds().contains(pos))
			return true;

		if (quickslot_active
			&& Rectangle<int16_t>(quickslot_pos, quickslot_pos + quickslot.get_dimensions()).contains(pos))
			return true;

		if (CHAT_STRIP.contains(pos))
			return false;

		return Rectangle<int16_t>(Point<int16_t>(), dimension).contains(pos);
	}

	Cursor::State UIStatusBar::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		// Both go in the one-line box (0x8e6e7d) at the cursor + (20, 20).
		if (EXP_NUMBERS.contains(cursorpos - position))
		{
			int16_t level = stats.get_stat(MapleStat::Id::LEVEL);
			int64_t need = level < ExpTable::LEVELCAP ? ExpTable::values[level] : 0;

			UI::get().show_line(Tooltip::Parent::STATUSBAR,
					gacha_exp::tooltip(stats.get_exp(), need, stats.get_gachaexp()));
		} else if (NAME_AREA.contains(cursorpos - position))
		{
			Configuration &config = Configuration::get();
			std::string channel = config.get_channel_name(config.get_worldid(), config.get_channelid());

			UI::get().show_line(Tooltip::Parent::STATUSBAR,
					config.get_login_id() + " / " + channel + " / " + stats.get_name());
		} else
		{
			UI::get().clear_tooltip(Tooltip::Parent::STATUSBAR);
		}

		// v83 FUN_008d795a: a press or the cursor off the cells takes a slot's tooltip down.
		if (quickslot_active)
		{
			int16_t slot = quickslot_at(cursorpos);

			if (clicked || slot < 0)
				UI::get().clear_tooltip(Tooltip::Parent::QUICKSLOT);
			else
				show_quickslot(slot);
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	// The slot's skill with its levels, its item as Item.nx has it (no count or flags), or its macro. Other slots
	// leave the last tooltip up, as v83 does.
	void UIStatusBar::show_quickslot(int16_t slot) const
	{
		Keyboard::Mapping mapping = quickslot_mapping(UI::get().get_quick_slots().key(slot));

		switch (mapping.type)
		{
			case KeyType::Id::SKILL:
			{
				const SkillBook &skills = Stage::get().get_player().get_skills();
				int32_t level = skills.get_level(mapping.action);

				if (level > 0)
					UI::get().show_skill(Tooltip::Parent::QUICKSLOT, mapping.action, level,
										 skills.get_masterlevel(mapping.action), skills.get_expiration(mapping.action));

				break;
			}
			case KeyType::Id::ITEM:
				UI::get().show_item(Tooltip::Parent::QUICKSLOT, mapping.action);
				break;
			case KeyType::Id::MACRO:
				if (mapping.action >= 0 && mapping.action < static_cast<int32_t>(SkillMacros::MAX_MACROS))
					UI::get().show_macro(Tooltip::Parent::QUICKSLOT, mapping.action);

				break;
			default:
				break;
		}
	}

	void UIStatusBar::remove_cursor()
	{
		UI::get().clear_tooltip(Tooltip::Parent::STATUSBAR);
		UI::get().clear_tooltip(Tooltip::Parent::QUICKSLOT);

		UIElement::remove_cursor();
	}

	void UIStatusBar::doubleclick(Point<int16_t> cursorpos)
	{
		// The note icon opens the notes waiting, as v83's 0x8d74ff does
		// over the box's right part, EquipKey's left edge included.
		if (Rectangle<int16_t>(610 + right, 642 + right, 10, 24).contains(cursorpos - position))
		{
			UIMemo::open();

			return;
		}

		if (!EXP_NUMBERS.contains(cursorpos - position)
			|| !gacha_exp::may_take(stats.get_stat(MapleStat::Id::LEVEL), stats.get_gachaexp()))
			return;

		// FUN_00a1288f checks riding and the level again once answered.
		UI::get().emplace<UIYesNo>(TAKE_GACHA_EXP, [](bool yes)
		{
			if (!yes)
				return;

			Player &player = Stage::get().get_player();

			if (player.ride_refuses(true))
				return;

			if (player.get_level() > gacha_exp::MAX_LEVEL)
				InChatMessage(Messages::ITEM_LEVEL_HIGH).drop();
			else
				player.request(GachaExpPacket(), ExclRequest::WAIT);
		});
	}

	bool UIStatusBar::button_covered(uint16_t buttonid, Point<int16_t> cursorpos) const
	{
		return menu_cover::covered(
				menu != Menu::NONE,
				get_menu_bounds(),
				buttonid >= Buttons::BT_GAME_CHANNEL,
				cursorpos - position
		);
	}

	UIElement::Type UIStatusBar::get_type() const
	{
		return TYPE;
	}

	void UIStatusBar::toggle_qs()
	{
		quickslot_active = !quickslot_active;

		buttons[Buttons::BT_QUICKSLOT_SHOW]->set_active(!quickslot_active);
		buttons[Buttons::BT_QUICKSLOT_HIDE]->set_active(quickslot_active);
	}

	void UIStatusBar::toggle_menu()
	{
		toggle_menu(Menu::SHORTCUT);
	}

	void UIStatusBar::toggle_menu(Menu next)
	{
		if (menu == next)
		{
			set_menu(Menu::NONE);
		} else
		{
			set_menu(next);

			Sound(Sound::Name::DLGNOTICE).play();
		}
	}

	void UIStatusBar::set_menu(Menu next)
	{
		menu = next;

		for (uint16_t i = Buttons::BT_GAME_CHANNEL; i <= Buttons::BT_GAME_QUIT; i++)
		{
			buttons[i]->set_active(menu == Menu::GAME);
			buttons[i]->set_state(Button::State::NORMAL);
		}

		for (uint16_t i = Buttons::BT_SHORTCUT_EQUIP; i <= Buttons::BT_SHORTCUT_MONSTERBOOK; i++)
		{
			buttons[i]->set_active(menu == Menu::SHORTCUT);
			buttons[i]->set_state(Button::State::NORMAL);
		}
	}

	void UIStatusBar::remove_menus()
	{
		set_menu(Menu::NONE);
	}

	bool UIStatusBar::is_menu_active()
	{
		return menu != Menu::NONE;
	}

	Rectangle<int16_t> UIStatusBar::get_menu_bounds() const
	{
		if (menu == Menu::GAME)
			return Rectangle<int16_t>(gamemenu_pos, gamemenu_pos + gamemenu.get_dimensions());

		return Rectangle<int16_t>(shortcutmenu_pos, shortcutmenu_pos + shortcutmenu.get_dimensions());
	}

	float UIStatusBar::getexppercent() const
	{
		int16_t level = stats.get_stat(MapleStat::Id::LEVEL);

		if (level >= ExpTable::LEVELCAP)
			return 0.0f;

		int64_t exp = stats.get_exp();

		return static_cast<float>(
				static_cast<double>(exp) / ExpTable::values[level]
		);
	}

	void UIStatusBar::GaugeFlash::check(int32_t value, int32_t max)
	{
		int32_t level = 5 * FLASH_LEVEL;

		if (level != 0 && max > 0 && value * 100 / max < level)
		{
			if (last > value)
			{
				animation.reset();
				active = true;
			}

			last = value;
		} else
		{
			// Falling under the level counts as a fall.
			last = level * max / 100;
		}
	}

	void UIStatusBar::GaugeFlash::update()
	{
		if (active && animation.update())
			active = false;
	}

	void UIStatusBar::GaugeFlash::draw(Point<int16_t> parent) const
	{
		if (active)
			animation.draw(parent + pos, 1.0f);
	}

	float UIStatusBar::gethppercent() const
	{
		int16_t hp = stats.get_stat(MapleStat::Id::HP);
		int32_t maxhp = stats.get_total(EquipStat::Id::HP);

		return static_cast<float>(hp) / maxhp;
	}

	float UIStatusBar::getmppercent() const
	{
		int16_t mp = stats.get_stat(MapleStat::Id::MP);
		int32_t maxmp = stats.get_total(EquipStat::Id::MP);

		return static_cast<float>(mp) / maxmp;
	}
}
