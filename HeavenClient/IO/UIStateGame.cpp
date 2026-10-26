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
#include "UIStateGame.h"
#include "WindowLayer.h"

#include "UI.h"
#include "InputFocus.h"
#include "UiControl.h"
#include "FamilyActions.h"
#include "Components/TooltipBox.h"

#include "../Util/ChatLinks.h"

#include "UITypes/UIBuffList.h"
#include "UITypes/UIPartyHP.h"
#include "UITypes/UIPartySearch.h"
#include "UITypes/UIQuestHelper.h"
#include "UITypes/UIChannel.h"
#include "UITypes/UICharInfo.h"
#include "UITypes/UIContextMenu.h"
#include "UITypes/UIChatBar.h"
#include "UITypes/UIEquipInventory.h"
#include "UITypes/UIItemInventory.h"
#include "UITypes/UIKeyConfig.h"
#include "UITypes/UIKite.h"
#include "UITypes/UIMessenger.h"
#include "UITypes/UIMiniMap.h"
#include "UITypes/UINotice.h"
#include "UITypes/UIQuestComplete.h"
#include "UITypes/UIScriptMessage.h"
#include "UITypes/UIQuestLog.h"
#include "UITypes/UIMedals.h"
#include "UITypes/UIQuit.h"
#include "UITypes/UIShop.h"
#include "UITypes/UIMonsterBook.h"
#include "UITypes/UISkillBook.h"
#include "UITypes/UIStatsInfo.h"
#include "UITypes/UIStorage.h"
#include "UITypes/UIStatusBar.h"
#include "UITypes/UIStatusMessenger.h"
#include "UITypes/UIInviteCard.h"
#include "UITypes/UIUserList.h"
#include "UITypes/UIWorldMap.h"

#include "../Character/Blacklist.h"

#include "../Net/Handlers/MiniGame.h"
#include "../Net/Packets/GameplayPackets.h"
#include "../Net/Packets/MiniRoomPackets.h"
#include "../Net/Packets/PlayerInteractionPackets.h"

#include <algorithm>

namespace ms
{
	namespace
	{
		Tooltip::Parent tooltip_parent_for(UIElement::Type type)
		{
			switch (type)
			{
				case UIElement::Type::EQUIPINVENTORY:
					return Tooltip::Parent::EQUIPINVENTORY;
				case UIElement::Type::ITEMINVENTORY:
					return Tooltip::Parent::ITEMINVENTORY;
				case UIElement::Type::SKILLBOOK:
					return Tooltip::Parent::SKILLBOOK;
				case UIElement::Type::SKILLMACRO:
					return Tooltip::Parent::SKILLMACRO;
				case UIElement::Type::SHOP:
					return Tooltip::Parent::SHOP;
				case UIElement::Type::STORAGE:
					return Tooltip::Parent::STORAGE;
				case UIElement::Type::DUEY:
					return Tooltip::Parent::DUEY;
				case UIElement::Type::WEDDINGREGISTRY:
					return Tooltip::Parent::WEDDINGREGISTRY;
				case UIElement::Type::FAMILY:
					return Tooltip::Parent::FAMILY;
				case UIElement::Type::FAMILYTREE:
					return Tooltip::Parent::FAMILYTREE;
				case UIElement::Type::TRADINGROOM:
					return Tooltip::Parent::TRADINGROOM;
				case UIElement::Type::PERSONALSHOP:
					return Tooltip::Parent::PERSONALSHOP;
				case UIElement::Type::ITEMMEGAPHONE:
					return Tooltip::Parent::ITEMMEGAPHONE;
				case UIElement::Type::VICIOUSHAMMER:
					return Tooltip::Parent::VICIOUSHAMMER;
				case UIElement::Type::MAKER:
					return Tooltip::Parent::MAKER;
				case UIElement::Type::VEGASPELL:
					return Tooltip::Parent::VEGASPELL;
				case UIElement::Type::ITEMPROTECTOR:
					return Tooltip::Parent::ITEMPROTECTOR;
				case UIElement::Type::KARMASCISSORS:
					return Tooltip::Parent::KARMASCISSORS;
				case UIElement::Type::VEGARESULT:
					return Tooltip::Parent::VEGARESULT;
				case UIElement::Type::ENCHANTSKILL:
					return Tooltip::Parent::ENCHANTSKILL;
				case UIElement::Type::EVENT:
					return Tooltip::Parent::EVENT;
				case UIElement::Type::KEYCONFIG:
					return Tooltip::Parent::KEYCONFIG;
				case UIElement::Type::WORLDMAP:
					return Tooltip::Parent::WORLDMAP;
				case UIElement::Type::MEDALS:
					return Tooltip::Parent::MEDALS;
				case UIElement::Type::TIMEDQUESTS:
					return Tooltip::Parent::TIMEDQUESTS;
				case UIElement::Type::MINIMAP:
					return Tooltip::Parent::MINIMAP;
				case UIElement::Type::MONSTERBOOK:
					return Tooltip::Parent::MONSTERBOOK;
				case UIElement::Type::BUFFLIST:
					return Tooltip::Parent::BUFFLIST;
				case UIElement::Type::STATUSBAR:
					return Tooltip::Parent::STATUSBAR;
				case UIElement::Type::PETITEMSEARCH:
					return Tooltip::Parent::PETITEMSEARCH;
				case UIElement::Type::ITEMSEARCHRESULT:
					return Tooltip::Parent::ITEMSEARCHRESULT;
				case UIElement::Type::USERLIST:
					return Tooltip::Parent::USERLIST;
				case UIElement::Type::CHATBAR:
					return Tooltip::Parent::CHATBAR;
				default:
					return Tooltip::Parent::NONE;
			}
		}

