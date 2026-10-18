#include "MapDoors.h"

#include "DoorRules.h"

#include "../../Constants.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// A door just cast stands 1.8 s later (FUN_007bd262).
		constexpr int16_t OPENING_TIME = 1800;
		// A removed door fades out over a second (FUN_007be064).
		constexpr int16_t FADE_TIME = 1000;
	}

	Animation MapDoors::opening_template;
	Animation MapDoors::standing_template;
	Animation MapDoors::frame_template;

	void MapDoors::init()
	{
		nl::node src = nl::nx::skill["231.img"]["skill"][std::to_string(door_rules::MYSTIC_DOOR)];

		opening_template = src["cDoor"];
		standing_template = src["mDoor"];
		frame_template = src["Frame"];
	}

	MapDoors::Spot MapDoors::place(const Physics &physics, Point<int16_t> position)
	{
		const FootholdTree &fht = physics.get_fht();

		if (uint16_t fhid = fht.get_fhid_below(position - Point<int16_t>(0, 5)))
		{
			const Foothold &fh = fht.get_fh(fhid);
			auto ground = static_cast<int16_t>(fh.ground_below(position.x()));

			return { { position.x(), ground }, static_cast<Layer::Id>(fh.layer()) };
		}

		return { position, Layer::ZERO };
	}

	void MapDoors::spawn(int32_t owner, Point<int16_t> position, bool just_cast)
	{
		Door door{ position, std::nullopt, opening_template, standing_template, frame_template };
		door.opened = !just_cast;

		doors[owner] = door;
	}

	void MapDoors::remove(int32_t owner, bool fade)
	{
		auto iter = doors.find(owner);

		if (iter == doors.end())
			return;

		if (fade)
			iter->second.fading = FADE_TIME;
		else
			doors.erase(iter);
	}

	void MapDoors::update(const Physics &physics)
	{
		if (town_standing.get_duration() == 0)
		{
			town_standing = standing_template;
			town_frame = frame_template;
		}

		town_standing.update();
		town_frame.update();

		for (auto iter = doors.begin(); iter != doors.end();)
		{
			Door &door = iter->second;

			if (!door.spot)
				door.spot = place(physics, door.position);

			if (door.fading >= 0)
			{
				door.fading -= Constants::TIMESTEP;

				if (door.fading < 0)
				{
					iter = doors.erase(iter);

					continue;
				}
			}

			if (door.opened)
			{
				door.standing.update();
				door.frame.update();
			} else
			{
				// cDoor plays once and holds its last frame until the door
				// stands.
				if (door.age + Constants::TIMESTEP < door.opening.get_duration())
					door.opening.update();

				door.age += Constants::TIMESTEP;

				if (door.age > OPENING_TIME)
					door.opened = true;
			}

			++iter;
		}
	}

	void MapDoors::draw(Layer::Id layer, const std::vector<Spot> &town, Point<int16_t> viewpos, float alpha) const
	{
		for (const Spot &spot : town)
			if (spot.layer == layer)
				draw_standing(town_standing, town_frame, spot.position + viewpos, 1.0f, alpha);

		for (const auto &iter : doors)
		{
			const Door &door = iter.second;

			if (!door.spot || door.spot->layer != layer)
				continue;

			Point<int16_t> position = door.spot->position + viewpos;
			float opacity = door.fading >= 0 ? static_cast<float>(door.fading) / FADE_TIME : 1.0f;

			if (door.opened)
				draw_standing(door.standing, door.frame, position, opacity, alpha);
			else
				door.opening.draw(DrawArgument(position, opacity), alpha);
		}
	}

	int32_t MapDoors::find(Point<int16_t> playerpos) const
	{
		for (const auto &iter : doors)
			if (iter.second.spot && door_rules::reaches(iter.second.spot->position, playerpos))
				return iter.first;

		return 0;
	}

	void MapDoors::clear()
	{
		doors.clear();
	}

	// The Frame sits one below mDoor (FUN_007bd262), so it draws first.
	void MapDoors::draw_standing(const Animation &standing, const Animation &frame, Point<int16_t> position,
								 float opacity, float alpha)
	{
		frame.draw(DrawArgument(position, opacity), alpha);
		standing.draw(DrawArgument(position, opacity), alpha);
	}
}
