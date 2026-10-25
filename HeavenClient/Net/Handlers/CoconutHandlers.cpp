#include "CoconutHandlers.h"

#include "../../Character/Coconut.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/Misc.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The equip slots a uniform covers: the cap, and the overall over
		// the top and the bottom.
		constexpr int16_t UNIFORM_SLOTS[] = { 1, 5, 6 };
	}

	void enter_coconut_field(int32_t mapid, int32_t fieldtype)
	{
		if (coconut::state().dressed)
			for (int16_t slot : UNIFORM_SLOTS)
				Stage::get().get_player().change_equip(slot);

		// The Init (FUN_00548b77) counts the nuts and reads the uniforms
		// of each team and gender.
		nl::node src = NxHelper::Map::get_map_node_name(mapid)["coconut"];
		int32_t count = src["countFalling"].get_integer() + src["countBombing"].get_integer() +
						src["countStopped"].get_integer();
		std::array<coconut::Uniform, 4> uniforms;

		for (size_t team = 0; team < 2; team++)
		{
			for (size_t gender = 0; gender < 2; gender++)
			{
				nl::node avatar = src["avatar"][std::to_string(team)][std::to_string(gender)];
				uniforms[team * 2 + gender] = { avatar["cap"], avatar["clothes"] };
			}
		}

		coconut::enter_field(fieldtype, count > 0 ? static_cast<int16_t>(count) : coconut::DEFAULT_COUNT,
							 uniforms);
	}

	void CoconutHitHandler::handle(InPacket& recv) const
	{
		coconut::HitPacket hit = coconut::read_hit(recv);

		if (coconut::on_field())
			coconut::state().hits.add(hit, coconut::now(), coconut::count());
	}

	void CoconutScoreHandler::handle(InPacket& recv) const
	{
		coconut::state().score = coconut::read_score(recv);
	}

	void FieldSpecificDataHandler::handle(InPacket& recv) const
	{
		uint8_t team = static_cast<uint8_t>(recv.read_byte());
		Player& player = Stage::get().get_player();

		if (auto uniform = coconut::uniform(team, player.get_stats().get_female()))
		{
			player.get_look().add_equip(uniform->cap);
			player.get_look().add_equip(uniform->clothes);
			coconut::state().dressed = true;
		}
	}
}