		// The quick slots' tooltips belong to the status bar too.
		bool tooltip_belongs(Tooltip::Parent parent, UIElement::Type type)
		{
			return tooltip_parent_for(type) == parent ||
				   (parent == Tooltip::Parent::QUICKSLOT && type == UIElement::Type::STATUSBAR);
		}

		// Where tooltips are drawn against the cursor. The quick slots put the box's top-left 128 above it instead
		// (FUN_008d795a).
		const Point<int16_t> TOOLTIP_POS = Point<int16_t>(0, 22);
		const Point<int16_t> QUICKSLOT_TOOLTIP_POS = Point<int16_t>(0, -128) - tooltip::OFFSET;
		// The item search's results put it 20 right of the cursor (FUN_008fbd2e).
		const Point<int16_t> SEARCH_TOOLTIP_POS = Point<int16_t>(20, 0) - tooltip::OFFSET;
		// The Owl of Minerva's results put it 20 right of and below the cursor (FUN_008a45b9), as
		// does the User List (0x91d82f).
		const Point<int16_t> OWL_TOOLTIP_POS = Point<int16_t>(20, 20) - tooltip::OFFSET;
		// A chat log line's item goes 20 below the cursor (FUN_008f5b20).
		const Point<int16_t> CHAT_TOOLTIP_POS = Point<int16_t>(0, chat_link::BELOW) - tooltip::OFFSET;

		Point<int16_t> tooltip_pos(Tooltip::Parent parent)
		{
			switch (parent)
			{
				case Tooltip::Parent::QUICKSLOT:
					return QUICKSLOT_TOOLTIP_POS;
				case Tooltip::Parent::PETITEMSEARCH:
					return SEARCH_TOOLTIP_POS;
				case Tooltip::Parent::ITEMSEARCHRESULT:
				case Tooltip::Parent::USERLIST:
					return OWL_TOOLTIP_POS;
				case Tooltip::Parent::CHATBAR:
					return CHAT_TOOLTIP_POS;
				default:
					return TOOLTIP_POS;
			}
		}
	}

	UIStateGame::UIStateGame() : stats(Stage::get().get_player().get_stats()), dragged(nullptr)
	{
		focused = UIElement::Type::NONE;
		tooltipparent = Tooltip::Parent::NONE;
		// A shop left by a channel change or a lost connection never closed.
		Blacklist::live = false;
		ui_control::state() = {};

		const CharLook &look = Stage::get().get_player().get_look();
		const Inventory &inventory = Stage::get().get_player().get_inventory();

		emplace<UIStatusMessenger>();
		emplace<UIStatusBar>(stats);
		emplace<UIChatBar>();
		emplace<UIMiniMap>(stats);
		emplace<UIBuffList>();
		emplace<UIQuestHelper>(Stage::get().get_player().get_quests(), inventory);
		emplace<UIQuestComplete>(Stage::get().get_player().get_quests(), inventory);
		emplace<UIScriptMessage>();
		emplace<UIPartyHP>(Stage::get().get_player().get_party(), stats);
		emplace<UIShop>(look, inventory);
		emplace<UIStorage>(inventory);
		emplace<UIInviteCard>();

		VWIDTH = Constants::Constants::get().get_viewwidth();
		VHEIGHT = Constants::Constants::get().get_viewheight();
	}

	void UIStateGame::draw(float inter, Point<int16_t> cursor) const
	{
		for (auto &type : elementorder)
		{
			auto &element = elements[type];

			if (element && element->is_active())
				element->draw(inter);
		}

		// A screen effect with its own z, such as the reward roulette, plays above the windows.
		Stage::get().draw_overlay(inter);

		if (tooltip)
			tooltip->draw(cursor + tooltip_pos(tooltipparent));

		if (draggedicon)
			draggedicon->dragdraw(cursor);
	}

