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

#include "../../Configuration.h"
#include "../../Audio/Audio.h"
#include "../../Character/Inventory/ScrollRules.h"
#include "../../Data/CurseData.h"
#include "../../Data/ItemData.h"
#include "../../Data/SkillData.h"
#include "../../Data/SwindleData.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"
#include "../Packets/MessagingPackets.h"
#include "StatusMessages.h"

#include "../../IO/UITypes/UIChatBar.h"
#include "../../IO/UITypes/UIMessenger.h"
#include "../../IO/UITypes/UINotice.h"
#include "../../IO/UITypes/UIStatusMessenger.h"
#include "../../Util/CurseWords.h"
#include "../../Util/Misc.h"

#include <nlnx/nx.hpp>

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

		std::string unable_to_find(const std::string& target)
		{
			return "Unable to find '" + target + "'";
		}

		// The name the server list gave a channel (0-based) of this world.
		std::string channel_name(int32_t channel)
		{
			Configuration& config = Configuration::get();
			return config.get_channel_name(config.get_worldid(), channel);
		}

		struct FindLine
		{
			std::string text;
			UIChatBar::LineType type;
		};

		// Where a /find found its character, worded as v83 does (FUN_0053228e).
		// A map is grey, anything else pink.
		FindLine build_find_chatline(const std::string& target, int8_t where, int32_t value)
		{
			switch (where)
			{
				case 1:
				{
					// Event maps, whose ids read x09 xxx xxx, aren't named.
					if (static_cast<uint32_t>(value) / 1000000 % 100 == 9)
						return { "'" + target + "' is at the event map.", UIChatBar::LineType::SKILL };

					NxHelper::Map::MapInfo map = NxHelper::Map::get_map_info_by_id(value);

					// v83 prints an empty line for a map it has no name for.
					if (map.street_name.empty() && map.name.empty())
						return {};

					std::string place = map.name.empty() ? map.street_name : map.street_name + " : " + map.name;
					return { "'" + target + "' is currently at '" + place + "'.", UIChatBar::LineType::GREY };
				}
				case 2:
					return { "'" + target + "' is at the Cash Shop.", UIChatBar::LineType::SKILL };
				case 3:
				{
					std::string channel = channel_name(value);

					if (channel.empty())
						return { unable_to_find(target), UIChatBar::LineType::SKILL };

					return { "'" + target + "' is at '" + channel + "'.", UIChatBar::LineType::SKILL };
				}
				default:
					return { unable_to_find(target), UIChatBar::LineType::SKILL };
			}
		}

		// A whisper from another channel names it.
		std::string build_whisper_chatline(const std::string& sender, int8_t channel, const std::string& message)
		{
			if (channel == Configuration::get().get_channelid())
				return sender + ">> " + message;

			return sender + "[" + channel_name(channel) + "]>> " + message;
		}

		// The inventory tab an item goes to, as the v83 client names it (FUN_005cfa86).
		std::string tab_name(int32_t itemid)
		{
			switch (itemid / 1'000'000)
			{
				case 1: return "Eqp";
				case 2: return "Use";
				case 3: return "Setup";
				case 4: return "Etc";
				case 5: return "Cash";
				default: return "";
			}
		}

		// A quest item gain or loss, worded as in the 2025 MapleRoyals v83 client
		// (FUN_009377d9): gains name the tab, losses don't. Nothing for a zero quantity.
		std::string build_item_gain_chatline(int32_t itemid, const std::string& name, int32_t quantity)
		{
			if (quantity > 1)
				return "You have gained items in the " + tab_name(itemid) + " tab (" + name + " " + std::to_string(quantity) + ")";

			if (quantity == 1)
				return "You have gained an item in the " + tab_name(itemid) + " tab (" + name + ")";

			if (quantity == -1)
				return "You have lost an item (" + name + ")";

			if (quantity < -1)
				return "You have lost items (" + name + " " + std::to_string(-quantity) + ")";

			return {};
		}
	}

	// Modes, as v83 numbers them (FUN_00a209d4):
	// 0 - Picking up an item or mesos
	// 1 - Quest progress
	// 2 - A cash item's time is up
	// 3 - EXP
	// 4 - Fame, in the chat log
	// 5 - Mesos, in the chat log
	// 6 - Guild points
	// 7 - An item's effect
	// 8 - Items whose time is up
	// 9 - A line from the server
	// 10 - A quest's extra record, which shows nothing
	// 11 - Items whose seal is gone
	// 12 - Lines from the server
	// 13 - Skills whose time is up
	// v83 ignores any other mode.
	void ShowStatusInfoHandler::handle(InPacket& recv) const
	{
		int8_t mode = recv.read_byte();

		if (mode == 0)
		{
			int8_t mode2 = recv.read_byte();

			if (mode2 == 0 || mode2 == 2)
			{
				int32_t itemid = recv.read_int();
				int32_t qty = mode2 == 0 ? recv.read_int() : 1;

				const ItemData& idata = ItemData::get(itemid);

				if (!idata.is_valid())
					return;

				// The name is cut with "..", shorter when a count follows it.
				size_t width = qty < 2 ? PICKUP_NAME_WIDTH : PICKUP_STACK_NAME_WIDTH;
				Text name(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, idata.get_name());

				if (name.width() > width)
					string_format::format_with_ellipsis(name, width);

				show_status(Color::Name::WHITE, status_messages::item_line(status_messages::tab_name(itemid), name.get_text(), qty));
			}
			else if (mode2 == 1)
			{
				// Set when part of the mesos was lost.
				bool portion = recv.read_bool();
				int32_t gain = recv.read_int();
				int16_t cafe = recv.read_short();

				show_status(Color::Name::WHITE, "You have gained mesos (+" + std::to_string(gain) + ")");

				if (portion)
					chat_line("A portion was not found after falling on the ground.", UIChatBar::LineType::YELLOW);

				if (cafe != 0)
					show_status(Color::Name::YELLOW, "Internet Cafe Meso Bonus (+" + std::to_string(cafe) + ")");
			}
			else if (mode2 == -2)
			{
				show_status(Color::Name::WHITE, "This item is unavailable for the pick-up.");
			}
			else if (mode2 == -3)
			{
				show_status(Color::Name::WHITE, "You cannot acquire any items.");
				chat_line("You cannot acquire any items because the game file has been damaged. Please try again after reinstalling the game.", UIChatBar::LineType::SKILL);
			}
			else
			{
				show_status(Color::Name::WHITE, "You can't get anymore items.");
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
			status_messages::ExpGain exp = status_messages::read_exp(recv);

			if (exp.inchat)
			{
				for (const auto& line : status_messages::chat_exp_lines(exp))
					chat_line(line);
			}
			else
			{
				for (const auto& line : status_messages::exp_lines(exp))
					show_status(line.yellow ? Color::Name::YELLOW : Color::Name::WHITE, line.text);
			}
		}
		else if (mode == 4)
		{
			chat_line(status_messages::fame_line(recv.read_int()));
		}
		else if (mode == 5)
		{
			chat_line(status_messages::meso_line(recv.read_int()));
		}
		else if (mode == 2)
		{
			const std::string& name = ItemData::get(recv.read_int()).get_name();

			if (!name.empty())
				chat_line("[" + name + "] has passed its expiration date and will be removed from your inventory.", UIChatBar::LineType::SKILL);
		}
		else if (mode == 6)
		{
			chat_line(status_messages::guild_points_line(recv.read_int()));
		}
		else if (mode == 7)
		{
			chat_line(ItemData::get(recv.read_int()).get_desc());
		}
		else if (mode == 8 || mode == 11)
		{
			// v83 stops at the first item it can't name.
			for (auto count = static_cast<uint8_t>(recv.read_byte()); count > 0; count--)
			{
				const std::string& name = ItemData::get(recv.read_int()).get_name();

				if (name.empty())
					break;

				chat_line(mode == 8 ? "The item [" + name + "] has been expired, and therefore, deleted from your inventory." : name + "'s seal has expired.", UIChatBar::LineType::SKILL);
			}
		}
		else if (mode == 9 || mode == 12)
		{
			auto count = static_cast<uint8_t>(mode == 9 ? 1 : recv.read_byte());

			for (; count > 0; count--)
			{
				std::string line = recv.read_string();

				if (!line.empty())
					chat_line(line, UIChatBar::LineType::SKILL);
			}
		}
		else if (mode == 13)
		{
			for (auto count = static_cast<uint8_t>(recv.read_byte()); count > 0; count--)
			{
				const std::string& name = SkillData::get(recv.read_int()).get_name();

				if (!name.empty())
					chat_line(name + " has disappeared as the time limit has passed.", UIChatBar::LineType::SKILL);
			}
		}
	}

	void ShowStatusInfoHandler::chat_line(const std::string& message, UIChatBar::LineType type) const
	{
		if (auto chatbar = UI::get().get_element<UIChatBar>())
			chatbar->send_chatline(message, type);
	}

	void ShowStatusInfoHandler::show_status(Color::Name color, const std::string& message) const
	{
		if (auto messenger = UI::get().get_element<UIStatusMessenger>())
			messenger->show_status(color, message);
	}

	void ServerMessageHandler::handle(InPacket& recv) const
	{
		// v83 reads a flag first only for the scrolling notice, which has no
		// message when it is off (FUN_00a22785).
		int8_t type = recv.read_byte();

		if (type == 4 && !recv.read_bool())
			return;

		std::string message = recv.read_string();
		auto chatbar = UI::get().get_element<UIChatBar>();

		switch (type)
		{
			case 0:
				// StringPool 0x37a
				if (chatbar)
					chatbar->send_chatline("[Notice] " + message, UIChatBar::LineType::BLUE);
				break;
			case 1:
				UI::get().emplace<UIOk>(message, [](bool) {});
				break;
			case 3:
				recv.read_byte(); // channel
				recv.read_bool(); // megaphone
				break;
			case 4:
				UI::get().set_scrollnotice(message);
				break;
			case 5:
				if (chatbar)
					chatbar->send_chatline(message, UIChatBar::LineType::SKILL);
				break;
			case 6:
				// An item id, linking the item to the line when not 0
				recv.read_int();

				if (chatbar)
					chatbar->send_chatline(message, UIChatBar::LineType::BLUE);
				break;
			case 7:
				recv.read_int(); // npcid
				break;
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

		auto warning = SwindleData::warning(message);

		// v83 stars out curse words in other players' chat, and shows what
		// it can't print as spaces (FUN_00937138).
		if (!Stage::get().is_player(charid))
			message = CurseData::filter(message);

		message = CurseWords::printable(message);

		if (auto character = Stage::get().get_character(charid))
		{
			message = character->get_name() + " : " + message;
			character->speak(message);
		}

		// As in v83 (FUN_00937138), a balloon-only line stays out of the log.
		if (balloon_only)
			return;

		if (auto chatbar = UI::get().get_element<UIChatBar>())
		{
			chatbar->send_chatline(message, gm ? UIChatBar::LineType::GM : UIChatBar::LineType::WHITE);

			// v83 follows the line with a scam warning (FUN_00937138).
			if (warning)
				chatbar->send_chatline(*warning, UIChatBar::LineType::YELLOW);
		}
	}

	void WhisperHandler::handle(InPacket& recv) const
	{
		int8_t mode = recv.read_byte();

		auto chatbar = UI::get().get_element<UIChatBar>();

		if (!chatbar)
			return;

		if (mode == WHISPER_RECEIVE_MODE)
		{
			std::string sender = recv.read_string();
			int8_t channel = recv.read_byte();
			// From an admin: v83 shows it even with whispers blocked, which
			// HeavenClient can't do.
			recv.read_bool();
			std::string message = CurseData::filter(recv.read_string());

			// Print the warning before the whisper.
			if (auto warning = SwindleData::warning(message))
				chatbar->send_chatline(*warning, UIChatBar::LineType::YELLOW);

			chatbar->send_chatline(build_whisper_chatline(sender, channel, message), UIChatBar::LineType::WHISPER);
		}
		else if (mode == WHISPER_RESULT_MODE)
		{
			std::string target = recv.read_string();
			bool success = recv.read_bool();
			std::string message = chatbar->whisper_answered();

			// v83 shows the whisper once the server has answered, delivered or not.
			if (!message.empty())
				chatbar->send_chatline(target + "<< " + message, UIChatBar::LineType::WHISPER);

			if (!success)
				chatbar->send_chatline(unable_to_find(target), UIChatBar::LineType::SKILL);
		}
		else if (mode == FIND_RESULT_MODE)
		{
			std::string target = recv.read_string();
			int8_t where = recv.read_byte();
			int32_t value = recv.read_int();
			FindLine line = build_find_chatline(target, where, value);

			chatbar->whisper_answered();

			if (!line.text.empty())
				chatbar->send_chatline(line.text, line.type);
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
			// v83 writes both results into an open messenger's log and drops
			// them otherwise (FUN_00851293, FUN_00851387).
			case Mode::INVITE_RESULT:
			{
				std::string target = recv.read_string();
				bool sent = recv.read_bool();

				if (messenger && messenger->is_active())
					messenger->add_note(sent ? "- You have sent the invite to '" + target + "'."
											 : "- '" + target + "' can't be found.");

				break;
			}
			case Mode::DECLINED:
			{
				std::string target = recv.read_string();
				bool blocking = recv.read_bool();

				if (messenger && messenger->is_active())
					messenger->add_note(blocking ? "'" + target + "' is currently not accepting chat."
												 : "'" + target + "' denied the request.");

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

	// v83 FUN_0093354d. Everyone sees the glow and hears the sound; the
	// scroller also reads the outcome in the chat log. The Legendary Spirit's
	// result (legendary = 1) belongs to its dialog, which HeavenClient lacks.
	// The inventory operation, not this, answers the scroller's request.
	void ScrollResultHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		bool success = recv.read_byte() != 0;
		bool cursed = recv.read_bool();
		bool legendary = recv.read_bool();
		bool whitescroll = recv.read_bool();
		bool own = Stage::get().is_player(cid);

		if (own && legendary)
			return;

		Sound(success ? Sound::Name::ENCHANTSUCCESS : Sound::Name::ENCHANTFAILURE).play();
		Stage::get().show_character_effect(cid, success ? CharEffect::Id::SCROLL_SUCCESS : CharEffect::Id::SCROLL_FAILURE);

		if (own)
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline(scroll_rules::result_text(success, cursed, whitescroll), UIChatBar::LineType::SKILL);
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
			// shown as one grey chat line per item (chat type 7).
			uint8_t count = recv.read_byte();

			for (uint8_t i = 0; i < count; i++)
			{
				int32_t itemid = recv.read_int();
				int32_t qty = recv.read_int();

				const ItemData& idata = ItemData::get(itemid);

				if (!idata.is_valid())
					continue;

				std::string line = build_item_gain_chatline(itemid, idata.get_name(), qty);

				if (line.empty())
					continue;

				if (auto chatbar = UI::get().get_element<UIChatBar>())
					chatbar->send_chatline(line, UIChatBar::LineType::GREY);
			}

			// A count of 0 is a quest message instead: a light blue status line,
			// with Effect/Quest.img/N on the player when N isn't negative.
			if (count == 0)
			{
				std::string message = recv.read_string();
				int32_t effect = recv.read_int();

				if (effect >= 0)
					if (nl::node animation = nl::nx::effect["Quest.img"][std::to_string(effect)])
						Stage::get().get_player().show_effect(Animation(animation));

				if (!message.empty())
					if (auto messenger = UI::get().get_element<UIStatusMessenger>())
						messenger->show_status(Color::Name::PICTONBLUE, message);
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
		else if (mode1 == 2) // a party member's skill reached the player
		{
			int32_t skillid = recv.read_int();
			recv.read_byte(); // level

			Stage::get().get_combat().show_player_affected(skillid);
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
