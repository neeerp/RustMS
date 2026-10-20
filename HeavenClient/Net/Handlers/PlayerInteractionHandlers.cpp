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
#include "PlayerInteractionHandlers.h"
#include "MiniRoom.h"

#include "Helpers/LoginParser.h"

#include "../../Configuration.h"
#include "../../IO/UI.h"

#include "../../Gameplay/Stage.h"
#include "../../IO/UITypes/UICharInfo.h"
#include "../../IO/UITypes/UIChatBar.h"
#include "../../IO/UITypes/UINotice.h"
#include "../../IO/UITypes/UIInviteCard.h"
#include "../../IO/UITypes/UIPersonalShop.h"
#include "../../IO/UITypes/UITradingRoom.h"
#include "../Packets/MiniRoomPackets.h"

#include <iostream>
#include <optional>

namespace ms
{
	void CharInfoHandler::handle(InPacket& recv) const
	{
		Stage::get().get_player().request_answered();

		int32_t character_id = recv.read_int();
		int8_t character_level = recv.read_byte();
		int16_t character_job_id = recv.read_short();
		int16_t character_fame = recv.read_short();
		recv.skip_byte(); // character_marriage_ring

		std::string guild_name = recv.read_string();
		std::string alliance_name = recv.read_string();

		// v83 shows the mount only when this or the first pet's byte is 0 (0xa23869).
		int8_t unknown = recv.read_byte();
		int8_t pet_unique_id = recv.read_byte();
		bool shows_mount = unknown == 0 || pet_unique_id == 0;

		// The pets out, each after a non-zero byte (FUN_00903ce7).
		std::vector<UICharInfo::Pet> pets;

		while (pet_unique_id != 0)
		{
			UICharInfo::Pet pet;
			pet.itemid = recv.read_int();
			pet.name = recv.read_string();
			pet.level = static_cast<uint8_t>(recv.read_byte());
			pet.closeness = recv.read_short();
			pet.fullness = static_cast<uint8_t>(recv.read_byte());
			recv.skip_short();
			pet.wear = recv.read_int();
			pets.push_back(pet);

			pet_unique_id = recv.read_byte();
		}

		std::optional<UICharInfo::Mount> mount;

		if (recv.read_byte() != 0)
		{
			int32_t mount_level = recv.read_int();
			int32_t mount_exp = recv.read_int();
			int32_t mount_tiredness = recv.read_int();

			if (shows_mount)
				mount = UICharInfo::Mount{ mount_level, mount_exp, mount_tiredness };
		}

		int8_t wishlist_size = recv.read_byte();

		for (int8_t sn = 0; sn < wishlist_size; sn++)
			recv.skip_int(); // wishlist_item

		recv.skip_int(); // monster_book_level
		recv.skip_int(); // monster_book_card_normal
		recv.skip_int(); // monster_book_card_special
		recv.skip_int(); // monster_book_cards_total
		recv.skip_int(); // monster_book_cover

		recv.skip_int(); // medal

		int16_t medal_quests_size = recv.read_short();

		for (int16_t s = 0; s < medal_quests_size; s++)
			recv.skip_short(); // medal_quest

		// Update the character information window
		if (auto charinfo = UI::get().get_element<UICharInfo>())
		{
			charinfo->update_stats(character_id, character_job_id, character_level, character_fame, guild_name, alliance_name);
			// v83 opens the pet box, then the taming box over it.
			charinfo->update_pets(character_id, pets);

			if (mount)
				charinfo->update_mount(character_id, *mount);
		}
	}

	namespace
	{
		void fame_line(const std::string &message)
		{
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline(message, UIChatBar::LineType::SKILL);
		}

		// v83's lines put the name in '%s''s, quotes and all.
		std::string fame_named(const std::string &format, const std::string &name)
		{
			std::string line = format;
			line.replace(line.find("%s"), 2, name);

			return line;
		}
	}

	void FameResponseHandler::handle(InPacket& recv) const
	{
		switch (recv.read_byte())
		{
			case 0:
			{
				std::string target = recv.read_string();
				bool up = recv.read_bool();
				int32_t fame = recv.read_int();

				if (auto charinfo = UI::get().get_element<UICharInfo>())
					charinfo->change_fame(static_cast<int16_t>(fame));

				fame_line(fame_named(up ? "You have raised '%s''s level of fame." : "You have dropped '%s''s level of fame.", target));
				break;
			}
			case 1:
				fame_line("The user name is incorrectly entered.");
				break;
			case 2:
				fame_line("Users under level 15 are unable to toggle with fame.");
				break;
			case 3:
				fame_line("You can't raise or drop a level of fame anymore for today.");
				break;
			case 4:
				fame_line("You can't raise or drop a level of fame of that character anymore for this month.");
				break;
			case 5:
			{
				std::string giver = recv.read_string();
				bool up = recv.read_bool();
				std::string line = fame_named(up ? "'%s' have raised '%s''s level of fame." : "'%s' have dropped '%s''s level of fame.", giver);

				fame_line(fame_named(line, Stage::get().get_player().get_name()));
				break;
			}
			default:
				fame_line("The level of fame has neither been raised or dropped due to an unexpected error.");
				break;
		}
	}