	void UIStateGame::update()
	{
		bool update_screen = false;
		int16_t new_width = Constants::Constants::get().get_viewwidth();
		int16_t new_height = Constants::Constants::get().get_viewheight();

		if (VWIDTH != new_width || VHEIGHT != new_height)
		{
			update_screen = true;
			VWIDTH = new_width;
			VHEIGHT = new_height;

			UI::get().remove(UIElement::Type::STATUSBAR);

			const CharStats &stats = Stage::get().get_player().get_stats();
			emplace<UIStatusBar>(stats);
		}

		for (auto &type : elementorder)
		{
			auto &element = elements[type];

			if (element && element->is_active())
			{
				element->update();

				if (update_screen)
					element->update_screen(new_width, new_height);
			}
		}

		// A window hidden by a key leaves the cursor where it was, so nothing
		// else takes its tooltip down.
		for (auto &type : elementorder)
		{
			auto &element = elements[type];

			if (tooltipparent != Tooltip::Parent::NONE && owns_tooltip(type) &&
				!(element && element->is_active()))
			{
				clear_tooltip(tooltipparent);
				break;
			}
		}
	}

	bool UIStateGame::drop_icon(const Icon &icon, Point<int16_t> pos)
	{
		if (UIElement *front = get_front(pos))
			return front->send_icon(icon, pos);
		else
			icon.drop_on_stage();

		return true;
	}

	void UIStateGame::remove_icon()
	{
		draggedicon->reset();
		draggedicon = {};
	}

	void UIStateGame::remove_cursors()
	{
		for (auto type : elementorder)
		{
			auto element = elements[type].get();

			if (element && element->is_active())
				element->remove_cursor();
		}
	}

	void UIStateGame::remove_cursor(UIElement::Type t)
	{
		for (auto type : elementorder)
		{
			auto element = elements[type].get();

			if (element && element->is_active() && element->get_type() != t)
				element->remove_cursor();
		}
	}

	void UIStateGame::doubleclick(Point<int16_t> pos)
	{
		// A window holding the mouse, such as a dialog, hears it alone, so the
		// windows behind it take no double-click (FUN_009e42b2).
		if (UIElement *focusedelement = get_focused())
			focusedelement->doubleclick(pos);
		else if (UIElement *front = get_front(pos))
			front->doubleclick(pos);
		// While a dialog is open the field takes no click (FUN_005299de).
		else if (UI::get().dialog_open())
			return;
		// Someone's shop or game balloon comes before them; a locked game
		// asks for its password (FUN_0094fbbf).
		else if (std::optional<MiniRoomHit> room = Stage::get().mini_room_at(pos))
		{
			int32_t serial = room->serial;

			if (!room->locked)
			{
				MiniRoomVisitPacket(serial).dispatch();
				return;
			}

			UI::get().emplace<UIEnterText>("Please enter the password.", [serial](const std::string &password)
			{
				MiniRoomVisitPacket(serial, password).dispatch();
			}, mini_game::PASSWORD_LIMIT);
		}
		// Then a hired merchant's; its owner is asked first (SP 0xd8b).
		else if (std::optional<MapMerchants::Hit> merchant = Stage::get().merchant_at(pos))
		{
			int32_t serial = merchant->serial;

			if (merchant->owner != Stage::get().get_player().get_oid())
			{
				MiniRoomVisitPacket(serial).dispatch();
				return;
			}

			std::string ask = "You can't sell items while managing the store.\\n"
							  "Would you like to start managing the store?";

			UI::get().emplace<UIYesNo>(ask, [serial](bool yes)
			{
				if (yes)
					MiniRoomVisitPacket(serial).dispatch();
			});
		}
		// Then a kite, which opens its message.
		else if (const MapKites::Kite *kite = Stage::get().kite_at(pos))
			emplace<UIKiteView>(kite->itemid, kite->name, kite->message);
		else if (int32_t cid = Stage::get().character_at(pos))
		{
			// v83 asks only when it may ask the server, as for an item
			// (FUN_00a23fba).
			if (Stage::get().get_player().request(CharInfoRequestPacket(cid), ExclRequest::USE_WAIT))
				emplace<UICharInfo>(cid);
		}
	}

