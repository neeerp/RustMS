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
#include "UI.h"

#include <chrono>

#include "UIStateCashShop.h"
#include "UIStateGame.h"
#include "UIStateLogin.h"
#include "Window.h"

#include "../Character/Inventory/Equip.h"
#include "../Gameplay/Stage.h"
#include "../Net/Handlers/StorageContents.h"
#include "../Net/Packets/GameplayPackets.h"

#include "Components/Slider.h"

#include "UITypes/UICashShop.h"
#include "UITypes/UIChannel.h"
#include "UITypes/UIChatBar.h"
#include "UITypes/UIDuey.h"
#include "UITypes/UIGameOption.h"
#include "UITypes/UIItemMegaphone.h"
#include "UITypes/UIItemProtector.h"
#include "UITypes/UIIncubator.h"
#include "UITypes/UIKarmaScissors.h"
#include "UITypes/UIKite.h"
#include "UITypes/UITransform.h"
#include "UITypes/UINpcTalk.h"
#include "UITypes/UIPersonalShop.h"
#include "UITypes/UIQuickSlotConfig.h"
#include "UITypes/UIShop.h"
#include "UITypes/UIStorage.h"
#include "UITypes/UIStoreBank.h"
#include "UITypes/UIStatusBar.h"
#include "UITypes/UISystemOption.h"
#include "UITypes/UITradingRoom.h"
#include "UITypes/UIVegaSpell.h"
#include "UITypes/UIWeddingRegistry.h"
#include "UITypes/UIWishlistInput.h"
#include "UITypes/UIEnchantSkill.h"
#include "UITypes/UIViciousHammer.h"
#include "UITypes/UIWorldMap.h"

namespace ms
{
	UI::UI()
	{
		state = std::make_unique<UIStateNull>();
		enabled = true;
	}

	void UI::init()
	{
		cursor.init();
		change_state(State::LOGIN);
	}

	void UI::draw(float alpha) const
	{
		state->draw(alpha, cursor.get_position());

		scrollingnotice.draw(alpha);

		cursor.draw(alpha);
	}

	void UI::update()
	{
		state->update();

		Slider::update_held(cursor.is_pressed());

		scrollingnotice.update();

		cursor.update();
	}

	void UI::enable()
	{
		enabled = true;
	}

	void UI::disable()
	{
		enabled = false;
	}

	void UI::change_state(State id)
	{
		switch (id)
		{
		case State::LOGIN:
			state = std::make_unique<UIStateLogin>();
			break;
		case State::GAME:
			state = std::make_unique<UIStateGame>();
			break;
		case State::CASHSHOP:
			state = std::make_unique<UIStateCashShop>();
			break;
		}
	}

	void UI::quit()
	{
		quitted = true;
	}

	bool UI::not_quitted() const
	{
		return !quitted;
	}

	bool UI::modal_open() const
	{
		return state->modal_open();
	}

	bool UI::dialog_open()
	{
		auto open = [](auto element)
		{
			return element && element->is_active();
		};

		return open(get_element<UITradingRoom>()) || open(get_element<UIPersonalShop>())
			|| open(get_element<UIShop>()) || open(get_element<UIStorage>()) || open(get_element<UIDuey>())
			|| open(get_element<UIStoreBank>())
			|| open(get_element<UIItemMegaphone>()) || open(get_element<UIItemProtector>())
			|| open(get_element<UIKarmaScissors>()) || open(get_element<UIIncubator>())
			|| open(get_element<UIViciousHammer>())
			|| open(get_element<UIVegaSpell>()) || open(get_element<UIKiteWrite>())
			|| open(get_element<UITransform>())
			|| open(get_element<UIEnchantSkill>())
			|| open(get_element<UIWishlistInput>()) || open(get_element<UIWeddingRegistry>());
	}

