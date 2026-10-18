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
			recv.skip_int(); // alliance rank

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
	}
}