	void UIStateGame::rightclick(Point<int16_t> pos)
	{
		// A window holding the mouse, such as the menu, hears it alone.
		if (UIElement *focusedelement = get_focused())
			focusedelement->rightclick(pos);
		else if (UIElement *front = get_front(pos))
			front->rightclick(pos);
		else if (UI::get().dialog_open())
			return;
		// Releasing the button over someone else opens the menu on them
		// (FUN_0095009a).
		else if (int32_t cid = Stage::get().other_character_at(pos))
		{
			if (Optional<OtherChar> other = Stage::get().get_chars().get_char(cid))
				emplace<UIContextMenu>(cid, other->get_name(), pos);
		}
	}

	void UIStateGame::send_key(KeyType::Id type, int32_t action, bool pressed, bool escape)
	{
		dispatch_focused_key(get_focused(), action, pressed, escape, [&]
		{
			// A locked UI takes no hotkeys and the player no keys
			// (FUN_00a07431, FUN_0094c856).
			if (ui_control::state().locked)
				return;

			switch (type)
			{
				case KeyType::Id::MENU:
					if (pressed)
					{
						switch (action)
						{
							case KeyAction::Id::EQUIPMENT:
								emplace<UIEquipInventory>(
										Stage::get().get_player().get_inventory()
								);
								break;
							case KeyAction::Id::ITEMS:
								emplace<UIItemInventory>(
										Stage::get().get_player().get_inventory()
								);
								break;
							case KeyAction::Id::STATS:
								emplace<UIStatsInfo>(
										Stage::get().get_player().get_stats()
								);
								break;
							case KeyAction::Id::SKILLS:
								emplace<UISkillBook>(
										Stage::get().get_player().get_stats(),
										Stage::get().get_player().get_skills()
								);
								break;
							case KeyAction::Id::MONSTERBOOK:
								emplace<UIMonsterBook>(
										Stage::get().get_player().get_monsterbook()
								);
								break;
							case KeyAction::Id::FRIENDS:
							case KeyAction::Id::PARTY:
							case KeyAction::Id::BOSSPARTY:
							case KeyAction::Id::GUILD:
							{
								UIUserList::Tab tab;

								switch (action)
								{
									case KeyAction::Id::FRIENDS:
										tab = UIUserList::Tab::FRIEND;
										break;
									case KeyAction::Id::PARTY:
									case KeyAction::Id::BOSSPARTY:
										tab = UIUserList::Tab::PARTY;
										break;
									case KeyAction::Id::GUILD:
										tab = UIUserList::Tab::GUILD;
										break;
								}

								auto userlist = UI::get().get_element<UIUserList>();

								if (userlist && userlist->get_tab() != tab && userlist->is_active())
								{
									userlist->change_tab(tab);
								} else
								{
									emplace<UIUserList>(tab);

									if (userlist && userlist->get_tab() != tab && userlist->is_active())
										userlist->change_tab(tab);
								}
							}
								break;
							case KeyAction::Id::WORLDMAP:
								UIWorldMap::open();
								break;
							case KeyAction::Id::PARTYSEARCH:
								UIPartySearch::toggle();
								break;
							case KeyAction::Id::FAMILY:
								FamilyActions::toggle_window();
								break;
							case KeyAction::Id::MAPLECHAT:
								UIMessenger::toggle();
								break;
							case KeyAction::Id::MINIMAP:
								if (auto minimap = UI::get().get_element<UIMiniMap>())
									minimap->send_key(action, pressed, escape);

								break;
							case KeyAction::Id::QUESTLOG:
								emplace<UIQuestLog>(
										Stage::get().get_player().get_quests()
								);
								break;
							case KeyAction::Id::MEDALS:
								emplace<UIMedals>(Stage::get().get_player().get_quests());
								break;
							case KeyAction::Id::NOTIFIER:
								if (auto helper = UI::get().get_element<UIQuestHelper>())
									helper->toggle_minimized();

								break;
							case KeyAction::Id::MENU:
								// Esc is bound to MENU by default, but it opens the
								// Game Menu rather than the ShortCut menu.
								if (auto statusbar = UI::get().get_element<UIStatusBar>())
								{
									if (escape)
										statusbar->send_key(action, pressed, escape);
									else
										statusbar->toggle_menu();
								}

								break;
							case KeyAction::Id::QUICKSLOTS:
								if (auto statusbar = UI::get().get_element<UIStatusBar>())
									statusbar->toggle_qs();

								break;
							case KeyAction::Id::CASHSHOP:
								Stage::get().enter_cash_shop();
								break;
							case KeyAction::Id::SAY:
								if (auto chatbar = UI::get().get_element<UIChatBar>())
									chatbar->set_target(UIChatBar::Target::ALL);

								break;
							case KeyAction::Id::PARTYCHAT:
								if (auto chatbar = UI::get().get_element<UIChatBar>())
									chatbar->set_target(UIChatBar::Target::PARTY);

								break;
							case KeyAction::Id::FRIENDSCHAT:
								if (auto chatbar = UI::get().get_element<UIChatBar>())
									chatbar->set_target(UIChatBar::Target::BUDDY);

								break;
							case KeyAction::Id::GUILDCHAT:
								if (auto chatbar = UI::get().get_element<UIChatBar>())
									chatbar->set_target(UIChatBar::Target::GUILD);

								break;
							case KeyAction::Id::ALLIANCECHAT:
								if (auto chatbar = UI::get().get_element<UIChatBar>())
									chatbar->set_target(UIChatBar::Target::ALLIANCE);

								break;
							case KeyAction::Id::TOSPOUSE:
								if (auto chatbar = UI::get().get_element<UIChatBar>())
									chatbar->set_target(UIChatBar::Target::SPOUSE);

								break;
							case KeyAction::Id::WHISPER:
								if (auto chatbar = UI::get().get_element<UIChatBar>())
									chatbar->ask_whisper_name();

								break;
							case KeyAction::Id::TOGGLECHAT:
								if (auto chatbar = UI::get().get_element<UIChatBar>())
									if (!chatbar->is_chatfieldopen())
										chatbar->toggle_chat();

								break;
							case KeyAction::Id::KEYBINDINGS:
							{
								auto keyconfig = UI::get().get_element<UIKeyConfig>();

								if (!keyconfig || !keyconfig->is_active())
									emplace<UIKeyConfig>(
											Stage::get().get_player().get_inventory(),
											Stage::get().get_player().get_skills()
									);
								else
									keyconfig->safe_close();

								break;
							}
							case KeyAction::Id::MAINMENU:
								if (auto statusbar = UI::get().get_element<UIStatusBar>())
									statusbar->send_key(action, pressed, escape);

								break;
							case KeyAction::Id::CHANGECHANNEL:
								emplace<UIChannel>();
								break;
							case KeyAction::Id::INTERACT_HARVEST:
								Stage::get().talk_to_nearest_npc();
								break;
							default:
								std::cout << "Action (" << action << ") not handled!" << std::endl;
								break;
						}
					}
					break;
				case KeyType::Id::ACTION:
				case KeyType::Id::FACE:
				case KeyType::Id::ITEM:
				case KeyType::Id::SKILL:
				case KeyType::Id::EFFECT:
				case KeyType::Id::MACRO:
					Stage::get().send_key(type, action, pressed);
					break;
			}
		});
	}

