#include "GuildHandlers.h"

#include "Helpers/GuildParser.h"

#include "../Packets/GameplayPackets.h"

#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UIChatBar.h"
#include "../../IO/UITypes/UIEnterText.h"
#include "../../IO/UITypes/UIInviteCard.h"
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
			INVITE = 0x05,
			INFO = 0x1A,
			// 0xcce
			NAME_TAKEN = 0x1C,
			// 0xccf for the master, 0x150 for the others
			CREATED = 0x20,
			// 0x15c
			ALREADY_IN_GUILD = 0x21,
			// 0x15d
			LEVEL_TOO_LOW = 0x23,
			// 0x15a for the player, 0x15b for the others
			JOINED = 0x27,
			// 0x15c
			ALREADY_JOINED = 0x28,
			// 0x160
			GUILD_FULL = 0x29,
			// 0x16a
			NOT_IN_CHANNEL = 0x2A,
			// 0x152 for the player, 0x155 for the others
			LEFT = 0x2C,
			// 0x15e
			LEAVE_NOT_IN_GUILD = 0x2D,
			// 0x15e0 for the player, 0x153 for the others
			EXPELLED = 0x2F,
			// 0x15e
			EXPEL_NOT_IN_GUILD = 0x30,
			// 0x14e
			NOT_ACCEPTING = 0x35,
			// 0xaa3
			ANOTHER_INVITATION = 0x36,
			// 0x14f
			DECLINED = 0x37,
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

		// An invitation (FUN_00a37490): a card, unless one is up already or
		// the player is busy with a window, when the client turns it down
		// itself. The card's X declines; letting it fade says nothing.
		void invited(int32_t guild_id, const std::string& inviter)
		{
			std::string self = Stage::get().get_player().get_stats().get_name();
			auto cards = UI::get().get_element<UIInviteCard>();

			if (cards && cards->showing(UIInviteCard::GUILD))
				return DenyGuildRequestPacket(DenyGuildRequestPacket::ANOTHER, inviter, self).dispatch();

			if (!cards || UI::get().modal_open())
				return DenyGuildRequestPacket(DenyGuildRequestPacket::BUSY, inviter, self).dispatch();

			cards->add(UIInviteCard::GUILD, inviter, [guild_id, inviter, self](bool yes)
			{
				if (yes)
					JoinGuildPacket(guild_id, Stage::get().get_player().get_oid()).dispatch();
				else
					DenyGuildRequestPacket(DenyGuildRequestPacket::DECLINED, inviter, self).dispatch();
			});
		}

		// A member left (LEFT) or was expelled; the player, if it was them.
		void departed(Guild& guild, int32_t cid, const std::string& name, bool expelled)
		{
			if (cid == Stage::get().get_player().get_oid())
			{
				guild.clear();
				chat_line(expelled ? "You have been expelled from the guild." : "You have quit the guild.");
			}
			else
			{
				guild.remove(cid);

				if (name.empty())
					chat_line("Deleted Chracters have been expelled.");
				else
					chat_line("'" + name + (expelled ? "' has been expelled from the guild." : "' quitted the guild."));
			}

			refresh();
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
		case Operation::INVITE:
		{
			int32_t guild_id = recv.read_int();
			invited(guild_id, recv.read_string());
			break;
		}
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
		case Operation::JOINED:
		{
			int32_t guild_id = recv.read_int();
			int32_t cid = recv.read_int();

			if (cid == player.get_oid())
			{
				// The server sends the guild when asked.
				chat_line("You have entered the guild.");
				ReloadGuildPacket().dispatch();
				break;
			}

			if (!guild.exists() || guild_id != guild.get_data().id)
				break;

			GuildMember member = GuildParser::parse_member(recv);
			member.id = cid;
			chat_line("'" + member.name + "' has joined the guild.");
			guild.add(std::move(member));
			refresh();
			break;
		}
		case Operation::ALREADY_JOINED:
			chat_line("Already joined the guild.");
			break;
		case Operation::GUILD_FULL:
			chat_line("The guild, you are trying to join, has already reached the max number of users.");
			break;
		case Operation::NOT_IN_CHANNEL:
			chat_line("The character cannot be found in the current channel.");
			break;
		case Operation::LEFT:
		case Operation::EXPELLED:
		{
			int32_t guild_id = recv.read_int();
			int32_t cid = recv.read_int();
			std::string name = recv.read_string();

			if (guild.exists() && guild_id == guild.get_data().id)
				departed(guild, cid, name, mode == Operation::EXPELLED);

			break;
		}
		case Operation::LEAVE_NOT_IN_GUILD:
		case Operation::EXPEL_NOT_IN_GUILD:
			chat_line("You are not in the guild.");
			break;
		case Operation::NOT_ACCEPTING:
			chat_line(recv.read_string() + " is currently not accepting guild invite message.");
			break;
		case Operation::ANOTHER_INVITATION:
			chat_line("'" + recv.read_string() + "' is taking care of another invitation.");
			break;
		case Operation::DECLINED:
			chat_line(recv.read_string() + " has denied your guild invitation.");
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