	void UI::send_cursor(Point<int16_t> cursorpos, Cursor::State cursorstate)
	{
		Cursor::State nextstate = state->send_cursor(cursorstate, cursorpos);
		cursor.set_state(nextstate);
		cursor.set_position(cursorpos);
	}

	void UI::send_focus(int focused)
	{
		if (focused)
		{
			// The window gained input focus
			Sound::set_sfxvolume(Sound::configured_volume());
			Music::set_bgmvolume(Music::configured_volume());

			// v83 checks the balances on coming back to the Cash Shop.
			if (auto cashshop = get_element<UICashShop>(); cashshop && cashshop->request())
				CheckCashPacket().dispatch();
		} else
		{
			// The window lost input focus, and won't see the releases of the
			// keys held now.
			Sound::set_sfxvolume(0);
			Music::set_bgmvolume(0);

			is_key_down.clear();
			Stage::get().release_keys();
			Stage::get().release_cursor();
			cursor.set_pressed(false);
		}
	}

	void UI::send_scroll(double yoffset)
	{
		state->send_scroll(yoffset, cursor.get_position());
	}

	void UI::send_close()
	{
		state->send_close();
	}

	void UI::send_cursor(bool pressed)
	{
		Cursor::State cursorstate = (pressed && enabled) ? Cursor::State::CLICKING : Cursor::State::IDLE;
		Point<int16_t> cursorpos = cursor.get_position();
		cursor.set_pressed(pressed && enabled);

		// A click that opens a window can focus that window's field, such as the number entry. Only a field that
		// was focused before the click loses focus to it.
		Textfield *focused_before = focusedtextfield.get();
		send_cursor(cursorpos, cursorstate);

		// A release over a window never reaches the stage.
		if (!pressed)
			Stage::get().release_cursor();

		if (focusedtextfield && pressed && focusedtextfield.get() == focused_before)
		{
			Cursor::State tstate = focusedtextfield->send_cursor(cursorpos, pressed);

			switch (tstate)
			{
				case Cursor::State::IDLE:
					focusedtextfield = {};
					break;
			}
		}
	}

	void UI::send_cursor(Point<int16_t> pos)
	{
		send_cursor(pos, cursor.get_state());
	}

	bool UI::is_cursor_pressed() const
	{
		return cursor.is_pressed();
	}

	void UI::rightclick()
	{
		Point<int16_t> pos = cursor.get_position();
		state->rightclick(pos);
	}

	void UI::doubleclick()
	{
		Point<int16_t> pos = cursor.get_position();
		state->doubleclick(pos);
	}