	Cursor::State UIStateGame::send_cursor(Cursor::State cursorstate, Point<int16_t> cursorpos)
	{
		if (draggedicon)
		{
			if (cursorstate == Cursor::State::CLICKING)
			{
				if (drop_icon(*draggedicon, cursorpos))
					remove_icon();

				return cursorstate;
			}

			return Cursor::State::GRABBING;
		} else
		{
			bool clicked = cursorstate == Cursor::State::CLICKING || cursorstate == Cursor::State::VSCROLLIDLE
				|| cursorstate == Cursor::State::HSCROLLIDLE;

			// A focused window that has closed lets go of the cursor, which
			// then reaches whatever is under it.
			if (auto focusedelement = get_focused())
			{
				clear_tooltip(Tooltip::Parent::FIELD);
				remove_cursor(focusedelement->get_type());

				return focusedelement->send_cursor(clicked, cursorpos);
			} else
			{
				if (!clicked)
				{
					dragged = nullptr;

					if (auto front = get_front(cursorpos))
					{
						UIElement::Type front_type = front->get_type();

						if (tooltipparent != Tooltip::Parent::NONE)
							if (!owns_tooltip(front_type))
								clear_tooltip(tooltipparent);

						remove_cursor(front_type);

						return front->send_cursor(clicked, cursorpos);
					} else
					{
						remove_cursors();

						return Stage::get().send_cursor(clicked, cursorpos);
					}
				} else
				{

					if (!dragged)
					{
						UIElement::Type drag_element_type = UIElement::Type::NONE;

						for (auto iter = elementorder.rbegin(); iter != elementorder.rend(); ++iter)
						{
							auto &element = elements[*iter];

							if (element && element->is_active() && element->is_in_range(cursorpos))
							{
								dragged = element.get();
								drag_element_type = *iter;
								break;
							}
						}

						if (drag_element_type != UIElement::Type::NONE)
							raise(drag_element_type);
					}

					if (dragged)
						return dragged->send_cursor(clicked, cursorpos);
					else
						return Stage::get().send_cursor(clicked, cursorpos);
				}
			}
		}
	}

	// Only the window under the cursor gets the wheel, even with nothing to scroll, and none over the field. A modal
	// window gets it wherever the cursor is (FUN_009e42b2).
	void UIStateGame::send_scroll(double yoffset, Point<int16_t> pos)
	{
		UIElement *target = get_focused();

		if (!target)
			target = get_front(pos);

		if (target)
			target->send_scroll(yoffset);
	}

