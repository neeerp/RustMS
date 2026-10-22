#pragma once

#include "../../InPacket.h"

#include "../../../Character/Family.h"

namespace ms
{
	namespace FamilyParser
	{
		// FAMILY_INFO_RESULT (0xa344f8).
		FamilyInfo parse_info(InPacket& recv);

		// FAMILY_PRIVILEGE_LIST.
		std::vector<FamilyPrivilege> parse_privileges(InPacket& recv);

		// FAMILY_CHART_RESULT (0x80a5f7), not yet arranged into boxes.
		FamilyChart parse_chart(InPacket& recv, int32_t self);
	}
}