	void UI::send_key(int32_t keycode, bool pressed)
	{
		// Scroll Lock takes a screenshot before anything else sees the key
		// (FUN_009e40f1).
		if (keycode == GLFW_KEY_SCROLL_LOCK)
		{
			if (pressed)
			{
				int64_t now = std::chrono::duration_cast<std::chrono::milliseconds>(
					std::chrono::steady_clock::now().time_since_epoch()).count();
				bool in_game = static_cast<bool>(get_element<UIStatusBar>());

				switch (screenshots.press(now, in_game))
				{
					case Screenshot::Press::TAKE:
						Window::get().screenshot();
						break;
					case Screenshot::Press::REFUSE:
						if (auto chatbar = get_element<UIChatBar>())
							chatbar->send_chatline("You cannot take screenshots at this moment. Please try again later.",
												   UIChatBar::LineType::SKILL);
						break;
					default:
						break;
				}
			}

			return;
		}

		if ((is_key_down[GLFW_KEY_LEFT_ALT] || is_key_down[GLFW_KEY_RIGHT_ALT]) &&
			(is_key_down[GLFW_KEY_ENTER] || is_key_down[GLFW_KEY_KP_ENTER]))
		{
			Window::get().toggle_fullscreen();

			is_key_down[GLFW_KEY_LEFT_ALT] = false;
			is_key_down[GLFW_KEY_RIGHT_ALT] = false;
			is_key_down[GLFW_KEY_ENTER] = false;
			is_key_down[GLFW_KEY_KP_ENTER] = false;

			return;
		}

		if (is_key_down[keyboard.capslockcode()])
			caps_lock_enabled = !caps_lock_enabled;

		auto quickslots = get_element<UIQuickSlotConfig>();
		UIElement *quickslot_notice = state->get(UIElement::Type::NOTICE);

		if (quickslots && quickslots->is_active() && !(quickslot_notice && quickslot_notice->is_active()))
		{
			// The quick slot editor reads every key itself while none of its
			// notices is up (OnKey 72ce91).
			quickslots->send_raw_key(keycode, pressed);
		} else if (focusedtextfield)
		{
			bool ctrl = is_key_down[keyboard.leftctrlcode()] || is_key_down[keyboard.rightctrlcode()];

			if (ctrl)
			{
				if (!pressed)
				{
					KeyAction::Id action = keyboard.get_ctrl_action(keycode);

					switch (action)
					{
						case KeyAction::Id::COPY:
							Window::get().setclipboard(focusedtextfield->get_text());
							break;
						case KeyAction::Id::PASTE:
							focusedtextfield->add_string(Window::get().getclipboard());
							break;
					}
				}
			} else
			{
				bool shift = is_key_down[keyboard.leftshiftcode()] || is_key_down[keyboard.rightshiftcode()] ||
							 caps_lock_enabled;
				Keyboard::Mapping mapping = keyboard.get_text_mapping(keycode, shift);
				focusedtextfield->send_key(mapping.type, mapping.action, pressed);
			}
		} else
		{
			Keyboard::Mapping mapping = keyboard.get_mapping(keycode);

			bool sent = false;
			std::list<UIElement::Type> types;

			bool escape = keycode == GLFW_KEY_ESCAPE;
			bool tab = keycode == GLFW_KEY_TAB;
			bool enter = keycode == GLFW_KEY_ENTER || keycode == GLFW_KEY_KP_ENTER;
			bool up_down = keycode == GLFW_KEY_UP || keycode == GLFW_KEY_DOWN;
			bool left_right = keycode == GLFW_KEY_LEFT || keycode == GLFW_KEY_RIGHT;
			bool arrows = up_down || left_right;

			auto statusbar = UI::get().get_element<UIStatusBar>();
			auto channel = UI::get().get_element<UIChannel>();
			auto worldmap = UI::get().get_element<UIWorldMap>();
			auto gameoption = UI::get().get_element<UIGameOption>();
			auto systemoption = UI::get().get_element<UISystemOption>();
			auto shop = UI::get().get_element<UIShop>();
			auto storage = UI::get().get_element<UIStorage>();
			auto npctalk = UI::get().get_element<UINpcTalk>();
			//auto report = UI::get().get_element<UIReport>();
			//auto whisper = UI::get().get_element<UIWhisper>();

			// A notice over an NPC's talk, such as a prompt's complaint, hears
			// the keys first.
			UIElement *notice = state->get(UIElement::Type::NOTICE);
			bool notice_open = notice && notice->is_active();

			if (npctalk && npctalk->is_active() && !notice_open)
			{
				npctalk->send_key(mapping.action, pressed, escape);
				sent = true;
			} else if (statusbar && statusbar->is_menu_active())
			{
				statusbar->send_key(mapping.action, pressed, escape);
				sent = true;
			} else if (channel && channel->is_active() && mapping.action != KeyAction::Id::CHANGECHANNEL)
			{
				channel->send_key(mapping.action, pressed, escape);
				sent = true;
			} else if (worldmap && worldmap->is_active() && mapping.action != KeyAction::Id::WORLDMAP)
			{
				worldmap->send_key(mapping.action, pressed, escape);
				sent = true;
			} else if (gameoption && gameoption->is_active())
			{
				gameoption->send_key(mapping.action, pressed, escape);
				sent = true;
			} else if (systemoption && systemoption->is_active())
			{
				systemoption->send_key(mapping.action, pressed, escape);
				sent = true;
			} else if (shop && shop->is_active())
			{
				shop->send_key(mapping.action, pressed, escape);
				sent = true;
			} else if (storage && storage->is_active())
			{
				storage->send_key(mapping.action, pressed, escape);
				sent = true;
			} else
			{
				// All
				if (escape || tab || enter || arrows)
				{
					// Login
					types.emplace_back(UIElement::Type::WORLDSELECT);
					types.emplace_back(UIElement::Type::CHARSELECT);
					types.emplace_back(UIElement::Type::RACESELECT);            // No tab
					types.emplace_back(
							UIElement::Type::CLASSCREATION);            // No tab (No arrows, but shouldn't send else where)
					types.emplace_back(
							UIElement::Type::LOGINNOTICE);            // No tab (No arrows, but shouldn't send else where)
					types.emplace_back(
							UIElement::Type::LOGINNOTICE_CONFIRM);    // No tab (No arrows, but shouldn't send else where)
					types.emplace_back(
							UIElement::Type::LOGINWAIT);                // No tab (No arrows, but shouldn't send else where)
				}

				if (escape)
				{
					// Login
					types.emplace_back(UIElement::Type::SOFTKEYBOARD);

					// Game
					types.emplace_back(UIElement::Type::NOTICE);
					types.emplace_back(UIElement::Type::ENTERNUMBER);
					types.emplace_back(UIElement::Type::KEYCONFIG);
					types.emplace_back(UIElement::Type::EVENT);
					types.emplace_back(UIElement::Type::STATSINFO);
					types.emplace_back(UIElement::Type::ITEMINVENTORY);
					types.emplace_back(UIElement::Type::EQUIPINVENTORY);
					types.emplace_back(UIElement::Type::SKILLBOOK);
					types.emplace_back(UIElement::Type::MONSTERBOOK);
					types.emplace_back(UIElement::Type::QUESTLOG);
					types.emplace_back(UIElement::Type::USERLIST);
					types.emplace_back(UIElement::Type::PARTYSEARCH);
					types.emplace_back(UIElement::Type::FRIENDGROUP);
					types.emplace_back(UIElement::Type::NPCTALK);
					types.emplace_back(UIElement::Type::PETITEMSEARCH);
					types.emplace_back(UIElement::Type::ITEMSEARCH);
					types.emplace_back(UIElement::Type::ITEMSEARCHRESULT);
					types.emplace_back(UIElement::Type::CHARINFO);
					types.emplace_back(UIElement::Type::TRADINGROOM);
					types.emplace_back(UIElement::Type::PERSONALSHOP);
					types.emplace_back(UIElement::Type::SHOPSELL);
					types.emplace_back(UIElement::Type::SHOPSALES);
					types.emplace_back(UIElement::Type::MERCHANTVISITORS);
					types.emplace_back(UIElement::Type::MERCHANTBLACKLIST);
					types.emplace_back(UIElement::Type::TELEPORTROCK);
					types.emplace_back(UIElement::Type::MEGAPHONE);
					types.emplace_back(UIElement::Type::MAPLETV);
					types.emplace_back(UIElement::Type::ITEMMEGAPHONE);
					types.emplace_back(UIElement::Type::VICIOUSHAMMER);
					types.emplace_back(UIElement::Type::MAKER);
					types.emplace_back(UIElement::Type::DUEY);
					types.emplace_back(UIElement::Type::STOREBANK);
					types.emplace_back(UIElement::Type::VEGASPELL);
					types.emplace_back(UIElement::Type::ITEMPROTECTOR);
					types.emplace_back(UIElement::Type::KARMASCISSORS);
					types.emplace_back(UIElement::Type::INCUBATOR);
					types.emplace_back(UIElement::Type::INVITATION);
					types.emplace_back(UIElement::Type::VEGARESULT);
					types.emplace_back(UIElement::Type::ENCHANTSKILL);
					types.emplace_back(UIElement::Type::PETCHOOSER);
					types.emplace_back(UIElement::Type::APRESET);
					types.emplace_back(UIElement::Type::SPRESET);
					types.emplace_back(UIElement::Type::GUILDMARKMAKER);
					types.emplace_back(UIElement::Type::AVATARSELECT);
					types.emplace_back(UIElement::Type::SLIDEMENU);
					types.emplace_back(UIElement::Type::KITEWRITE);
					types.emplace_back(UIElement::Type::TRANSFORM);
					types.emplace_back(UIElement::Type::KITEVIEW);
					types.emplace_back(UIElement::Type::FAMILY);
					types.emplace_back(UIElement::Type::FAMILYTREE);
				}
				else if (enter)
				{
					// Login
					types.emplace_back(UIElement::Type::SOFTKEYBOARD);

					// Game
					types.emplace_back(UIElement::Type::NOTICE);
					types.emplace_back(UIElement::Type::ENTERNUMBER);
					types.emplace_back(UIElement::Type::FRIENDGROUP);
					types.emplace_back(UIElement::Type::TELEPORTROCK);
					types.emplace_back(UIElement::Type::MEGAPHONE);
					types.emplace_back(UIElement::Type::MAPLETV);
					types.emplace_back(UIElement::Type::ITEMPROTECTOR);
					types.emplace_back(UIElement::Type::KARMASCISSORS);
					types.emplace_back(UIElement::Type::INCUBATOR);
					types.emplace_back(UIElement::Type::INVITATION);
					types.emplace_back(UIElement::Type::MERCHANTVISITORS);
					types.emplace_back(UIElement::Type::MERCHANTBLACKLIST);
					types.emplace_back(UIElement::Type::PETCHOOSER);
					types.emplace_back(UIElement::Type::APRESET);
					types.emplace_back(UIElement::Type::SPRESET);
					types.emplace_back(UIElement::Type::GUILDMARKMAKER);
					types.emplace_back(UIElement::Type::AVATARSELECT);
					types.emplace_back(UIElement::Type::SLIDEMENU);
					types.emplace_back(UIElement::Type::KITEWRITE);
					types.emplace_back(UIElement::Type::TRANSFORM);
					types.emplace_back(UIElement::Type::KITEVIEW);
				} else if (tab)
				{
					// Game
					types.emplace_back(UIElement::Type::ITEMINVENTORY);
					types.emplace_back(UIElement::Type::EQUIPINVENTORY);
					types.emplace_back(UIElement::Type::SKILLBOOK);
					types.emplace_back(UIElement::Type::QUESTLOG);
					types.emplace_back(UIElement::Type::USERLIST);
				}

				if (types.size() > 0)
				{
					auto element = state->get_front(types);

					if (element && element != nullptr)
					{
						element->send_key(mapping.action, pressed, escape);
						sent = true;
					}
				}
				// TODO: (rich) fix
			}

			if (!sent)
			{
				auto chatbar = UI::get().get_element<UIChatBar>();

				if (escape)
				{
					if (chatbar && chatbar->is_chatfieldopen())
						chatbar->send_key(mapping.action, pressed, escape);
					else
						state->send_key(mapping.type, mapping.action, pressed, escape);
				} else if (enter)
				{
					if (chatbar)
						chatbar->send_key(mapping.action, pressed, escape);
					else
						state->send_key(mapping.type, mapping.action, pressed, escape);
				} else
				{
					state->send_key(mapping.type, mapping.action, pressed, escape);
				}
			}
		}

		// A dialog or text field may have taken this release. The player
		// still has to let go of the key, or it keeps walking.
		if (!pressed)
		{
			Keyboard::Mapping mapping = keyboard.get_mapping(keycode);
			Stage::get().release_key(mapping.type, mapping.action);
		}

		is_key_down[keycode] = pressed;
	}

