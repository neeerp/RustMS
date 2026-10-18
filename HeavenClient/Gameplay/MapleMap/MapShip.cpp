#include "MapShip.h"

#include "../../Constants.h"
#include "../../Util/Randomizer.h"

#include <nlnx/nx.hpp>

#include <cmath>

namespace ms
{
	MapShip::MapShip(nl::node src)
	{
		nl::node obj = src["shipObj"];

		if (src["info"]["fieldType"].get_integer() != 2 || !obj["shipObj"])
			return;

		int32_t kind = static_cast<int32_t>(obj["shipKind"].get_integer());

		if (kind != ship::DOCKING && kind != ship::BALROG)
			return;

		motion.emplace(kind, static_cast<int32_t>(obj["x"].get_integer()),
					   static_cast<int32_t>(obj["x0"].get_integer()), ship::signed_y(obj["y"].get_integer()),
					   obj["f"].get_bool(), static_cast<int32_t>(obj["tMove"].get_integer()));
		animation = nl::nx::map.resolve(ship::obj_path(obj["shipObj"].get_string()));

		if (kind == ship::DOCKING)
			whistle = nl::nx::sound["Object.img"]["Whistle"];
	}

	MapShip::MapShip() {}

	int32_t MapShip::get_kind() const
	{
		return motion ? motion->get_kind() : -1;
	}

	void MapShip::move(uint8_t target, uint8_t event)
	{
		if (motion)
			act(ship::on_move(target, event, motion->get_kind()));
	}

	void MapShip::show_state(uint8_t state, uint8_t flag)
	{
		if (motion)
			act(ship::on_state(state, flag, motion->get_kind()));
	}

	void MapShip::act(ship::Action action)
	{
		switch (action)
		{
			case ship::Action::ENTER:
				if (motion->enter())
					whistle.play();

				break;
			case ship::Action::LEAVE:
				if (motion->leave())
					whistle.play();

				break;
			case ship::Action::APPEAR:
				// v83 leaves the Balrog's theme playing after it goes.
				if (motion->appear())
					Music("Bgm04.img/ArabPirate").play();

				break;
			case ship::Action::DISAPPEAR:
				motion->disappear();
				break;
			case ship::Action::NONE:
				break;
		}
	}

	void MapShip::update()
	{
		if (!motion)
			return;

		Randomizer randomizer;
		motion->update(Constants::TIMESTEP, [&](int32_t below) { return randomizer.next_int(below); });

		if (motion->is_shown())
			animation.update();
	}

	void MapShip::draw_behind(double viewx, double viewy, float alpha) const
	{
		if (motion && motion->get_kind() == ship::DOCKING)
			draw(viewx, viewy, alpha);
	}

	void MapShip::draw_above(double viewx, double viewy, float alpha) const
	{
		if (motion && motion->get_kind() == ship::BALROG)
			draw(viewx, viewy, alpha);
	}

	void MapShip::draw(double viewx, double viewy, float alpha) const
	{
		if (!motion->is_shown())
			return;

		Point<int16_t> position(static_cast<int16_t>(std::round(motion->get_x() + viewx)),
								static_cast<int16_t>(std::round(motion->get_y() + viewy)));
		animation.draw(DrawArgument(position, motion->get_flip(), static_cast<float>(motion->get_opacity())), alpha);
	}
}
