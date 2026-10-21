#include "GuildHandlers.h"

#include "Helpers/AllianceNews.h"
#include "Helpers/GuildBbsParser.h"
#include "Helpers/GuildParser.h"

#include "../Packets/GameplayPackets.h"

#include "../../Character/Job.h"
#include "../../Configuration.h"
#include "../../Data/CurseData.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UIChatBar.h"
#include "../../IO/UITypes/UIEnterText.h"
#include "../../IO/UITypes/UIGuildBBS.h"
#include "../../IO/UITypes/UIGuildMarkMaker.h"
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
			// Lea's emblem maker, which only the master's client opens
			EMBLEM_MAKER = 0x11,
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
			// 0xcd0 for the master, 0x159 for the others
			DISBANDED = 0x32,
			// 0xcd4
			DISBAND_FAILED = 0x34,
			// 0x14e
			NOT_ACCEPTING = 0x35,
			// 0xaa3
			ANOTHER_INVITATION = 0x36,
			// 0x14f
			DECLINED = 0x37,
			// 0x167
			ADMIN = 0x38,
			// 0xcd1 for the master
			CAPACITY_CHANGED = 0x3A,
			// 0xcd5
			GROW_FAILED = 0x3B,
			// Silent
			MEMBER_LEVEL_JOB = 0x3C,
			// 0x2ca and 0x2cb on the card when they log in
			MEMBER_ONLINE = 0x3D,
			// 0xce0 for the master
			TITLES_CHANGED = 0x3E,
			// 0xcdf
			RANK_CHANGED = 0x40,
			// 0xd0c for the master
			EMBLEM_CHANGED = 0x42,
			// 0xd15
			NOTICE_CHANGED = 0x44
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
		// the client refuses asks again.
		void prompt_name()
		{
			UI::get().emplace<UIEnterText>(
				"Please type the name of the guild.",
				[](const std::string& name)
				{
					Guild::Name check = Guild::check_name(name);

					if (check == Guild::Name::OK && CurseData::in_name(name))
						check = Guild::Name::NOT_ALLOWED;

					switch (check)
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

		// The player's tag and the stat window.
		void show_guild()
		{
			Stage::get().get_player().show_guild();

			if (auto stats = UI::get().get_element<UIStatsInfo>())
				stats->update_all_stats();
		}

		// After the guild changes: the alliance too.
		void refresh()
		{
			Player& player = Stage::get().get_player();
			player.get_alliance().follow(player.get_guild());
			show_guild();
		}

		// An invitation (FUN_00a37490): a card, unless one is up already, the
		// Game Option blocks it or the player is busy with a window, when the
		// client turns it down itself. The card's X declines; letting it fade
		// says nothing.
		void invited(int32_t guild_id, const std::string& inviter)
		{
			std::string self = Stage::get().get_player().get_stats().get_name();
			auto cards = UI::get().get_element<UIInviteCard>();

			if (cards && cards->showing(UIInviteCard::GUILD))
				return DenyGuildRequestPacket(DenyGuildRequestPacket::ANOTHER, inviter, self).dispatch();

			if (!cards || !Setting<AllowGuildInvite>::get().load() || UI::get().modal_open())
				return DenyGuildRequestPacket(DenyGuildRequestPacket::BUSY, inviter, self).dispatch();

			cards->add(UIInviteCard::GUILD, inviter, [guild_id, inviter, self](bool yes)
			{
				if (yes)
					JoinGuildPacket(guild_id, Stage::get().get_player().get_oid()).dispatch();
				else
					DenyGuildRequestPacket(DenyGuildRequestPacket::DECLINED, inviter, self).dispatch();
			});
		}

		// An alliance's invitation (FUN_00a39f4e): a card, unless the Game
		// Option blocks it or one is up already, when the client turns it
		// down itself. Its O sends the player's guild's name; letting it fade
		// says nothing.
		void invited_to_alliance(int32_t alliance_id, const std::string& inviter)
		{
			std::string guild = Stage::get().get_player().get_guild().get_data().name;
			auto cards = UI::get().get_element<UIInviteCard>();

			if (!Setting<AllowAllianceInvite>::get().load())
				return DenyAllianceRequestPacket(DenyAllianceRequestPacket::BLOCKING, inviter, guild).dispatch();

			if (!cards || cards->showing(UIInviteCard::ALLIANCE))
				return DenyAllianceRequestPacket(DenyAllianceRequestPacket::ANOTHER, inviter, guild).dispatch();

			cards->add(UIInviteCard::ALLIANCE, inviter, [alliance_id, inviter, guild](bool yes)
			{
				if (yes)
					JoinAlliancePacket(alliance_id, guild).dispatch();
				else
					DenyAllianceRequestPacket(DenyAllianceRequestPacket::DECLINED, inviter, guild).dispatch();
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

	void GuildBbsHandler::handle(InPacket& recv) const
	{
		auto board = UI::get().get_element<UIGuildBBS>();

		if (!board || !board->is_active())
			return;

		switch (recv.read_byte())
		{
		case 6:
			board->show_list(GuildBbsParser::parse_list(recv));
			break;
		case 7:
			board->show_thread(GuildBbsParser::parse_thread(recv));
			break;
		case 8:
			board->thread_missing();
			break;
		default:
			break;
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
		case Operation::EMBLEM_MAKER:
			// The others' clients ignore it (FUN_00a37490).
			if (guild.is_master(player.get_oid()))
				UI::get().emplace<UIGuildMarkMaker>(guild.get_data().emblem);

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

			// The alliance comes when asked (FUN_00a37490).
			if (guild.get_data().alliance_id != 0)
				AllianceInfoPacket().dispatch();

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
		case Operation::DISBANDED:
		{
			int32_t guild_id = recv.read_int();

			if (!guild.exists() || guild_id != guild.get_data().id)
				break;

			// Asked before the guild, and with it its master, is gone.
			bool master = guild.is_master(player.get_oid());
			guild.clear();

			if (master)
				heracle_says("The guild has been disbanded..  Please come back to me when you want to make a guild, again..",
							 nullptr);
			else
				chat_line("The guild has been disbanded.");

			refresh();
			break;
		}
		case Operation::DISBAND_FAILED:
			heracle_says("The problem has happened during the process of disbanding the guild... Plese try again later...",
						 nullptr);
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
		case Operation::CAPACITY_CHANGED:
		{
			int32_t guild_id = recv.read_int();
			int32_t capacity = static_cast<uint8_t>(recv.read_byte());

			if (!guild.exists() || guild_id != guild.get_data().id)
				break;

			// Everyone's client keeps it, for the invite check; only the
			// master, who paid, hears it.
			guild.set_capacity(capacity);

			if (guild.is_master(player.get_oid()))
				heracle_says("Congratulation~  The number of guld members has now increased to " +
							 std::to_string(capacity) + " ...  Please come back to me whenever you need to..",
							 nullptr);

			refresh();
			break;
		}
		case Operation::GROW_FAILED:
			heracle_says("The problem has happened during the process of increasing the guild... Plese try again later...",
						 nullptr);
			break;
		case Operation::MEMBER_LEVEL_JOB:
		{
			int32_t guild_id = recv.read_int();
			int32_t cid = recv.read_int();
			int32_t level = recv.read_int();
			int32_t job = recv.read_int();

			if (guild.exists() && guild_id == guild.get_data().id)
				guild.set_level_job(cid, level, job);

			break;
		}
		case Operation::MEMBER_ONLINE:
		{
			int32_t guild_id = recv.read_int();
			int32_t cid = recv.read_int();
			bool online = recv.read_bool();

			if (!guild.exists() || guild_id != guild.get_data().id || !guild.set_online(cid, online))
				break;

			// The buddy list's card, for anyone in the guild; the client
			// checks neither the old flag nor who it is.
			if (online)
				if (auto cards = UI::get().get_element<UIInviteCard>())
					cards->notify(guild.find(cid)->name, false);

			break;
		}
		case Operation::TITLES_CHANGED:
		{
			int32_t guild_id = recv.read_int();
			std::array<std::string, 5> titles;

			for (auto& title : titles)
				title = recv.read_string();

			if (!guild.exists() || guild_id != guild.get_data().id)
				break;

			guild.set_titles(std::move(titles));

			// Only the master hears it, who asked.
			if (guild.is_master(player.get_oid()))
				UI::get().emplace<UIOk>("It is saved.", [](bool) {});

			refresh();
			break;
		}
		case Operation::RANK_CHANGED:
		{
			int32_t guild_id = recv.read_int();
			int32_t cid = recv.read_int();
			int32_t rank = static_cast<uint8_t>(recv.read_byte());

			if (!guild.exists() || guild_id != guild.get_data().id)
				break;

			if (guild.set_rank(cid, rank))
				chat_line("[" + guild.find(cid)->name + "]'s position has been changed to [" + guild.title(rank) + "].");

			refresh();
			break;
		}
		case Operation::EMBLEM_CHANGED:
		{
			int32_t guild_id = recv.read_int();
			GuildEmblem emblem = GuildParser::parse_emblem(recv);

			if (!guild.exists() || guild_id != guild.get_data().id)
				break;

			guild.set_emblem(emblem);

			// Only the master hears it, who made it. The player's own tag
			// changes here, from the guild; 203 is for the others.
			if (guild.is_master(player.get_oid()))
				UI::get().emplace<UIOk>("It is completed.", [](bool) {});

			refresh();
			break;
		}
		case Operation::NOTICE_CHANGED:
		{
			int32_t guild_id = recv.read_int();
			std::string notice = recv.read_string();

			if (!guild.exists() || guild_id != guild.get_data().id)
				break;

			// An empty one clears the notice quietly.
			if (!notice.empty())
				chat_line("* Guild Notice : " + notice);

			guild.set_notice(std::move(notice));
			refresh();
			break;
		}
		default:
			chat_line("The guild request has not been accepted, due to unknown reason.");
			break;
		}
	}

	void AllianceOperationHandler::handle(InPacket& recv) const
	{
		Player& player = Stage::get().get_player();
		uint8_t mode = static_cast<uint8_t>(recv.read_byte());

		if (mode == AllianceNews::Mode::INVITE)
		{
			int32_t alliance_id = recv.read_int();
			invited_to_alliance(alliance_id, recv.read_string());
			return;
		}

		AllianceNews::Outcome outcome = AllianceNews::apply(mode, recv, player.get_alliance(), player.get_guild(), player.get_oid());

		if (!outcome.chat.empty())
			chat_line(outcome.chat);

		// The buddy list's card, whatever the Game Option says.
		if (!outcome.logged_in.empty())
			if (auto cards = UI::get().get_element<UIInviteCard>())
				cards->notify(outcome.logged_in, false);

		if (outcome.ask_info)
			AllianceInfoPacket().dispatch();

		show_guild();
	}

	void NotifyLevelUpHandler::handle(InPacket& recv) const
	{
		uint8_t type = static_cast<uint8_t>(recv.read_byte());
		int32_t level = recv.read_int();
		std::string name = recv.read_string();

		// 0x1245; type 0 also tells a family junior of a lower level that
		// their reps are halved (0x1246), which needs families.
		chat_line("<" + std::string(type == 2 ? "Guild" : "Family") + "> " + name + " has reached Lv. " + std::to_string(level) + ".");
	}

	void NotifyJobChangeHandler::handle(InPacket& recv) const
	{
		uint8_t type = static_cast<uint8_t>(recv.read_byte());
		int32_t job = recv.read_int();
		std::string name = recv.read_string();

		if (type > 1)
			return;

		// 0x148c has no '>' after the group: the server sends it with the
		// name.
		chat_line("<" + std::string(type == 0 ? "Guild" : "Family") + name + " has advanced to a(an) " + Job(static_cast<uint16_t>(job)).get_name() + ".");
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