	void UI::set_scrollnotice(const std::string &notice)
	{
		scrollingnotice.setnotice(notice);
	}

	void UI::focus_textfield(Textfield *tofocus)
	{
		if (focusedtextfield)
			focusedtextfield->set_state(Textfield::State::NORMAL);

		focusedtextfield = tofocus;
	}

	void UI::remove_textfield()
	{
		if (focusedtextfield)
			focusedtextfield->set_state(Textfield::State::NORMAL);

		focusedtextfield = {};
	}

	void UI::drag_icon(Icon *icon)
	{
		state->drag_icon(icon);
	}

	void UI::cancel_drag()
	{
		state->cancel_drag();
	}

	void UI::add_keymapping(uint8_t no, uint8_t type, int32_t action)
	{
		keyboard.assign(no, type, action);
	}

	void UI::clear_tooltip(Tooltip::Parent parent)
	{
		state->clear_tooltip(parent);
	}

	void UI::show_equip(Tooltip::Parent parent, int16_t slot)
	{
		state->show_equip(parent, slot);
	}

	void UI::show_item(Tooltip::Parent parent, int32_t item_id, int64_t expiration, int16_t flags, bool owned,
					   int32_t remain_life, const std::string &owner)
	{
		state->show_item(parent, item_id, expiration, flags, owned, remain_life, owner);
	}

