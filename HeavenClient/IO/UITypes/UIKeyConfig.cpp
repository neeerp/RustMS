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
#include "UIKeyConfig.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Data/ItemData.h"
#include "../../Data/SkillData.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/PlayerPackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The window, measured from the background and period footage.
		constexpr Point<int16_t> CLOSE_POS = Point<int16_t>(604, 6);
		constexpr Point<int16_t> DEFAULT_POS = Point<int16_t>(10, 237);
		constexpr Point<int16_t> DELETE_POS = Point<int16_t>(74, 237);
		constexpr Point<int16_t> QUICKSLOT_POS = Point<int16_t>(155, 237);
		constexpr Point<int16_t> OK_POS = Point<int16_t>(519, 237);
		constexpr Point<int16_t> CANCEL_POS = Point<int16_t>(569, 237);
		// The notices sit over the keyboard, with their own buttons on
		// the grey strip at their bottom.
		constexpr Point<int16_t> NOTICE_POS = Point<int16_t>(181, 76);
		constexpr Point<int16_t> NOTICE_OK_POS = Point<int16_t>(160, 59);
		constexpr Point<int16_t> NOTICE_CANCEL_POS = Point<int16_t>(211, 59);
		// The palette of unbound functions under the keyboard.
		constexpr Point<int16_t> PALETTE = Point<int16_t>(8, 266);
		constexpr int16_t PALETTE_COLUMNS = 18;
		constexpr int16_t PALETTE_ROWS = 3;
		constexpr int16_t SLOT = 34;
		constexpr int16_t ICON_SIZE = 32;
		// A key's icon sits just inside its face, under the key's label.
		constexpr Point<int16_t> FACE_ICON = Point<int16_t>(-2, -2);
		constexpr Point<int16_t> FACE_LABEL = Point<int16_t>(1, 2);

		// A bindable key: its face's top-left and width, and the label
		// bitmap drawn over it.
		struct KeyFace
		{
			int16_t x;
			int16_t y;
			int16_t width;
			int32_t label;
		};

		const std::map<int32_t, KeyFace>& key_faces()
		{
			static const std::map<int32_t, KeyFace> faces = []()
			{
				constexpr int16_t R0 = 29, R1 = 68, R2 = 101, R3 = 134, R4 = 167, R5 = 200;
				constexpr int16_t F_KEYS[] = { 83, 117, 151, 185, 227, 261, 295, 329, 371, 405, 439, 473 };

				std::map<int32_t, KeyFace> f;

				auto key = [&](int32_t code, int16_t x, int16_t y, int16_t width = 28, int32_t label = -1)
				{
					f[code] = { x, y, width, label < 0 ? code : label };
				};

				for (int16_t i = 0; i < 10; i++)
					key(KeyConfig::Key::F1 + i, F_KEYS[i], R0);

				key(KeyConfig::Key::F11, F_KEYS[10], R0);
				key(KeyConfig::Key::F12, F_KEYS[11], R0);

				key(KeyConfig::Key::GRAVE_ACCENT, 15, R1);

				for (int16_t i = 0; i < 12; i++)
					key(KeyConfig::Key::NUM1 + i, 49 + 34 * i, R1);

				key(KeyConfig::Key::INSERT, 515, R1);
				key(KeyConfig::Key::HOME, 549, R1);
				key(KeyConfig::Key::PAGE_UP, 583, R1);

				for (int16_t i = 0; i < 12; i++)
					key(KeyConfig::Key::Q + i, 65 + 34 * i, R2);

				key(KeyConfig::Key::BACKSLASH, 473, R2);
				key(KeyConfig::Key::DELETE, 515, R2);
				key(KeyConfig::Key::END, 549, R2);
				key(KeyConfig::Key::PAGE_DOWN, 583, R2);

				for (int16_t i = 0; i < 11; i++)
					key(KeyConfig::Key::A + i, 82 + 34 * i, R3);

				key(KeyConfig::Key::LEFT_SHIFT, 15, R4, 78);

				for (int16_t i = 0; i < 9; i++)
					key(KeyConfig::Key::Z + i, 99 + 34 * i, R4);

				key(KeyConfig::Key::RIGHT_SHIFT, 439, R4, 62, KeyConfig::Key::LEFT_SHIFT);
				key(KeyConfig::Key::LEFT_CONTROL, 15, R5, 43);
				key(KeyConfig::Key::LEFT_ALT, 114, R5, 47);
				key(KeyConfig::Key::SPACE, 167, R5, 164);
				key(KeyConfig::Key::RIGHT_ALT, 337, R5, 50, KeyConfig::Key::LEFT_ALT);
				key(KeyConfig::Key::RIGHT_CONTROL, 449, R5, 52, KeyConfig::Key::LEFT_CONTROL);

				return f;
			}();

			return faces;
		}

		bool is_bindable(int32_t key)
		{
			return key_faces().count(key) > 0;
		}

		Rectangle<int16_t> face_bounds(const KeyFace& face)
		{
			return Rectangle<int16_t>(
				Point<int16_t>(face.x - 2, face.y - 2),
				Point<int16_t>(face.x + face.width + 2, face.y + 30)
			);
		}

		// The functions the window offers, by the index of their v83 icon.
		// Party search and family have no counterpart here.
		struct Function
		{
			KeyAction::Id action;
			int16_t icon;
		};

		constexpr Function FUNCTIONS[] = {
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

		// Functions fill the palette in icon order: the menus from the
		// first slot, the actions from the 31st, the faces from the 37th.
		Point<int16_t> palette_slot(int16_t icon)
		{
			int16_t slot = icon < 50 ? icon : icon < 100 ? icon - 20 : icon - 64;

			return PALETTE + Point<int16_t>(slot % PALETTE_COLUMNS * SLOT, slot / PALETTE_COLUMNS * SLOT);
		}

		// The server's default bindings (game/src/keymap.rs), v83's layout.
		const std::map<int32_t, Keyboard::Mapping> DEFAULT_BINDINGS = {
			{ KeyConfig::Key::F1, { KeyType::Id::FACE, KeyAction::Id::FACE1 } },
			{ KeyConfig::Key::F2, { KeyType::Id::FACE, KeyAction::Id::FACE2 } },
			{ KeyConfig::Key::F3, { KeyType::Id::FACE, KeyAction::Id::FACE3 } },
			{ KeyConfig::Key::F4, { KeyType::Id::FACE, KeyAction::Id::FACE4 } },
			{ KeyConfig::Key::F5, { KeyType::Id::FACE, KeyAction::Id::FACE5 } },
			{ KeyConfig::Key::F6, { KeyType::Id::FACE, KeyAction::Id::FACE6 } },
			{ KeyConfig::Key::F7, { KeyType::Id::FACE, KeyAction::Id::FACE7 } },
			{ KeyConfig::Key::LEFT_ALT, { KeyType::Id::ACTION, KeyAction::Id::JUMP } },
			{ KeyConfig::Key::E, { KeyType::Id::MENU, KeyAction::Id::EQUIPMENT } },
			{ KeyConfig::Key::I, { KeyType::Id::MENU, KeyAction::Id::ITEMS } },
			{ KeyConfig::Key::S, { KeyType::Id::MENU, KeyAction::Id::STATS } },
			{ KeyConfig::Key::K, { KeyType::Id::MENU, KeyAction::Id::SKILLS } },
			{ KeyConfig::Key::R, { KeyType::Id::MENU, KeyAction::Id::FRIENDS } },
			{ KeyConfig::Key::W, { KeyType::Id::MENU, KeyAction::Id::WORLDMAP } },
			{ KeyConfig::Key::C, { KeyType::Id::MENU, KeyAction::Id::MAPLECHAT } },
			{ KeyConfig::Key::M, { KeyType::Id::MENU, KeyAction::Id::MINIMAP } },
			{ KeyConfig::Key::Q, { KeyType::Id::MENU, KeyAction::Id::QUESTLOG } },
			{ KeyConfig::Key::BACKSLASH, { KeyType::Id::MENU, KeyAction::Id::KEYBINDINGS } },
			{ KeyConfig::Key::APOSTROPHE, { KeyType::Id::MENU, KeyAction::Id::TOGGLECHAT } },
			{ KeyConfig::Key::H, { KeyType::Id::MENU, KeyAction::Id::WHISPER } },
			{ KeyConfig::Key::NUM1, { KeyType::Id::MENU, KeyAction::Id::SAY } },
			{ KeyConfig::Key::NUM2, { KeyType::Id::MENU, KeyAction::Id::PARTYCHAT } },
			{ KeyConfig::Key::NUM3, { KeyType::Id::MENU, KeyAction::Id::FRIENDSCHAT } },
			{ KeyConfig::Key::NUM4, { KeyType::Id::MENU, KeyAction::Id::GUILDCHAT } },
			{ KeyConfig::Key::NUM5, { KeyType::Id::MENU, KeyAction::Id::ALLIANCECHAT } },
			{ KeyConfig::Key::NUM6, { KeyType::Id::MENU, KeyAction::Id::TOSPOUSE } },
			{ KeyConfig::Key::ESCAPE, { KeyType::Id::MENU, KeyAction::Id::MENU } },
			{ KeyConfig::Key::LEFT_BRACKET, { KeyType::Id::MENU, KeyAction::Id::MENU } },
			{ KeyConfig::Key::RIGHT_BRACKET, { KeyType::Id::MENU, KeyAction::Id::QUICKSLOTS } },
			{ KeyConfig::Key::G, { KeyType::Id::MENU, KeyAction::Id::GUILD } },
			{ KeyConfig::Key::P, { KeyType::Id::MENU, KeyAction::Id::PARTY } },
			{ KeyConfig::Key::L, { KeyType::Id::MENU, KeyAction::Id::NOTIFIER } },
			{ KeyConfig::Key::GRAVE_ACCENT, { KeyType::Id::MENU, KeyAction::Id::CASHSHOP } },
			{ KeyConfig::Key::SEMICOLON, { KeyType::Id::MENU, KeyAction::Id::MEDALS } },
			{ KeyConfig::Key::B, { KeyType::Id::MENU, KeyAction::Id::MONSTERBOOK } },
			{ KeyConfig::Key::LEFT_CONTROL, { KeyType::Id::ACTION, KeyAction::Id::ATTACK } },
			{ KeyConfig::Key::Z, { KeyType::Id::ACTION, KeyAction::Id::PICKUP } },
			{ KeyConfig::Key::X, { KeyType::Id::ACTION, KeyAction::Id::SIT } },
			{ KeyConfig::Key::SPACE, { KeyType::Id::ACTION, KeyAction::Id::INTERACT_HARVEST } }
		};
	}

	UIKeyConfig::UIKeyConfig(const Inventory& in_inventory, const SkillBook& in_skillbook) : UIDragElement<PosKEYCONFIG>(), inventory(in_inventory), skillbook(in_skillbook), dirty(false), notice(Notice::NONE)
	{
		keyboard = &UI::get().get_keyboard();
		staged_mappings = keyboard->get_maplekeys();

		nl::node KeyConfig = NxFiles::ui_classic["UIWindow.img"]["KeyConfig"];
		nl::node backgrnd = KeyConfig["backgrnd"];
		Texture bg = backgrnd;

		sprites.emplace_back(backgrnd, DrawArgument(bg.get_origin()));

		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(KeyConfig["BtClose"], CLOSE_POS);
		buttons[Buttons::DEFAULT] = std::make_unique<MapleButton>(KeyConfig["BtDefault"], DEFAULT_POS);
		buttons[Buttons::DELETE] = std::make_unique<MapleButton>(KeyConfig["BtDelete"], DELETE_POS);
		buttons[Buttons::QUICKSLOT] = std::make_unique<MapleButton>(KeyConfig["BtQuickSlot"], QUICKSLOT_POS);
		buttons[Buttons::OK] = std::make_unique<MapleButton>(KeyConfig["BtOK"], OK_POS);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(KeyConfig["BtCancel"], CANCEL_POS);
		buttons[Buttons::NOTICE_OK] = std::make_unique<MapleButton>(KeyConfig["BtOK"], NOTICE_POS + NOTICE_OK_POS);
		buttons[Buttons::NOTICE_CANCEL] = std::make_unique<MapleButton>(KeyConfig["BtCancel"], NOTICE_POS + NOTICE_CANCEL_POS);

		// The quick slot is not configurable yet.
		buttons[Buttons::QUICKSLOT]->set_state(Button::State::DISABLED);
		buttons[Buttons::NOTICE_OK]->set_active(false);
		buttons[Buttons::NOTICE_CANCEL]->set_active(false);

		for (size_t i = 0; i < Notice::NONE; i++)
			notices[i] = KeyConfig["notice"][i];

		for (auto const& it : key_faces())
			key_labels[it.second.label] = KeyConfig["key"][it.second.label];

		for (auto const& function : FUNCTIONS)
		{
			action_mappings.emplace_back(get_keytype(function.action), function.action);
			action_icons[function.action] = std::make_unique<Icon>(std::make_unique<MappingIcon>(function.action), KeyConfig["icon"][function.icon], -1);
			unbound_actions_pos[function.action] = palette_slot(function.icon);
		}

		dimension = bg.get_dimensions();
		dragarea = Point<int16_t>(dimension.x(), 20);

		load_item_icons();
		load_skill_icons();
		refresh_bound_actions();
	}

	/// Load
	void UIKeyConfig::load_item_icons()
	{
		for (auto const& it : staged_mappings)
		{
			Keyboard::Mapping mapping = it.second;

			if (mapping.type == KeyType::Id::ITEM)
			{
				int32_t item_id = mapping.action;
				int16_t count = inventory.get_total_item_count(item_id);
				Texture tx = get_item_texture(item_id);

				item_icons[item_id] = std::make_unique<Icon>(std::make_unique<CountableMappingIcon>(mapping, count), tx, count);
			}
		}
	}

	void UIKeyConfig::load_skill_icons()
	{
		for (auto const& it : staged_mappings)
		{
			Keyboard::Mapping mapping = it.second;

			if (mapping.type == KeyType::Id::SKILL)
			{
				int32_t skill_id = mapping.action;
				Texture tx = get_skill_texture(skill_id);

				skill_icons[skill_id] = std::make_unique<Icon>(std::make_unique<MappingIcon>(mapping), tx, -1);
			}
		}
	}

	/// UI: General
	void UIKeyConfig::draw(float inter) const
	{
		UIElement::draw_sprites(inter);

		for (auto const& it : key_faces())
		{
			const KeyFace& face = it.second;
			Point<int16_t> face_pos = position + Point<int16_t>(face.x, face.y);

			if (Icon* ficon = mapping_icon(get_staged_mapping(it.first)))
				ficon->draw(face_pos + FACE_ICON);

			key_labels.at(face.label).draw(face_pos + FACE_LABEL);
		}

		for (auto const& it : action_icons)
			if (!is_bound(it.first))
				it.second->draw(position + unbound_actions_pos.at(it.first));

		if (notice != Notice::NONE)
			notices[notice].draw(position + NOTICE_POS);

		UIElement::draw_buttons(inter);
	}

	void UIKeyConfig::close()
	{
		clear_tooltip();
		hide_notice();
		deactivate();
		reset();
	}

	Button::State UIKeyConfig::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
		case Buttons::CLOSE:
		case Buttons::CANCEL:
			close();
			break;
		case Buttons::DEFAULT:
			show_notice(Notice::RESET);
			break;
		case Buttons::DELETE:
			show_notice(Notice::CLEAR);
			break;
		case Buttons::OK:
			save_staged_mappings();
			close();
			break;
		case Buttons::NOTICE_OK:
			answer_notice(true);
			break;
		case Buttons::NOTICE_CANCEL:
			answer_notice(false);
			break;
		default:
			break;
		}

		return Button::State::NORMAL;
	}

	Cursor::State UIKeyConfig::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State dstate = UIDragElement::send_cursor(clicked, cursorpos);

		if (dragged || notice != Notice::NONE)
			return dstate;

		KeyAction::Id unbound = unbound_action_by_position(cursorpos);

		if (unbound != KeyAction::Id::LENGTH)
		{
			Icon* icon = action_icons.at(unbound).get();

			if (clicked)
			{
				icon->start_drag(cursorpos - position - unbound_actions_pos.at(unbound));
				UI::get().drag_icon(icon);

				return Cursor::State::GRABBING;
			}

			return Cursor::State::CANGRAB;
		}

		clear_tooltip();

		int32_t key = key_by_position(cursorpos);

		if (key != KeyConfig::Key::LENGTH)
		{
			Keyboard::Mapping mapping = get_staged_mapping(key);

			if (Icon* ficon = mapping_icon(mapping))
			{
				if (clicked)
				{
					const KeyFace& face = key_faces().at(key);

					ficon->start_drag(cursorpos - position - Point<int16_t>(face.x, face.y) - FACE_ICON);
					UI::get().drag_icon(ficon);

					return Cursor::State::GRABBING;
				}

				if (mapping.type == KeyType::Id::ITEM)
					show_item(mapping.action);
				else if (mapping.type == KeyType::Id::SKILL)
					show_skill(mapping.action);

				return Cursor::State::CANGRAB;
			}
		}

		return dstate;
	}

	bool UIKeyConfig::send_icon(const Icon& icon, Point<int16_t> cursorpos)
	{
		if (notice != Notice::NONE)
			return true;

		Rectangle<int16_t> palette = Rectangle<int16_t>(
			position + PALETTE,
			position + PALETTE + Point<int16_t>(PALETTE_COLUMNS * SLOT, PALETTE_ROWS * SLOT)
		);

		if (palette.contains(cursorpos))
			icon.drop_on_bindings(cursorpos, true);
		else if (key_by_position(cursorpos) != KeyConfig::Key::LENGTH)
			icon.drop_on_bindings(cursorpos, false);

		return true;
	}

	void UIKeyConfig::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed || !escape)
			return;

		if (notice != Notice::NONE)
			answer_notice(false);
		else
			safe_close();
	}

	UIElement::Type UIKeyConfig::get_type() const
	{
		return TYPE;
	}

	void UIKeyConfig::safe_close()
	{
		if (dirty)
			show_notice(Notice::SAVE);
		else
			close();
	}

	/// Notices
	void UIKeyConfig::show_notice(Notice n)
	{
		clear_tooltip();
		notice = n;

		for (auto const& it : buttons)
		{
			Buttons id = static_cast<Buttons>(it.first);

			if (id == Buttons::NOTICE_OK || id == Buttons::NOTICE_CANCEL)
				it.second->set_active(true);
			else if (id != Buttons::QUICKSLOT)
				it.second->set_state(Button::State::DISABLED);
		}
	}

	void UIKeyConfig::hide_notice()
	{
		notice = Notice::NONE;

		for (auto const& it : buttons)
		{
			Buttons id = static_cast<Buttons>(it.first);

			if (id == Buttons::NOTICE_OK || id == Buttons::NOTICE_CANCEL)
			{
				it.second->set_active(false);
				it.second->set_state(Button::State::NORMAL);
			}
			else if (id != Buttons::QUICKSLOT)
			{
				it.second->set_state(Button::State::NORMAL);
			}
		}
	}

	void UIKeyConfig::answer_notice(bool ok)
	{
		Notice answered = notice;
		hide_notice();

		switch (answered)
		{
		case Notice::RESET:
			if (ok)
			{
				clear();

				for (auto const& it : DEFAULT_BINDINGS)
					staged_mappings[it.first] = it.second;

				refresh_bound_actions();
			}

			break;
		case Notice::CLEAR:
			if (ok)
				clear();

			break;
		case Notice::SAVE:
			if (ok)
				save_staged_mappings();

			close();
			break;
		default:
			break;
		}
	}

	/// UI: Tooltip
	void UIKeyConfig::show_item(int32_t item_id)
	{
		UI::get().show_item(Tooltip::Parent::KEYCONFIG, item_id);
	}

	void UIKeyConfig::show_skill(int32_t skill_id)
	{
		int32_t level = skillbook.get_level(skill_id);
		int32_t masterlevel = skillbook.get_masterlevel(skill_id);
		int64_t expiration = skillbook.get_expiration(skill_id);

		UI::get().show_skill(Tooltip::Parent::KEYCONFIG, skill_id, level, masterlevel, expiration);
	}

	void UIKeyConfig::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::KEYCONFIG);
	}

	/// Keymap Staging
	namespace
	{
		// Unbind the bindable key holding the mapping, if any.
		bool unbind(std::map<int32_t, Keyboard::Mapping>& mappings, Keyboard::Mapping mapping)
		{
			for (auto const& it : mappings)
			{
				int32_t key = it.first;

				if (it.second != mapping || !is_bindable(key))
					continue;

				mappings.erase(key);

				return true;
			}

			return false;
		}
	}

	void UIKeyConfig::stage_mapping(Point<int16_t> cursorposition, Keyboard::Mapping mapping)
	{
		stage_key(key_by_position(cursorposition), mapping);
	}

	void UIKeyConfig::stage_key(int32_t key, Keyboard::Mapping mapping)
	{
		if (key == KeyConfig::Key::LENGTH)
			return;

		key = KeyConfig::fold(key);

		Keyboard::Mapping prior_staged = get_staged_mapping(key);

		if (prior_staged == mapping)
			return;

		if (prior_staged.type != KeyType::Id::NONE)
			unstage_mapping(prior_staged);

		unbind(staged_mappings, mapping);

		staged_mappings[key] = mapping;

		if (mapping.type == KeyType::Id::ITEM)
		{
			int32_t item_id = mapping.action;

			if (item_icons.find(item_id) == item_icons.end())
			{
				int16_t count = inventory.get_total_item_count(item_id);
				Texture tx = get_item_texture(item_id);
				item_icons[item_id] = std::make_unique<Icon>(std::make_unique<CountableMappingIcon>(mapping, count), tx, count);
			}
		}
		else if (mapping.type == KeyType::Id::SKILL)
		{
			int32_t skill_id = mapping.action;

			if (skill_icons.find(skill_id) == skill_icons.end())
			{
				Texture tx = get_skill_texture(skill_id);
				skill_icons[skill_id] = std::make_unique<Icon>(std::make_unique<MappingIcon>(mapping), tx, -1);
			}
		}

		refresh_bound_actions();
		dirty = true;
	}

	void UIKeyConfig::unstage_mapping(Keyboard::Mapping mapping)
	{
		if (!unbind(staged_mappings, mapping))
			return;

		if (mapping.type == KeyType::Id::ITEM)
			item_icons.erase(mapping.action);
		else if (mapping.type == KeyType::Id::SKILL)
			skill_icons.erase(mapping.action);

		refresh_bound_actions();
		dirty = true;
	}

	void UIKeyConfig::save_staged_mappings()
	{
		std::vector<std::tuple<KeyConfig::Key, KeyType::Id, int32_t>> updated_actions;

		for (auto const& it : staged_mappings)
		{
			Keyboard::Mapping mapping = it.second;

			if (mapping != keyboard->get_maple_mapping(it.first))
				updated_actions.emplace_back(KeyConfig::actionbyid(it.first), mapping.type, mapping.action);
		}

		for (auto const& it : keyboard->get_maplekeys())
			if (staged_mappings.find(it.first) == staged_mappings.end())
				updated_actions.emplace_back(KeyConfig::actionbyid(it.first), KeyType::Id::NONE, KeyAction::Id::LENGTH);

		if (updated_actions.size() > 0)
			ChangeKeyMapPacket(updated_actions).dispatch();

		for (auto const& action : updated_actions)
		{
			KeyConfig::Key key = std::get<0>(action);
			KeyType::Id type = std::get<1>(action);
			int32_t keyAction = std::get<2>(action);

			if (type == KeyType::Id::NONE)
				keyboard->remove(key);
			else
				keyboard->assign(key, type, keyAction);
		}

		dirty = false;
	}

	void UIKeyConfig::refresh_bound_actions()
	{
		bound_actions.clear();

		for (auto const& it : staged_mappings)
			if (is_bindable(it.first) && is_action_mapping(it.second))
				bound_actions.push_back(KeyAction::actionbyid(it.second.action));
	}

	// Unbind every key the window shows; fixed keys such as Esc keep
	// their function.
	void UIKeyConfig::clear()
	{
		item_icons.clear();
		skill_icons.clear();

		for (auto it = staged_mappings.begin(); it != staged_mappings.end();)
		{
			if (is_bindable(it->first))
				it = staged_mappings.erase(it);
			else
				++it;
		}

		refresh_bound_actions();
		dirty = true;
	}

	void UIKeyConfig::reset()
	{
		item_icons.clear();
		skill_icons.clear();
		staged_mappings = keyboard->get_maplekeys();
		load_item_icons();
		load_skill_icons();
		refresh_bound_actions();
		dirty = false;
	}

	/// Helpers
	Icon* UIKeyConfig::mapping_icon(Keyboard::Mapping mapping) const
	{
		if (mapping.type == KeyType::Id::ITEM)
		{
			auto iter = item_icons.find(mapping.action);

			return iter == item_icons.end() ? nullptr : iter->second.get();
		}

		if (mapping.type == KeyType::Id::SKILL)
		{
			auto iter = skill_icons.find(mapping.action);

			return iter == skill_icons.end() ? nullptr : iter->second.get();
		}

		if (is_action_mapping(mapping))
			return action_icons.at(KeyAction::actionbyid(mapping.action)).get();

		return nullptr;
	}

	Texture UIKeyConfig::get_item_texture(int32_t item_id) const
	{
		const ItemData& data = ItemData::get(item_id);
		return data.get_icon(false);
	}

	Texture UIKeyConfig::get_skill_texture(int32_t skill_id) const
	{
		const SkillData& data = SkillData::get(skill_id);
		return data.get_icon(SkillData::Icon::NORMAL);
	}

	int32_t UIKeyConfig::key_by_position(Point<int16_t> cursorpos) const
	{
		for (auto const& it : key_faces())
			if (face_bounds(it.second).contains(cursorpos - position))
				return it.first;

		return KeyConfig::Key::LENGTH;
	}

	KeyAction::Id UIKeyConfig::unbound_action_by_position(Point<int16_t> cursorpos) const
	{
		for (auto const& it : unbound_actions_pos)
		{
			if (is_bound(it.first))
				continue;

			Rectangle<int16_t> icon_rect = Rectangle<int16_t>(
				position + it.second,
				position + it.second + Point<int16_t>(ICON_SIZE, ICON_SIZE)
			);

			if (icon_rect.contains(cursorpos))
				return it.first;
		}

		return KeyAction::Id::LENGTH;
	}

	Keyboard::Mapping UIKeyConfig::get_staged_mapping(int32_t keycode) const
	{
		auto iter = staged_mappings.find(KeyConfig::fold(keycode));

		if (iter == staged_mappings.end())
			return {};

		return iter->second;
	}

	bool UIKeyConfig::is_action_mapping(Keyboard::Mapping mapping) const
	{
		return std::find(action_mappings.begin(), action_mappings.end(), mapping) != action_mappings.end();
	}

	bool UIKeyConfig::is_bound(KeyAction::Id action) const
	{
		return std::find(bound_actions.begin(), bound_actions.end(), action) != bound_actions.end();
	}

	KeyType::Id UIKeyConfig::get_keytype(KeyAction::Id action)
	{
		switch (action)
		{
		case KeyAction::Id::EQUIPMENT:
		case KeyAction::Id::ITEMS:
		case KeyAction::Id::STATS:
		case KeyAction::Id::SKILLS:
		case KeyAction::Id::FRIENDS:
		case KeyAction::Id::WORLDMAP:
		case KeyAction::Id::MAPLECHAT:
		case KeyAction::Id::MINIMAP:
		case KeyAction::Id::QUESTLOG:
		case KeyAction::Id::KEYBINDINGS:
		case KeyAction::Id::TOGGLECHAT:
		case KeyAction::Id::WHISPER:
		case KeyAction::Id::SAY:
		case KeyAction::Id::PARTYCHAT:
		case KeyAction::Id::MENU:
		case KeyAction::Id::QUICKSLOTS:
		case KeyAction::Id::GUILD:
		case KeyAction::Id::FRIENDSCHAT:
		case KeyAction::Id::PARTY:
		case KeyAction::Id::NOTIFIER:
		case KeyAction::Id::CASHSHOP:
		case KeyAction::Id::GUILDCHAT:
		case KeyAction::Id::MEDALS:
		case KeyAction::Id::BITS:
		case KeyAction::Id::ALLIANCECHAT:
		case KeyAction::Id::MAPLENEWS:
		case KeyAction::Id::MANAGELEGION:
		case KeyAction::Id::PROFESSION:
		case KeyAction::Id::BOSSPARTY:
		case KeyAction::Id::ITEMPOT:
		case KeyAction::Id::EVENT:
		case KeyAction::Id::SILENTCRUSADE:
		case KeyAction::Id::BATTLEANALYSIS:
		case KeyAction::Id::GUIDE:
		case KeyAction::Id::VIEWERSCHAT:
		case KeyAction::Id::ENHANCEEQUIP:
		case KeyAction::Id::MONSTERCOLLECTION:
		case KeyAction::Id::SOULWEAPON:
		case KeyAction::Id::CHARINFO:
		case KeyAction::Id::CHANGECHANNEL:
		case KeyAction::Id::MAINMENU:
		case KeyAction::Id::SCREENSHOT:
		case KeyAction::Id::PICTUREMODE:
		case KeyAction::Id::MAPLEACHIEVEMENT:
			return KeyType::Id::MENU;
		case KeyAction::Id::PICKUP:
		case KeyAction::Id::SIT:
		case KeyAction::Id::ATTACK:
		case KeyAction::Id::JUMP:
		case KeyAction::Id::INTERACT_HARVEST:
			return KeyType::Id::ACTION;
		case KeyAction::Id::MAPLESTORAGE:
		case KeyAction::Id::SAFEMODE:
		case KeyAction::Id::MUTE:
		case KeyAction::Id::MONSTERBOOK:
		case KeyAction::Id::TOSPOUSE:
			return KeyType::Id::MENU;
		case KeyAction::Id::FACE1:
		case KeyAction::Id::FACE2:
		case KeyAction::Id::FACE3:
		case KeyAction::Id::FACE4:
		case KeyAction::Id::FACE5:
		case KeyAction::Id::FACE6:
		case KeyAction::Id::FACE7:
			return KeyType::Id::FACE;
		case KeyAction::Id::LEFT:
		case KeyAction::Id::RIGHT:
		case KeyAction::Id::UP:
		case KeyAction::Id::DOWN:
		case KeyAction::Id::BACK:
		case KeyAction::Id::TAB:
		case KeyAction::Id::RETURN:
		case KeyAction::Id::ESCAPE:
		case KeyAction::Id::SPACE:
		case KeyAction::Id::DELETE:
		case KeyAction::Id::HOME:
		case KeyAction::Id::END:
		case KeyAction::Id::COPY:
		case KeyAction::Id::PASTE:
		case KeyAction::Id::LENGTH:
		default:
			return KeyType::Id::NONE;
		}
	}

	/// Item count
	void UIKeyConfig::update_item_count(InventoryType::Id type, int16_t slot, int16_t change)
	{
		int32_t item_id = inventory.get_item_id(type, slot);

		if (item_icons.find(item_id) == item_icons.end())
			return;

		int16_t item_count = item_icons[item_id]->get_count();
		item_icons[item_id]->set_count(item_count + change);
	}

	/// MappingIcon
	UIKeyConfig::MappingIcon::MappingIcon(Keyboard::Mapping m) : mapping(m) {}

	UIKeyConfig::MappingIcon::MappingIcon(KeyAction::Id action)
	{
		KeyType::Id type = UIKeyConfig::get_keytype(action);
		mapping = Keyboard::Mapping(type, action);
	}

	void UIKeyConfig::MappingIcon::drop_on_stage() const
	{
		auto keyconfig = UI::get().get_element<UIKeyConfig>();
		keyconfig->unstage_mapping(mapping);
	}

	void UIKeyConfig::MappingIcon::drop_on_bindings(Point<int16_t> cursorposition, bool remove) const
	{
		auto keyconfig = UI::get().get_element<UIKeyConfig>();

		if (remove)
			keyconfig->unstage_mapping(mapping);
		else
			keyconfig->stage_mapping(cursorposition, mapping);
	}

	Icon::IconType UIKeyConfig::MappingIcon::get_type()
	{
		return Icon::IconType::KEY;
	}

	UIKeyConfig::CountableMappingIcon::CountableMappingIcon(Keyboard::Mapping m, int16_t c) : UIKeyConfig::MappingIcon(m), count(c) {}

	void UIKeyConfig::CountableMappingIcon::set_count(int16_t c)
	{
		count = c;
	}
}
