#include "GuildHandlers.h"

#include "Helpers/GuildParser.h"

#include "../Packets/GameplayPackets.h"

#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UIChatBar.h"
#include "../../IO/UITypes/UIEnterText.h"
#include "../../IO/UITypes/UINpcTalk.h"
#include "../../IO/UITypes/UIStatsInfo.h"

namespace ms
{
	namespace
	{
		// The modes of FUN_00a37490 the server uses so far. Each takes its
		// message from the StringPool entry named after it.
		enum Operation : uint8_t
		{
			NAME_PROMPT = 0x01,
			INFO = 0x1A,
			// 0xcce
			NAME_TAKEN = 0x1C,
			// 0xccf for the master, 0x150 for the others
			CREATED = 0x20,
			// 0x15c
			ALREADY_IN_GUILD = 0x21,
			// 0x15d
			LEVEL_TOO_LOW = 0x23,
			// 0x167
			ADMIN = 0x38
		};

		// Heracle, at the guild headquarters, says the client's own guild
		// lines (FUN_00a393d3).
		constexpr int32_t HERACLE = 2010007;

		// A Heracle say box with only OK, which the client answers itself.
		void heracle_says(const std::string& text, std::function<void()> then)
		{
			UI::get().emplace<UINpcTalk>();
			UI::get().enable();

			if (auto talk = UI::get().get_element<UINpcTalk>())
			{
				talk->change_text(HERACLE, 0, 0, 0, text);
				talk->set_local_responder([then](int8_t, int32_t)
				{
					if (then)
						then();
				});
			}
		}

		void chat_line(const std::string& text)
		{
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline(text, UIChatBar::LineType::SKILL);
		}

		// The new guild's name (FUN_005305ae). Cancel sends nothing; a name
		// the client refuses asks again. The client also runs its forbidden
		// word list here, which HeavenClient doesn't have.
		void prompt_name()
		{
			UI::get().emplace<UIEnterText>(
				"Please type the name of the guild.",
				[](const std::string& name)
				{
					switch (Guild::check_name(name))
					{
					case Guild::Name::OK:
						CreateGuildPacket(name).dispatch();
						break;
					case Guild::Name::TOO_SHORT:
						UI::get().emplace<UIOk>(
							"Please enter at least " + std::to_string(Guild::MIN_NAME) + " characters.",
							[](bool) { prompt_name(); }
						);
						break;
					case Guild::Name::NOT_ALLOWED:
						UI::get().emplace<UIOk>(
							"It cannot be used. Please check it again.",
							[](bool) { prompt_name(); }
						);
						break;
					}
				},
				Guild::MAX_NAME
			);
		}

		// After the guild changes: the player's tag and the stat window.
		void refresh()
		{
			Stage::get().get_player().show_guild();

			if (auto stats = UI::get().get_element<UIStatsInfo>())
				stats->update_all_stats();
		}
	}

	void GuildOperationHandler::handle(InPacket& recv) const
	{
		Player& player = Stage::get().get_player();
		Guild& guild = player.get_guild();
		uint8_t mode = static_cast<uint8_t>(recv.read_byte());

		switch (mode)
		{
		case Operation::NAME_PROMPT:
			prompt_name();
			break;
		case Operation::INFO:
			guild.clear();

			if (recv.read_bool())
				guild.update(GuildParser::parse_guild(recv));

			refresh();
			break;
		case Operation::NAME_TAKEN:
			heracle_says("The name is already in use... Please try other ones....", prompt_name);
			break;
		case Operation::CREATED:
		{
			guild.update(GuildParser::parse_guild(recv));
			const std::string& name = guild.get_data().name;

			if (guild.is_master(player.get_oid()))
				heracle_says("Congratulation~  " + name +
							 " guild has been registered to our guild office...  I wish the best luck for you guys~",
							 nullptr);
			else
				chat_line(name + " You have joined a guild.");

			refresh();
			break;
		}
		case Operation::ALREADY_IN_GUILD:
			chat_line("Already joined the guild.");
			break;
		case Operation::LEVEL_TOO_LOW:
			chat_line("You cannot make a guild, due to the limitation of minimum level requirement.");
			break;
		case Operation::ADMIN:
			chat_line("Admin cannot make a guild.");
			break;
		default:
			chat_line("The guild request has not been accepted, due to unknown reason.");
			break;
		}
	}

	void GuildNameChangedHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		std::string name = recv.read_string();

		if (auto otherchar = Stage::get().get_chars().get_char(cid))
			otherchar->set_guild_name(name);
	}

	void GuildMarkChangedHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		GuildEmblem emblem = GuildParser::parse_emblem(recv);

		if (auto otherchar = Stage::get().get_chars().get_char(cid))
			otherchar->set_guild_emblem(emblem);
	}
}
