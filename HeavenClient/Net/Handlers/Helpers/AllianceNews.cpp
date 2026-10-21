#include "AllianceNews.h"

#include "GuildParser.h"

#include <array>
#include <utility>

namespace ms
{
	namespace AllianceNews
	{
		namespace
		{
			// The alliance the player's guild is in, which most modes check
			// their id against.
			bool ours(const Guild& own, int32_t alliance_id)
			{
				return alliance_id == own.get_data().alliance_id;
			}

			bool own_guild(const Guild& own, int32_t guild_id)
			{
				return own.exists() && guild_id == own.get_data().id;
			}
		}

		Outcome apply(uint8_t mode, InPacket& recv, Alliance& alliance, Guild& own, int32_t self)
		{
			Outcome outcome;

			switch (mode)
			{
			case Mode::ALLIANCE:
				alliance.clear();

				if (recv.read_bool())
					alliance.set(GuildParser::parse_alliance(recv));

				break;
			case Mode::GUILDS:
			{
				int32_t count = recv.read_int();

				for (int32_t i = 0; i < count; i++)
					alliance.store(own, GuildParser::parse_guild(recv));

				break;
			}
			case Mode::MEMBER_ONLINE:
			{
				recv.skip_int(); // alliance id
				int32_t guild_id = recv.read_int();
				int32_t cid = recv.read_int();
				bool online = recv.read_bool();

				if (cid == self)
					break;

				// The player's guildmates aren't in the list; the guild's
				// packets bring theirs.
				const GuildMember* member = alliance.set_online(guild_id, cid, online);

				if (member && online)
					outcome.logged_in = member->name;

				break;
			}
			case Mode::INFO:
			{
				alliance.clear();
				alliance.set(GuildParser::parse_alliance(recv));

				// A GUILDDATA for each id, with no count of its own.
				for (size_t i = 0; i < alliance.get_data().guild_ids.size(); i++)
					alliance.store(own, GuildParser::parse_guild(recv));

				break;
			}
			case Mode::GUILD_LEFT:
			{
				AllianceData data = GuildParser::parse_alliance(recv);
				int32_t guild_id = recv.read_int();

				if (data.id != alliance.get_data().id)
					break;

				if (own_guild(own, guild_id))
				{
					alliance.clear();
					alliance.clear_guilds();
					own.update(GuildParser::parse_guild(recv));
					outcome.chat = recv.read_bool() ? "You are dismissed from the guild alliance."
													: "You leaved the guild alliance.";
				}
				else
				{
					// The rest goes unread.
					alliance.set(std::move(data));
					alliance.erase(guild_id);
				}

				break;
			}
			case Mode::GUILD_JOINED:
			{
				alliance.set(GuildParser::parse_alliance(recv));
				int32_t guild_id = recv.read_int();
				GuildData guild = GuildParser::parse_guild(recv);

				if (own_guild(own, guild_id))
				{
					own.update(std::move(guild));
					outcome.chat = "You joined [" + alliance.get_data().name + "] guild alliance.";
					outcome.ask_info = true;
				}
				else
				{
					outcome.chat = "[" + guild.name + "] guild joined in your guild alliance.";
					alliance.store(own, std::move(guild));
				}

				break;
			}
			case Mode::CHANGED:
			{
				AllianceData data = GuildParser::parse_alliance(recv);

				if (data.id == alliance.get_data().id)
					alliance.set(std::move(data));

				break;
			}
			case Mode::MEMBER_LEVEL_JOB:
			{
				int32_t alliance_id = recv.read_int();
				int32_t guild_id = recv.read_int();
				int32_t cid = recv.read_int();
				int32_t level = recv.read_int();
				int32_t job = recv.read_int();

				// The guild's own packets bring the player's guildmates'.
				if (ours(own, alliance_id) && guild_id != own.get_data().id)
					alliance.set_level_job(guild_id, cid, level, job);

				break;
			}
			case Mode::MASTER:
			{
				int32_t alliance_id = recv.read_int();
				int32_t old_cid = recv.read_int();
				int32_t new_cid = recv.read_int();

				if (ours(own, alliance_id))
					alliance.set_master(own, old_cid, new_cid);

				break;
			}
			case Mode::TITLES:
			{
				int32_t alliance_id = recv.read_int();
				std::array<std::string, 5> titles;

				for (auto& title : titles)
					title = recv.read_string();

				if (ours(own, alliance_id))
					alliance.set_titles(std::move(titles));

				break;
			}
			case Mode::RANK:
			{
				// No alliance id.
				int32_t cid = recv.read_int();
				int32_t rank = static_cast<uint8_t>(recv.read_byte());
				alliance.set_rank(own, cid, rank);
				break;
			}
			case Mode::NOTICE:
			{
				int32_t alliance_id = recv.read_int();
				std::string notice = recv.read_string();

				// The tab shows it; no chat line.
				if (ours(own, alliance_id))
					alliance.set_notice(std::move(notice));

				break;
			}
			case Mode::DISBANDED:
				if (ours(own, recv.read_int()))
				{
					alliance.clear_guilds();
					alliance.clear();
					own.leave_alliance();
				}

				break;
			case Mode::GUILD_CHANGED:
			{
				int32_t alliance_id = recv.read_int();
				int32_t guild_id = recv.read_int();
				GuildData guild = GuildParser::parse_guild(recv);

				if (!ours(own, alliance_id))
					break;

				if (own_guild(own, guild_id))
					own.update(std::move(guild));
				else
					alliance.store(own, std::move(guild));

				break;
			}
			default:
				break;
			}

			return outcome;
		}
	}
}
