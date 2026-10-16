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
#include "MessagingHandlers.h"

#include "Helpers/LoginParser.h"

#include "../../Audio/Audio.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"
#include "../Packets/MessagingPackets.h"

#include "../../IO/UITypes/UIChatBar.h"
#include "../../IO/UITypes/UIMessenger.h"
#include "../../IO/UITypes/UINotice.h"
#include "../../IO/UITypes/UIStatusMessenger.h"
#include "../../Util/Misc.h"

#include <nlnx/nx.hpp>

#include <cstdlib>

namespace ms
{
	namespace
	{
		// Canonical v83 WHISPER (opcode 135) sub-modes sent by the server
		// 0x09: reply to our find (string target, byte where, then for where
		//       1 int map id, int x, int y; 2 int -1 (Cash Shop); 3 int channel (0-based))
		// 0x0A: reply to our whisper or find request (string target, byte success)
		// 0x12: incoming whisper (string sender, byte channel (0-based), byte from_admin, string message)
		// Other modes (e.g. 0x48 buddy whisper) are not handled and are ignored.
		constexpr int8_t FIND_RESULT_MODE = 0x09;
		constexpr int8_t WHISPER_RESULT_MODE = 0x0A;
		constexpr int8_t WHISPER_RECEIVE_MODE = 0x12;

		// Where a /find found its character, or nothing for an unknown answer.
		std::string build_find_chatline(const std::string& target, int8_t where, int32_t value)
		{
			switch (where)
			{
				case 1:
					return "'" + target + "' is in " + NxHelper::Map::get_map_info_by_id(value).name + ".";
				case 2:
					return "'" + target + "' is in the Cash Shop.";
				case 3:
					return "'" + target + "' is in Channel " + std::to_string(value + 1) + ".";
				default:
					return {};
			}
		}

		std::string build_whisper_chatline(const std::string& sender, bool from_admin, const std::string& message)
		{
			std::string display_name = from_admin ? "[GM]" + sender : sender;
			return "#g" + display_name + ">>" + message;
		}

		// A quest item gain or loss, worded as in the original client (period GMS footage):
		// "You have gained an item (Roger's Apple)", "You have gained items (3 Green Apple)".
		std::string build_item_gain_chatline(const std::string& name, int32_t quantity)
		{
			std::string verb = quantity < 0 ? "You have lost " : "You have gained ";
			int32_t count = std::abs(quantity);

			if (count == 1)
				return verb + "an item (" + name + ")";

			return verb + "items (" + std::to_string(count) + " " + name + ")";
		}
	}