	void UIStateGame::send_close()
	{
		ask_to_quit();
	}

	void UIStateGame::drag_icon(Icon *drgic)
	{
		draggedicon = drgic;
	}

	void UIStateGame::cancel_drag()
	{
		if (draggedicon)
			remove_icon();
	}

	void UIStateGame::clear_tooltip(Tooltip::Parent parent)
	{
		if (parent == tooltipparent)
		{
			eqtooltip.set_equip(Tooltip::Parent::NONE, 0);
			ittooltip.set_item(0);
			sktooltip.set_skill(0, 0, 0, 0, true);
			tetooltip.set_text("");
			matooltip.reset();
			pltooltip.reset();
			tooltip = {};
			tooltipparent = Tooltip::Parent::NONE;
		}
	}

	void UIStateGame::show_equip(Tooltip::Parent parent, int16_t slot)
	{
		eqtooltip.set_equip(parent, slot);

		if (slot)
		{
			tooltip = eqtooltip;
			tooltipparent = parent;
		}
	}

	void UIStateGame::show_stored_equip(Tooltip::Parent parent, const Equip &equip, bool cash)
	{
		eqtooltip.set_stored(parent, equip, cash);
		tooltip = eqtooltip;
		tooltipparent = parent;
	}

	void UIStateGame::show_item(Tooltip::Parent parent, int32_t itemid, int64_t expiration, int16_t flags,
								bool owned, int32_t remain_life, const std::string &owner)
	{
		ittooltip.set_item(itemid, expiration, flags, owned, remain_life, owner);

		if (itemid)
		{
			tooltip = ittooltip;
			tooltipparent = parent;
		}
	}

	void UIStateGame::show_skill(Tooltip::Parent parent, int32_t skill_id, int32_t level, int32_t masterlevel,
								 int64_t expiration, bool levels)
	{
		sktooltip.set_skill(skill_id, level, masterlevel, expiration, levels);

		if (skill_id)
		{
			tooltip = sktooltip;
			tooltipparent = parent;
		}
	}

	void UIStateGame::show_macro(Tooltip::Parent parent, size_t index)
	{
		mctooltip.set_macro(index, UI::get().get_skill_macros());
		tooltip = mctooltip;
		tooltipparent = parent;
	}

	void UIStateGame::show_text(Tooltip::Parent parent, std::string text)
	{
		tetooltip.set_text(text);

		if (!text.empty())
		{
			tooltip = tetooltip;
			tooltipparent = parent;
		}
	}

	void
	UIStateGame::show_map(Tooltip::Parent parent, std::string name, std::string description)
	{
		matooltip.set_map(parent, name, description);

		if (!name.empty())
		{
			tooltip = matooltip;
			tooltipparent = parent;
		}
	}

	void UIStateGame::show_zone(Tooltip::Parent parent, std::string title, std::string description)
	{
		pltooltip.set_zone(title, description);

		if (!pltooltip.empty())
		{
			tooltip = pltooltip;
			tooltipparent = parent;
		}
	}

	void UIStateGame::show_line(Tooltip::Parent parent, std::string text)
	{
		pltooltip.set_line(text);

		if (!pltooltip.empty())
		{
			tooltip = pltooltip;
			tooltipparent = parent;
		}
	}

	void UIStateGame::show_help(UIElement::Type owner, std::string title, std::string description,
								std::optional<Point<int16_t>> anchor, bool above)
	{
		pltooltip.set_help(title, description, anchor, above);

		if (!pltooltip.empty())
		{
			tooltip = pltooltip;
			tooltipparent = Tooltip::Parent::HELP;
			help_owner = owner;
		}
	}

	void UIStateGame::clear_help(UIElement::Type owner)
	{
		if (tooltipparent == Tooltip::Parent::HELP && help_owner == owner)
			clear_tooltip(Tooltip::Parent::HELP);
	}

	bool UIStateGame::owns_tooltip(UIElement::Type type) const
	{
		return tooltipparent == Tooltip::Parent::HELP ? help_owner == type : tooltip_belongs(tooltipparent, type);
	}

