#include "SnowballHandlers.h"

#include "../../Character/Snowball.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UIChatBar.h"
#include "../../Util/Misc.h"

#include <nlnx/nx.hpp>

namespace ms
{
	void enter_snowball_field(int32_t mapid, int32_t fieldtype)
	{
		// The Init (A 574906) reads the lanes from the map's snowBall node.
		nl::node src = NxHelper::Map::get_map_node_name(mapid)["snowBall"];
		snowball::Rules rules;
		rules.dx = static_cast<int32_t>(src["dx"].get_integer(3));
		rules.x0 = src["x0"];
		rules.x = src["x"];
		rules.xmin = src["xMin"];
		rules.xmax = src["xMax"];

		for (size_t lane = 0; lane < 2; lane++)
		{
			nl::node line = src[std::to_string(lane)];
			rules.lanes[lane] = { line["y"], line["snowBall"].get_string(), line["snowMan"].get_string() };
		}

		snowball::enter_field(fieldtype, rules);
	}

	void SnowballStateHandler::handle(InPacket& recv) const
	{
		if (!snowball::on_field())
			return;

		snowball::StatePacket packet = snowball::read_state(recv, snowball::state().state == -1);
		snowball::apply(packet, snowball::now());
	}

	void HitSnowballHandler::handle(InPacket& recv) const
	{
		snowball::HitPacket hit = snowball::read_hit(recv);

		if (snowball::on_field())
			snowball::state().hits.add(hit, snowball::now());
	}

	void SnowballMessageHandler::handle(InPacket& recv) const
	{
		uint8_t team = static_cast<uint8_t>(recv.read_byte());
		uint8_t line = static_cast<uint8_t>(recv.read_byte());

		if (auto text = snowball::message(team, line))
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline(*text, UIChatBar::LineType::YELLOW);
	}

	void LeftKnockBackHandler::handle(InPacket&) const
	{
		// Left and up (FUN_00958164(300, 1)).
		Physics::knockback(Stage::get().get_player().get_phobj(), -300.0, -300.0);
	}
}
