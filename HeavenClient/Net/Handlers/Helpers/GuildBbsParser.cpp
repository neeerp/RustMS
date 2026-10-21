#include "GuildBbsParser.h"

namespace ms
{
	namespace GuildBbsParser
	{
		namespace
		{
			GuildBbs::Entry parse_entry(InPacket& recv)
			{
				GuildBbs::Entry entry;
				entry.id = recv.read_int();
				entry.poster = recv.read_int();
				entry.title = recv.read_string();
				entry.time = recv.read_long();
				entry.icon = recv.read_int();
				entry.replies = recv.read_int();

				return entry;
			}
		}

		GuildBbs::List parse_list(InPacket& recv)
		{
			GuildBbs::List list;
			list.has_notice = recv.read_byte() != 0;

			if (list.has_notice)
				list.notice = parse_entry(recv);

			list.total = recv.read_int();

			if (list.total > 0)
			{
				int32_t count = recv.read_int();

				for (int32_t i = 0; i < count; i++)
					list.entries.push_back(parse_entry(recv));
			}

			return list;
		}

		GuildBbs::Thread parse_thread(InPacket& recv)
		{
			GuildBbs::Thread thread;
			thread.id = recv.read_int();
			thread.poster = recv.read_int();
			thread.time = recv.read_long();
			thread.title = recv.read_string();
			thread.body = recv.read_string();
			thread.icon = recv.read_int();

			int32_t count = recv.read_int();

			for (int32_t i = 0; i < count; i++)
			{
				GuildBbs::Reply reply;
				reply.id = recv.read_int();
				reply.poster = recv.read_int();
				reply.time = recv.read_long();
				reply.text = recv.read_string();
				thread.replies.push_back(std::move(reply));
			}

			return thread;
		}
	}
}