	// Modes:
	// 0 - Item(0) or Meso(1) 
	// 3 - Exp gain
	// 4 - Fame
	// 5 - Mesos
	// 6 - Guild points
	void ShowStatusInfoHandler::handle(InPacket& recv) const
	{
		int8_t mode = recv.read_byte();

		if (mode == 0)
		{
			int8_t mode2 = recv.read_byte();

			if (mode2 == -1)
			{
				show_status(Color::Name::WHITE, "You can't get anymore items.");
			}
			else if (mode2 == 0)
			{
				int32_t itemid = recv.read_int();
				int32_t qty = recv.read_int();

				const ItemData& idata = ItemData::get(itemid);

				if (!idata.is_valid())
					return;

				std::string name = idata.get_name();

				if (name.length() > 21)
				{
					name = name.substr(0, 21);
					name += "..";
				}

				InventoryType::Id type = InventoryType::by_item_id(itemid);

				std::string tab = "";

				switch (type)
				{
				case InventoryType::Id::EQUIP:
					tab = "Eqp";
					break;
				case InventoryType::Id::USE:
					tab = "Use";
					break;
				case InventoryType::Id::SETUP:
					tab = "Setup";
					break;
				case InventoryType::Id::ETC:
					tab = "Etc";
					break;
				case InventoryType::Id::CASH:
					tab = "Cash";
					break;
				default:
					tab = "UNKNOWN";
					break;
				}

				// TODO: show_status(Color::Name::WHITE, "You have lost items in the " + tab + " tab (" + name + " " + std::to_string(qty) + ")");

				if (qty < 0)
					show_status(Color::Name::WHITE, "You have lost an item in the " + tab + " tab (" + name + ")");
				else if (qty == 1)
					show_status(Color::Name::WHITE, "You have gained an item in the " + tab + " tab (" + name + ")");
				else
					show_status(Color::Name::WHITE, "You have gained items in the " + tab + " tab (" + name + " " + std::to_string(qty) + ")");
			}
			else if (mode2 == 1)
			{
				recv.skip(1);

				int32_t gain = recv.read_int();

				if (gain < 0)
					show_status(Color::Name::WHITE, "You have lost mesos (" + std::to_string(gain) + ")");
				else
					show_status(Color::Name::WHITE, "You have gained mesos (+" + std::to_string(gain) + ")");
			}
			else
			{
				show_status(Color::Name::RED, "Mode: 0, Mode 2: " + std::to_string(mode2) + " is not handled.");
			}
		}
		else if (mode == 1)
		{
			int16_t id = recv.read_short();
			int8_t status = recv.read_byte();
			auto& quests = Stage::get().get_player().get_quests();
			if (status == 0)
				quests.remove(id);
			else if (status == 1)
				quests.add_started(id, recv.read_string());
			else if (status == 2)
				quests.add_completed(id, recv.read_long());
		}
		else if (mode == 3)
		{
			bool white = recv.read_bool();
			int32_t gain = recv.read_int();
			bool inchat = recv.read_bool();
			int32_t bonus1 = recv.read_int();

			recv.read_short();
			recv.read_int();	// bonus 2
			recv.read_bool();	// 'event or party'
			recv.read_int();	// bonus 3
			recv.read_int();	// bonus 4
			recv.read_int();	// bonus 5

			std::string message = "You have gained experience (+" + std::to_string(gain) + ")";

			if (inchat)
			{
				show_status(Color::Name::RED, "Mode: 3, inchat is not handled.");
			}
			else
			{
				show_status(white ? Color::Name::WHITE : Color::Name::YELLOW, message);

				if (bonus1 > 0)
					show_status(Color::Name::YELLOW, "+ Bonus EXP (+" + std::to_string(bonus1) + ")");
			}
		}
		else if (mode == 4)
		{
			int32_t gain = recv.read_int();
			std::string sign = (gain < 0) ? "-" : "+";

			// TODO: Lose fame?
			show_status(Color::Name::WHITE, "You have gained fame. (" + sign + std::to_string(gain) + ")");
		}
		else
		{
			show_status(Color::Name::RED, "Mode: " + std::to_string(mode) + " is not handled.");
		}
	}

	void ShowStatusInfoHandler::show_status(Color::Name color, const std::string& message) const
	{
		if (auto messenger = UI::get().get_element<UIStatusMessenger>())
			messenger->show_status(color, message);
	}

	void ServerMessageHandler::handle(InPacket& recv) const
	{
		int8_t type = recv.read_byte();
		bool servermessage = recv.inspect_bool();

		if (servermessage)
			recv.skip(1);

		std::string message = recv.read_string();

		if (type == 3)
		{
			recv.read_byte(); // channel
			recv.read_bool(); // megaphone
		}
		else if (type == 4)
		{
			UI::get().set_scrollnotice(message);
		}
		else if (type == 7)
		{
			recv.read_int(); // npcid
		}
	}

	void WeekEventMessageHandler::handle(InPacket& recv) const
	{
		recv.read_byte(); // TODO: Always 0xFF, Check this!

		std::string message = recv.read_string();

		static const std::string MAPLETIP = "[MapleTip]";

		if (message.substr(0, MAPLETIP.length()).compare("[MapleTip]"))
			message = "[Notice] " + message;

		UI::get().get_element<UIChatBar>()->send_chatline(message, UIChatBar::LineType::YELLOW);
	}