	void UI::show_item(Tooltip::Parent parent, const storage_contents::Stored &item)
	{
		if (item.itemid / 10000 == 500)
		{
			show_item(parent, item.itemid, item.date_dead, 0, true, item.remain_life);
		}
		else if (item.itemid / 1000000 == 1)
		{
			// The equip as the packet made it, upgrades and all, like one in the inventory.
			EnumMap<EquipStat::Id, uint16_t> stats;

			for (EquipStat::Id es : EquipStat::values)
				stats[es] = item.stats[es];

			state->show_stored_equip(parent,
									 Equip(item.itemid, item.expiration, item.owner, item.flags, item.slots,
										   item.level, stats, item.itemlevel, item.itemexp, item.vicious),
									 item.cash);
		}
		else
		{
			show_item(parent, item.itemid, item.expiration, item.flags, true, 0, item.owner);
		}
	}

	void
	UI::show_skill(Tooltip::Parent parent, int32_t skill_id, int32_t level, int32_t masterlevel, int64_t expiration,
				   bool levels)
	{
		state->show_skill(parent, skill_id, level, masterlevel, expiration, levels);
	}

	void UI::show_macro(Tooltip::Parent parent, size_t index)
	{
		state->show_macro(parent, index);
	}

	void UI::show_text(Tooltip::Parent parent, std::string text)
	{
		state->show_text(parent, text);
	}

	void UI::show_map(Tooltip::Parent parent, std::string name, std::string description)
	{
		state->show_map(parent, name, description);
	}

	void UI::show_zone(Tooltip::Parent parent, std::string title, std::string description)
	{
		state->show_zone(parent, title, description);
	}

	void UI::show_line(Tooltip::Parent parent, std::string text)
	{
		state->show_line(parent, text);
	}

	void UI::show_help(UIElement::Type owner, std::string title, std::string description,
					   std::optional<Point<int16_t>> anchor, bool above)
	{
		state->show_help(owner, title, description, anchor, above);
	}

	void UI::clear_help(UIElement::Type owner)
	{
		state->clear_help(owner);
	}

	SkillMacros &UI::get_skill_macros()
	{
		return skill_macros;
	}

	QuickSlots &UI::get_quick_slots()
	{
		return quick_slots;
	}

	Keyboard &UI::get_keyboard()
	{
		return keyboard;
	}

	void UI::remove(UIElement::Type type)
	{
		focusedtextfield = {};

		state->remove(type);
	}

	void UI::raise(UIElement::Type type)
	{
		state->raise(type);
	}
}