#include "FamilyParser.h"

namespace ms
{
	namespace FamilyParser
	{
		namespace
		{
			std::map<int32_t, int32_t> parse_counts(InPacket& recv)
			{
				std::map<int32_t, int32_t> counts;
				int32_t count = recv.read_int();

				for (int32_t i = 0; i < count; i++)
				{
					int32_t key = recv.read_int();
					counts[key] = recv.read_int();
				}

				return counts;
			}
		}

		FamilyInfo parse_info(InPacket& recv)
		{
			FamilyInfo info;
			info.rep = recv.read_int();
			info.total_rep = recv.read_int();
			info.today = recv.read_int();
			info.juniors = recv.read_short();
			info.max_juniors = recv.read_short();
			recv.skip_short();
			info.leader_id = recv.read_int();
			info.name = recv.read_string();
			info.precepts = recv.read_string();
			info.used = parse_counts(recv);

			return info;
		}

		std::vector<FamilyPrivilege> parse_privileges(InPacket& recv)
		{
			std::vector<FamilyPrivilege> privileges;
			int32_t count = recv.read_int();

			for (int32_t i = 0; i < count; i++)
			{
				FamilyPrivilege privilege;
				privilege.type = recv.read_byte();
				privilege.cost = recv.read_int();
				privilege.limit = recv.read_int();
				privilege.name = recv.read_string();
				privilege.description = recv.read_string();
				privileges.push_back(privilege);
			}

			return privileges;
		}

		FamilyChart parse_chart(InPacket& recv, int32_t self)
		{
			FamilyChart chart;
			chart.viewed = recv.read_int();
			chart.mine = chart.viewed == self;

			int32_t count = recv.read_int();

			for (int32_t i = 0; i < count; i++)
			{
				FamilyNode node;
				node.id = recv.read_int();
				node.senior_id = recv.read_int();
				node.job = recv.read_short();
				node.level = recv.read_byte();
				node.online = recv.read_bool();
				node.rep = recv.read_int();
				node.total_rep = recv.read_int();
				node.senior_rep = recv.read_int();
				node.today = recv.read_int();
				node.map_id = recv.read_int();
				node.minutes = recv.read_int();
				node.name = recv.read_string();
				chart.nodes.push_back(node);
			}

			chart.counts = parse_counts(recv);
			chart.used = parse_counts(recv);
			chart.max_juniors = recv.read_short();

			return chart;
		}
	}
}