	template<class T, typename...Args>
	void UIStateGame::emplace(Args &&...args)
	{
		if (auto iter = pre_add(T::TYPE, T::TOGGLED, T::FOCUSED))
		{
			(*iter).second = std::make_unique<T>(
					std::forward<Args>(args)...
			);

			auto silent_types = {
					UIElement::Type::STATUSMESSENGER,
					UIElement::Type::STATUSBAR,
					UIElement::Type::CHATBAR,
					UIElement::Type::MINIMAP,
					UIElement::Type::BUFFLIST,
					UIElement::Type::QUESTHELPER,
					UIElement::Type::QUESTCOMPLETE,
					UIElement::Type::SCRIPTMESSAGE,
					UIElement::Type::PARTYHP,
					UIElement::Type::NPCTALK,
					UIElement::Type::IMAGESLIDES,
					UIElement::Type::AVATARSELECT,
					UIElement::Type::PETCHOOSER,
					UIElement::Type::APRESET,
					UIElement::Type::SPRESET,
					UIElement::Type::SLIDEMENU,
					UIElement::Type::QUIZ,
					UIElement::Type::SPEEDQUIZ,
					UIElement::Type::SHOP,
					UIElement::Type::STORAGE,
					UIElement::Type::STOREBANK,
					UIElement::Type::INVITECARD,
					UIElement::Type::MEMONOTICE,
					UIElement::Type::CONTEXTMENU,
					UIElement::Type::REVIVE,
					UIElement::Type::FIELDCLOCK,
					UIElement::Type::EVENTCLOCK,
					UIElement::Type::TIMEDQUESTS,
					UIElement::Type::MOBGAGE,
					UIElement::Type::ENERGYBAR,
					UIElement::Type::DOJOHUD,
					UIElement::Type::MONSTERCARNIVAL,
					UIElement::Type::COCONUT,
					UIElement::Type::AVATARMEGAPHONE,
					UIElement::Type::PARTYSEARCHICON,
					UIElement::Type::GUILDRANK
			};

			if (std::find(silent_types.begin(), silent_types.end(), T::TYPE) == silent_types.end())
			{
				if (T::TYPE == UIElement::Type::WORLDMAP)
					Sound(Sound::Name::WORLDMAPOPEN).play();
				else
					Sound(Sound::Name::MENUUP).play();

				UI::get().send_cursor(false);
			}
		}
	}

	UIState::Iterator UIStateGame::pre_add(UIElement::Type type, bool is_toggled, bool is_focused)
	{
		auto &element = elements[type];

		if (element && is_toggled)
		{
			raise(type);

			bool active = element->is_active();

			element->toggle_active();

			if (active != element->is_active())
			{
				if (element->is_active())
				{
					if (type == UIElement::Type::WORLDMAP)
						Sound(Sound::Name::WORLDMAPOPEN).play();
					else
						Sound(Sound::Name::MENUUP).play();

					UI::get().send_cursor(false);
				} else
				{
					if (type == UIElement::Type::WORLDMAP)
						Sound(Sound::Name::WORLDMAPCLOSE).play();
					else
						Sound(Sound::Name::MENUDOWN).play();

					element->remove_cursor();

					if (draggedicon)
						if (element->get_type() == icon_map[draggedicon.get()->get_type()])
							remove_icon();

					UI::get().send_cursor(false);
				}
			}

			return elements.end();
		} else
		{
			remove(type);
			raise(type);

			if (is_focused)
			{
				if (get_focused() && focused != type)
					focus_below.push_back(focused);

				focused = type;
			}

			return elements.find(type);
		}
	}

	void UIStateGame::raise(UIElement::Type type)
	{
		auto is_top = [](UIElement::Type t)
		{
			switch (t)
			{
				case UIElement::Type::INVITECARD:
				case UIElement::Type::MEMONOTICE:
				case UIElement::Type::QUESTCOMPLETE:
				case UIElement::Type::SCRIPTMESSAGE:
				case UIElement::Type::NOTICE:
				case UIElement::Type::ENTERNUMBER:
				case UIElement::Type::FRIENDGROUP:
				case UIElement::Type::NPCTALK:
				case UIElement::Type::IMAGESLIDES:
				case UIElement::Type::AVATARSELECT:
				case UIElement::Type::PETCHOOSER:
				case UIElement::Type::SLIDEMENU:
				case UIElement::Type::SHOP:
				case UIElement::Type::STORAGE:
				case UIElement::Type::STOREBANK:
				case UIElement::Type::TRADINGROOM:
				case UIElement::Type::PERSONALSHOP:
				case UIElement::Type::SHOPSELL:
				case UIElement::Type::SHOPSALES:
				case UIElement::Type::MERCHANTVISITORS:
				case UIElement::Type::MERCHANTBLACKLIST:
				case UIElement::Type::MINIGAME:
				case UIElement::Type::MINIGAMECREATE:
				case UIElement::Type::CLAIMNOTICE:
				case UIElement::Type::CLAIM:
				case UIElement::Type::GMBLOCK:
					return true;
				default:
					return false;
			}
		};

		window_layer::raise(elementorder, type, is_top);
	}

