#include "GuildBossHandlers.h"

#include "../../Character/GuildBoss.h"
#include "../../Util/Misc.h"

#include <nlnx/nx.hpp>

namespace ms
{
	void enter_guild_boss_field(int32_t mapid, int32_t fieldtype)
	{
		// The Init (A 5587b3) reads the healer's x, its top and picture,
		// and the pulley's place and picture.
		nl::node src = NxHelper::Map::get_map_node_name(mapid);
		nl::node healer = src["healer"];
		nl::node pulley = src["pulley"];
		guild_boss::Rules rules;
		rules.healer_x = healer["x"];
		rules.y_min = healer["yMin"];
		rules.healer = healer["healer"].get_string();
		rules.pulley_x = pulley["x"];
		rules.pulley_y = pulley["y"];
		rules.pulley = pulley["pulley"].get_string();

		guild_boss::enter_field(fieldtype, rules);
	}

	void GuildBossHealerMoveHandler::handle(InPacket& recv) const
	{
		int16_t y = guild_boss::read_healer_move(recv);

		if (guild_boss::on_field())
			guild_boss::state().healer_y = y;
	}

	void GuildBossPulleyStateHandler::handle(InPacket& recv) const
	{
		uint8_t locked = guild_boss::read_pulley_state(recv);

		if (guild_boss::on_field())
			guild_boss::state().pulley = locked;
	}
}