	void PlayerInteractionHandler::handle(InPacket& recv) const
	{
		enum Mode : int8_t
		{
			INVITE = 2,
			INVITE_RESULT = 3,
			VISIT = 4,
			ENTER = 5,
			CHAT = 6,
			AVATAR = 9,
			LEAVE = 10,
			// The trading room's own
			PUT_ITEM = 0x0F,
			PUT_MESO = 0x10,
			CONFIRMED = 0x11,
			MESO_LIMIT = 0x15,
			// The personal shop's own
			BUY_RESULT = 0x18,
			LISTINGS = 0x19,
			SOLD = 0x1A
		};

		// The chat line of a notice by code rather than a member
		constexpr int8_t CHAT_NOTICE = 7;

		int8_t mode = recv.read_byte();
		auto room = UI::get().get_element<UITradingRoom>();
		auto shop = UI::get().get_element<UIPersonalShop>();
		bool trading = room && room->is_active();
		bool shopping = shop && shop->is_active();

		if (mode == Mode::INVITE_RESULT)
		{
			int8_t result = recv.read_byte();
			std::string name = mini_room::invite_result_named(result) ? recv.read_string() : "";
			std::string text = mini_room::invite_result_text(result, name);

			if (!text.empty())
				if (auto chatbar = UI::get().get_element<UIChatBar>())
					chatbar->send_chatline(text, UIChatBar::LineType::SKILL);

			return;
		}

		if (!trading && !shopping)
		{
			switch (mode)
			{
				case Mode::INVITE:
				{
					int8_t type = recv.read_byte();
					std::string inviter = recv.read_string();
					int32_t serial = recv.read_int();

					if (type != mini_room::TRADING_ROOM)
					{
						std::cout << "[PlayerInteractionHandler]: Unsupported invite to room type " << static_cast<int32_t>(type) << std::endl;
						break;
					}

					// The Game Option turns trade requests away unseen (FUN_0065e53b).
					if (!Setting<AllowTradeRequest>::get().load())
					{
						MiniRoomDeclinePacket(serial, MiniRoomPacket::Decline::NOT_ACCEPTING).dispatch();
						break;
					}

					if (auto cards = UI::get().get_element<UIInviteCard>())
						cards->add(UIInviteCard::TRADE, inviter, [serial](bool yes)
						{
							if (yes)
								MiniRoomVisitPacket(serial).dispatch();
							else
								MiniRoomDeclinePacket(serial, MiniRoomPacket::Decline::DECLINED).dispatch();
						});

					break;
				}
				case Mode::ENTER:
				{
					mini_room::Enter enter = mini_room::read_enter(recv, LoginParser::parse_look);

					if (enter.type == mini_room::TRADING_ROOM)
					{
						UI::get().emplace<UITradingRoom>(enter);
					}
					else if (enter.type == mini_room::PERSONAL_SHOP)
					{
						UI::get().emplace<UIPersonalShop>(enter);
					}
					else if (enter.type == 0)
					{
						std::string text = mini_room::enter_error_text(enter.error);

						if (!text.empty())
							UI::get().emplace<UIOk>(text, [](bool) {});
					}
					else
					{
						std::cout << "[PlayerInteractionHandler]: Unsupported room type " << static_cast<int32_t>(enter.type) << std::endl;
					}

					break;
				}
				default:
					break;
			}

			return;
		}

		if (mode == Mode::INVITE)
		{
			recv.skip_byte();
			recv.skip_string();

			// v83 drops invites while a room is open; turn them down so the
			// inviter hears back.
			MiniRoomDeclinePacket(recv.read_int(), MiniRoomPacket::Decline::NOT_ACCEPTING).dispatch();

			return;
		}

		if (shopping)
		{
			switch (mode)
			{
				case Mode::VISIT:
				{
					uint8_t slot = recv.read_byte();

					shop->visit(mini_room::read_member(recv, slot, LoginParser::parse_look));
					break;
				}
				case Mode::CHAT:
				{
					if (recv.read_byte() == CHAT_NOTICE)
						break;

					uint8_t slot = recv.read_byte();
					std::string line = recv.read_string();

					shop->add_chat(slot, line);
					break;
				}
				case Mode::AVATAR:
				{
					uint8_t slot = recv.read_byte();

					shop->change_look(slot, LoginParser::parse_look(recv));
					break;
				}
				case Mode::LEAVE:
				{
					uint8_t slot = recv.read_byte();

					// Only our own leaving says why (FUN_006fc731).
					if (slot == shop->get_own_slot())
						shop->leave(recv.read_byte());
					else
						shop->seat_left(slot);

					break;
				}
				case Mode::BUY_RESULT:
					shop->buy_result(recv.read_byte());
					break;
				case Mode::LISTINGS:
					shop->set_listings(mini_room::read_listings(recv));
					break;
				case Mode::SOLD:
					shop->sold(personal_shop::read_sold(recv));
					break;
				default:
					break;
			}

			return;
		}

		switch (mode)
		{
			case Mode::VISIT:
			{
				uint8_t slot = recv.read_byte();

				room->visit(mini_room::read_member(recv, slot, LoginParser::parse_look));
				break;
			}
			case Mode::CHAT:
			{
				if (recv.read_byte() == CHAT_NOTICE)
					break;

				uint8_t slot = recv.read_byte();
				std::string line = recv.read_string();

				room->add_chat(slot, line);
				break;
			}
			case Mode::AVATAR:
			{
				uint8_t slot = recv.read_byte();

				room->change_look(slot, LoginParser::parse_look(recv));
				break;
			}
			case Mode::LEAVE:
			{
				recv.skip_byte(); // slot

				room->leave(recv.read_byte());
				break;
			}
			case Mode::PUT_ITEM:
				if (auto put = mini_room::read_put_item(recv))
					room->put_item(*put);

				break;
			case Mode::PUT_MESO:
				if (auto put = mini_room::read_put_meso(recv))
					room->put_meso(*put);

				break;
			case Mode::CONFIRMED:
				room->partner_confirmed();
				break;
			case Mode::MESO_LIMIT:
				room->meso_limit();
				break;
			default:
				break;
		}
	}
}
