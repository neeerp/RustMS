#include "GuildParser.h"

namespace ms
{
	namespace GuildParser
	{
		namespace
		{
			constexpr uint16_t NAME_LENGTH = 13;
		}

		GuildEmblem parse_emblem(InPacket& recv)
		{
			GuildEmblem emblem;
			emblem.background = static_cast<uint16_t>(recv.read_short());
			emblem.background_color = static_cast<uint8_t>(recv.read_byte());
			emblem.mark = static_cast<uint16_t>(recv.read_short());
			emblem.mark_color = static_cast<uint8_t>(recv.read_byte());

			return emblem;
		}

		GuildMember parse_member(InPacket& recv)
		{
			GuildMember member;
			member.name = recv.read_padded_string(NAME_LENGTH);
			member.job = recv.read_int();
			member.level = recv.read_int();
			member.rank = recv.read_int();
			member.online = recv.read_int() != 0;
			recv.skip_int(); // contribution
			member.alliance_rank = recv.read_int();

			return member;
		}

		GuildData parse_guild(InPacket& recv)
		{
			GuildData guild;
			guild.id = recv.read_int();
			guild.name = recv.read_string();

			for (auto& title : guild.titles)
				title = recv.read_string();

			uint8_t count = static_cast<uint8_t>(recv.read_byte());
			guild.members.resize(count);

			for (auto& member : guild.members)
				member.id = recv.read_int();

			for (auto& member : guild.members)
			{
				int32_t id = member.id;
				member = parse_member(recv);
				member.id = id;
			}

			guild.capacity = recv.read_int();
			guild.emblem = parse_emblem(recv);
			guild.notice = recv.read_string();
			guild.points = recv.read_int();
			guild.alliance_id = recv.read_int();

			return guild;
		}

		AllianceData parse_alliance(InPacket& recv)
		{
			AllianceData alliance;
			alliance.id = recv.read_int();
			alliance.name = recv.read_string();

			for (auto& title : alliance.titles)
				title = recv.read_string();

			uint8_t count = static_cast<uint8_t>(recv.read_byte());
			alliance.guild_ids.resize(count);

			for (auto& id : alliance.guild_ids)
				id = recv.read_int();

			alliance.capacity = recv.read_int();
			alliance.notice = recv.read_string();

			return alliance;
		}

		std::vector<GuildStanding> parse_ranking(InPacket& recv)
		{
			int32_t count = recv.read_int();
			std::vector<GuildStanding> guilds;

			for (int32_t i = 0; i < count && recv.available(); i++)
			{
				GuildStanding guild;
				guild.name = recv.read_string();
				guild.points = recv.read_int();
				guild.emblem.mark = static_cast<uint16_t>(recv.read_int());
				guild.emblem.mark_color = static_cast<uint8_t>(recv.read_int());
				guild.emblem.background = static_cast<uint16_t>(recv.read_int());
				guild.emblem.background_color = static_cast<uint8_t>(recv.read_int());
				guilds.push_back(std::move(guild));
			}

			return guilds;
		}
	}
}