	void ChatReceivedHandler::handle(InPacket& recv) const
	{
		int32_t charid = recv.read_int();

		bool gm = recv.read_bool();

		std::string message = recv.read_string();
		bool balloon_only = recv.read_bool();

		if (auto character = Stage::get().get_character(charid))
		{
			message = character->get_name() + " : " + message;
			character->speak(message);
		}

		// As in v83 (FUN_00937138), a balloon-only line stays out of the log.
		if (balloon_only)
			return;

		if (auto chatbar = UI::get().get_element<UIChatBar>())
			chatbar->send_chatline(message, gm ? UIChatBar::LineType::GM : UIChatBar::LineType::WHITE);
	}

	void WhisperHandler::handle(InPacket& recv) const
	{
		int8_t mode = recv.read_byte();

		if (mode == WHISPER_RECEIVE_MODE)
		{
			std::string sender = recv.read_string();
			recv.read_byte(); // channel the sender is on (0-based)
			bool from_admin = recv.read_bool();
			std::string message = recv.read_string();

			if (auto chatbar = UI::get().get_element<UIChatBar>())
			{
				chatbar->send_chatline(
						build_whisper_chatline(sender, from_admin, message),
						UIChatBar::LineType::WHITE
				);
			}
		}
		else if (mode == WHISPER_RESULT_MODE)
		{
			std::string target = recv.read_string();
			bool success = recv.read_bool();

			if (!success)
			{
				if (auto chatbar = UI::get().get_element<UIChatBar>())
					chatbar->send_chatline("Unable to find '" + target + "'.", UIChatBar::LineType::RED);
			}
		}
		else if (mode == FIND_RESULT_MODE)
		{
			std::string target = recv.read_string();
			int8_t where = recv.read_byte();
			int32_t value = recv.read_int();
			std::string line = build_find_chatline(target, where, value);

			if (line.empty())
				return;

			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline(line, UIChatBar::LineType::YELLOW);
		}
	}

	void MessengerHandler::handle(InPacket& recv) const
	{
		// Canonical v83 MESSENGER modes
		enum Mode : int8_t
		{
			ADD = 0x00,
			JOIN = 0x01,
			REMOVE = 0x02,
			INVITE = 0x03,
			INVITE_RESULT = 0x04,
			DECLINED = 0x05,
			CHAT = 0x06,
			UPDATE = 0x07
		};

		int8_t mode = recv.read_byte();
		auto messenger = UI::get().get_element<UIMessenger>();

		switch (mode)
		{
			case Mode::ADD:
			case Mode::UPDATE:
			{
				uint8_t slot = recv.read_byte();
				LookEntry look = LoginParser::parse_look(recv);
				std::string name = recv.read_string();

				if (messenger)
					messenger->set_member(slot, CharLook(look), name);

				break;
			}
			case Mode::JOIN:
				UIMessenger::seat(recv.read_byte());
				break;
			case Mode::REMOVE:
				if (messenger)
					messenger->remove_member(recv.read_byte());

				break;
			case Mode::INVITE:
			{
				std::string from = recv.read_string();
				recv.skip_byte();
				int32_t messenger_id = recv.read_int();

				UI::get().emplace<UIYesNo>(
						from + " has invited you to Maple Messenger. Would you like to join?",
						[from, messenger_id](bool yes)
						{
							if (yes)
								MessengerOpenPacket(messenger_id).dispatch();
							else
								MessengerDeclinePacket(from).dispatch();
						}
				);

				break;
			}
			case Mode::INVITE_RESULT:
			{
				std::string target = recv.read_string();
				bool sent = recv.read_bool();

				if (messenger)
					messenger->add_note(sent ? "You have sent an invite to " + target + "."
											 : "Unable to find '" + target + "'.");

				break;
			}
			case Mode::DECLINED:
			{
				std::string note = recv.read_string() + " has declined your invitation.";

				if (messenger && messenger->is_active())
					messenger->add_note(note);
				else if (auto chatbar = UI::get().get_element<UIChatBar>())
					chatbar->send_chatline(note, UIChatBar::LineType::RED);

				break;
			}
			case Mode::CHAT:
			{
				std::string line = recv.read_string();

				if (messenger && messenger->is_active())
					messenger->add_chat(line);

				break;
			}
		}
	}