	void UIStateGame::lock_hud(bool locked)
	{
		if (!locked)
		{
			// FUN_00a13220(1) builds the HUD again, fitted to the window as
			// it is now.
			for (UIElement::Type type : hidden_hud)
			{
				if (auto &element = elements[type])
				{
					element->makeactive();
					element->update_screen(Constants::Constants::get().get_viewwidth(),
							Constants::Constants::get().get_viewheight());
				}
			}

			hidden_hud.clear();

			return;
		}

		// FUN_00a13220(0): the status bar and the rest of the HUD go, and so
		// do the windows OPEN_UI opens. Nothing typed, held or dragged in
		// them outlives them.
		if (auto chatbar = UI::get().get_element<UIChatBar>())
			chatbar->toggle_chatfield(false);

		if (auto statusbar = UI::get().get_element<UIStatusBar>())
			statusbar->remove_menus();

		UI::get().remove_textfield();
		cancel_drag();

		if (dragged)
		{
			dragged->remove_cursor();
			dragged = nullptr;
		}

		// The fading notices stay, and hide themselves (FUN_0051fb72).
		constexpr UIElement::Type HUD[] = {
				UIElement::Type::STATUSMESSENGER,
				UIElement::Type::STATUSBAR,
				UIElement::Type::CHATBAR,
				UIElement::Type::MINIMAP,
				UIElement::Type::BUFFLIST,
				UIElement::Type::QUESTHELPER,
				UIElement::Type::PARTYHP
		};
		constexpr UIElement::Type WINDOWS[] = {
				UIElement::Type::ITEMINVENTORY,
				UIElement::Type::EQUIPINVENTORY,
				UIElement::Type::STATSINFO,
				UIElement::Type::SKILLBOOK,
				UIElement::Type::KEYCONFIG,
				UIElement::Type::QUESTLOG,
				UIElement::Type::MEDALS,
				UIElement::Type::MONSTERBOOK,
				UIElement::Type::CHARINFO,
				UIElement::Type::GUILDBBS,
				UIElement::Type::ENERGYBAR,
				UIElement::Type::PARTYSEARCH,
				UIElement::Type::MAKER,
				UIElement::Type::FAMILY,
				UIElement::Type::FAMILYTREE
		};

		for (UIElement::Type type : HUD)
		{
			if (auto &element = elements[type])
			{
				if (element->is_active())
				{
					if (owns_tooltip(type))
						clear_tooltip(tooltipparent);

					element->deactivate();
					hidden_hud.push_back(type);
				}
			}
		}

		for (UIElement::Type type : WINDOWS)
			if (elements[type])
				remove(type);

		// Nothing walks on with a key held from before, or casts on from a
		// macro.
		Stage::get().release_keys();
		Stage::get().stop_macro();
	}

	void UIStateGame::remove(UIElement::Type type)
	{
		if (type == focused)
			focused = UIElement::Type::NONE;

		if (owns_tooltip(type))
			clear_tooltip(tooltipparent);

		elementorder.remove(type);

		if (auto &element = elements[type])
		{
			element->deactivate();
			element.release();
		}
	}

	UIElement *UIStateGame::get(UIElement::Type type)
	{
		return elements[type].get();
	}

	bool UIStateGame::modal_open() const
	{
		auto open = [this](UIElement::Type type)
		{
			const UIElement::UPtr &element = elements[type];

			return type != UIElement::Type::NONE && element && element->is_active();
		};

		return open(focused) || std::any_of(focus_below.begin(), focus_below.end(), open);
	}

	UIElement *UIStateGame::get_focused()
	{
		while (true)
		{
			if (UIElement *element = get(focused); element && element->is_active())
				return element;

			if (focus_below.empty())
			{
				focused = UIElement::Type::NONE;

				return nullptr;
			}

			focused = focus_below.back();
			focus_below.pop_back();
		}
	}

	UIElement *UIStateGame::get_front(std::list<UIElement::Type> types)
	{
		auto begin = elementorder.rbegin();
		auto end = elementorder.rend();

		for (auto iter = begin; iter != end; ++iter)
		{
			if (std::find(types.begin(), types.end(), *iter) != types.end())
			{
				auto &element = elements[*iter];

				if (element && element->is_active())
					return element.get();
			}
		}

		return nullptr;
	}
	// TODO: (rich) fix

	UIElement *UIStateGame::get_front(Point<int16_t> pos)
	{
		auto begin = elementorder.rbegin();
		auto end = elementorder.rend();

		for (auto iter = begin; iter != end; ++iter)
		{
			auto &element = elements[*iter];

			if (element && element->is_active() && element->is_in_range(pos))
				return element.get();
		}

		return nullptr;
	}
}
