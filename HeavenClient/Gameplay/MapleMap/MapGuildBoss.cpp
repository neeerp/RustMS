#include "MapGuildBoss.h"

#include "../../Character/GuildBoss.h"
#include "../../Character/Snowball.h"

#include <nlnx/nx.hpp>

#include <cmath>

namespace ms
{
	MapGuildBoss::MapGuildBoss(nl::node src)
	{
		if (src["info"]["fieldType"].get_integer() != guild_boss::FIELD_TYPE || !guild_boss::on_field())
			return;

		active = true;

		const guild_boss::Rules& rules = guild_boss::rules();
		// The chandelier stays on its first frame (A 5587b3 never animates
		// it).
		healer = nl::nx::map.resolve(snowball::nx_path(rules.healer))["0"];
		pulley = nl::nx::map.resolve(snowball::nx_path(rules.pulley));
		pulley.stop();
	}

	MapGuildBoss::MapGuildBoss() {}

	bool MapGuildBoss::is_active() const
	{
		return active && guild_boss::on_field();
	}

	void MapGuildBoss::update()
	{
		if (is_active())
			pulley.update();
	}

	void MapGuildBoss::draw(double viewx, double viewy, float alpha) const
	{
		if (!is_active())
			return;

		const guild_boss::Rules& rules = guild_boss::rules();
		Point<int16_t> at(static_cast<int16_t>(std::round(rules.healer_x + viewx)),
						  static_cast<int16_t>(std::round(guild_boss::state().healer_y + viewy)));
		healer.draw(DrawArgument(at));

		Point<int16_t> pulled(static_cast<int16_t>(std::round(rules.pulley_x + viewx)),
							  static_cast<int16_t>(std::round(rules.pulley_y + viewy)));
		pulley.draw(DrawArgument(pulled), alpha);
	}

	void MapGuildBoss::swing()
	{
		if (is_active() && !pulley.is_playing())
			pulley.play(Animation::End::FIRST);
	}
}