	void ScrollResultHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		bool success = recv.read_bool();
		bool destroyed = recv.read_bool();

		recv.read_short(); // Legendary spirit if 1

		CharEffect::Id effect;
		Messages::Type message;

		if (success)
		{
			effect = CharEffect::Id::SCROLL_SUCCESS;
			message = Messages::Type::SCROLL_SUCCESS;
		}
		else
		{
			effect = CharEffect::Id::SCROLL_FAILURE;

			if (destroyed)
				message = Messages::Type::SCROLL_DESTROYED;
			else
				message = Messages::Type::SCROLL_FAILURE;
		}

		Stage::get().show_character_effect(cid, effect);

		if (Stage::get().is_player(cid))
		{
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->display_message(message, UIChatBar::LineType::RED);

			UI::get().enable();
		}
	}

	void PlayerHintHandler::handle(InPacket& recv) const
	{
		std::string text = recv.read_string();
		int16_t width = recv.read_short();
		recv.read_short(); // height
		recv.read_byte();

		Stage::get().get_player().show_hint(text, width);
	}

	void ShowItemGainInChatHandler::handle(InPacket& recv) const
	{
		int8_t mode1 = recv.read_byte();

		if (mode1 == 3)
		{
			// Quest item gains and losses: a count, then an item id and quantity for each,
			// shown as one grey chat line per item.
			uint8_t count = recv.read_byte();

			for (uint8_t i = 0; i < count; i++)
			{
				int32_t itemid = recv.read_int();
				int32_t qty = recv.read_int();

				const ItemData& idata = ItemData::get(itemid);

				if (!idata.is_valid())
					continue;

				if (auto chatbar = UI::get().get_element<UIChatBar>())
					chatbar->send_chatline(build_item_gain_chatline(idata.get_name(), qty), UIChatBar::LineType::GREY);
			}
		}
		else if (mode1 == 8) // job change
		{
			Stage::get().get_player().show_effect_id(CharEffect::Id::JOBCHANGE);
			Sound(Sound::Name::JOBCHANGED).play();
		}
		else if (mode1 == 9) // quest complete
		{
			Stage::get().get_player().show_effect_id(CharEffect::Id::QUEST_CLEAR);
			Sound(Sound::Name::QUESTCLEAR).play();
		}
		else if (mode1 == 13) // card effect
		{
			Stage::get().get_player().show_effect_id(CharEffect::Id::MONSTER_CARD);
		}
		else if (mode1 == 18) // intro effect
		{
			recv.read_string(); // path
		}
		else if (mode1 == 23) // info: a UI image anchored to the player (tutorial hints)
		{
			std::string path = recv.read_string();
			recv.read_int();

			// The path names an NX file ("UI/tutorial.img/25"), then a node in it.
			size_t slash = path.find('/');
			nl::node file = slash == std::string::npos ? nl::node() :
				path.compare(0, slash, "UI") == 0 ? nl::nx::ui :
				path.compare(0, slash, "Effect") == 0 ? nl::nx::effect :
				path.compare(0, slash, "Map") == 0 ? nl::nx::map : nl::node();

			if (file)
				Stage::get().get_player().show_effect(Animation(file.resolve(path.substr(slash + 1))));
		}
		else // Buff effect
		{
			int32_t skillid = recv.read_int();

			// More bytes, but we don't need them.
			Stage::get().get_combat().show_player_buff(skillid);
		}
	}
}
